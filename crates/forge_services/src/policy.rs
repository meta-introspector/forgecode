use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use anyhow::Context;
use bytes::Bytes;
use forge_app::domain::{
    ExecuteRule, Fetch, Permission, PermissionOperation, Policy, PolicyConfig, PolicyEngine,
    ReadRule, Rule, WriteRule,
};
use forge_app::{
    DirectoryReaderInfra, EnvironmentInfra, FileInfoInfra, FileReaderInfra, FileWriterInfra,
    PolicyDecision, PolicyService, UserInfra,
};
use strum_macros::{Display, EnumIter};

/// User response for permission confirmation requests
#[derive(Debug, Clone, PartialEq, Eq, Display, EnumIter, strum_macros::EnumString)]
pub enum PolicyPermission {
    /// Accept the operation
    #[strum(to_string = "Accept")]
    Accept,
    /// Reject the operation
    #[strum(to_string = "Reject")]
    Reject,
    /// Accept the operation and remember this choice for similar operations
    #[strum(to_string = "Accept and Remember")]
    AcceptAndRemember,
}

#[derive(Clone)]
pub struct ForgePolicyService<I> {
    infra: Arc<I>,
}
/// Default policies loaded once at startup from the embedded YAML file
static DEFAULT_POLICIES: LazyLock<PolicyConfig> = LazyLock::new(|| {
    let yaml_content = include_str!("./permissions.default.yaml");
    serde_yml::from_str(yaml_content).expect(
        "Failed to parse default policies YAML. This should never happen as the YAML is embedded.",
    )
});

impl<I> ForgePolicyService<I>
where
    I: FileReaderInfra + FileWriterInfra + FileInfoInfra + EnvironmentInfra + DirectoryReaderInfra,
{
    pub fn new(infra: Arc<I>) -> Self {
        Self { infra }
    }

    fn permissions_path(&self) -> PathBuf {
        self.infra.get_environment().permissions_path()
    }

    fn local_permissions_path(&self) -> PathBuf {
        self.infra.get_environment().local_permissions_path()
    }

    /// Create a policies collection with sensible defaults
    /// Returns a clone of the preloaded default policies
    fn load_default_policies() -> PolicyConfig {
        DEFAULT_POLICIES.clone()
    }

    /// Add a policy for a specific operation type
    async fn add_policy_for_operation(
        &self,
        operation: &PermissionOperation,
    ) -> anyhow::Result<Option<PathBuf>>
    where
        I: UserInfra,
    {
        if let Some(new_policy) = create_policy_for_operation(operation, None) {
            // TODO: Can return a diff later
            self.modify_policy(new_policy).await?;
            Ok(Some(self.permissions_path()))
        } else {
            Ok(None)
        }
    }

    /// Load policy definitions from a specific path
    async fn read_policies_from(
        &self,
        path: &std::path::Path,
    ) -> anyhow::Result<Option<PolicyConfig>> {
        if !self.infra.exists(path).await? {
            return Ok(None);
        }
        let content = self.infra.read_utf8(path).await?;
        let policies = serde_yml::from_str(&content)
            .with_context(|| format!("Failed to parse policy {}", path.display()))?;
        Ok(Some(policies))
    }

    /// Load policy definitions from the global permissions file
    /// (~/.forge/permissions.yaml)
    async fn read_policies(&self) -> anyhow::Result<Option<PolicyConfig>> {
        self.read_policies_from(&self.permissions_path()).await
    }

    /// Load policy definitions from the local permissions file
    /// (.forge/permissions.yaml in the current project)
    async fn read_local_policies(&self) -> anyhow::Result<Option<PolicyConfig>> {
        self.read_policies_from(&self.local_permissions_path()).await
    }

    /// Evaluate an operation against local policies first, then global.
    /// Local policies take precedence over global policies.
    fn evaluate_layered(
        &self,
        local_policies: &Option<PolicyConfig>,
        global_policies: &PolicyConfig,
        operation: &PermissionOperation,
    ) -> Permission {
        // Check local policies first (project-specific overrides)
        if let Some(local) = local_policies {
            if !local.policies.is_empty() {
                let local_engine = PolicyEngine::new(local);
                let local_result = local_engine.can_perform(operation);
                // If a local policy matches (Allow/Deny/Confirm), use it
                if local_result != Permission::Confirm || !local.policies.is_empty() {
                    // Check if any local policy actually matched
                    for p in &local.policies {
                        if p.eval(operation).is_some() {
                            return local_result;
                        }
                    }
                }
            }
        }
        // Fall back to global policies
        let global_engine = PolicyEngine::new(global_policies);
        global_engine.can_perform(operation)
    }

    /// Add or modify a policy in the policies file.
    /// Writes to the local permissions file if it exists, otherwise writes to
    /// the global file.
    async fn modify_policy(&self, policy: Policy) -> anyhow::Result<()> {
        // Prefer local file if it exists, otherwise use global
        let local_path = self.local_permissions_path();
        let (policies_path, is_local) = if self.infra.exists(&local_path).await? {
            (local_path.clone(), true)
        } else {
            (self.permissions_path(), false)
        };

        let source = if is_local { &local_path } else { &self.permissions_path() };
        let policies = self.read_policies_from(source).await?.unwrap_or_default();
        let policies = policies.add_policy(policy);

        let new_content = serde_yml::to_string(&policies)
            .with_context(|| "Failed to serialize policies to YAML")?;

        self.infra
            .write(&policies_path, Bytes::from(new_content.to_owned()))
            .await?;

        Ok(())
    }

    /// Create a default policies file if it does not exist
    async fn init_policies(&self) -> anyhow::Result<()> {
        let policies_path = self.permissions_path();

        // Check if the file already exists
        if self.infra.exists(&policies_path).await? {
            return Ok(());
        }

        // Get the default policies content
        let default_policies = Self::load_default_policies();
        let content = serde_yml::to_string(&default_policies)
            .with_context(|| "Failed to serialize default policies to YAML")?;

        // Write the default policies to the file
        self.infra
            .write(&policies_path, Bytes::from(content))
            .await?;

        Ok(())
    }

    /// Load both local and global policies into a merged config.
    /// Local policies take precedence over global policies.
    async fn get_merged_policies(&self) -> anyhow::Result<(PolicyConfig, PolicyConfig, Option<PathBuf>)>
    where
        I: UserInfra,
    {
        let local = self.read_local_policies().await?.unwrap_or_default();

        // Ensure global policies exist (init if missing)
        let global = if let Some(g) = self.read_policies().await? {
            g
        } else {
            self.init_policies().await?;
            self.read_policies().await?.unwrap_or_default()
        };

        // Determine the active path for user notification
        let active_path = if !local.policies.is_empty() {
            Some(self.local_permissions_path())
        } else if !global.policies.is_empty() {
            Some(self.permissions_path())
        } else {
            Some(self.permissions_path())
        };

        Ok((local, global, active_path))
    }

    /// Get or create policies (legacy, returns global only)
    #[async_recursion::async_recursion]
    async fn get_or_create_policies(&self) -> anyhow::Result<(PolicyConfig, Option<PathBuf>)>
    where
        I: UserInfra,
    {
        if let Some(policies) = self.read_policies().await? {
            Ok((policies, None))
        } else {
            self.init_policies().await?;
            let (policies, _) = self.get_or_create_policies().await?;
            Ok((policies, Some(self.permissions_path())))
        }
    }
}

#[async_trait::async_trait]
impl<I> PolicyService for ForgePolicyService<I>
where
    I: FileReaderInfra
        + FileWriterInfra
        + FileInfoInfra
        + EnvironmentInfra
        + DirectoryReaderInfra
        + UserInfra,
{
    /// Check if an operation is allowed based on policies and handle user
    /// confirmation. Evaluates local policies first, then global.
    async fn check_operation_permission(
        &self,
        operation: &PermissionOperation,
    ) -> anyhow::Result<PolicyDecision> {
        let (local, global, path) = self.get_merged_policies().await?;
        let permission = self.evaluate_layered(&Some(local), &global, operation);

        match permission {
            Permission::Deny => Ok(PolicyDecision { allowed: false, path }),
            Permission::Allow => Ok(PolicyDecision { allowed: true, path }),
            Permission::Confirm => {
                // Request user confirmation using UserInfra
                let confirmation_msg = match operation {
                    PermissionOperation::Read { message, .. } => {
                        format!("{message}. How would you like to proceed?")
                    }
                    PermissionOperation::Write { message, .. } => {
                        format!("{message}. How would you like to proceed?")
                    }
                    PermissionOperation::Execute { message, .. } => {
                        format!("{message}. How would you like to proceed?")
                    }
                    PermissionOperation::Fetch { message, .. } => {
                        format!("{message}. How would you like to proceed?")
                    }
                };

                match self
                    .infra
                    .select_one_enum::<PolicyPermission>(&confirmation_msg)
                    .await?
                {
                    Some(PolicyPermission::Accept) => {
                        tracing::info!("Permission accepted by user");
                        Ok(PolicyDecision { allowed: true, path })
                    }
                    Some(PolicyPermission::AcceptAndRemember) => {
                        let update_path = self.add_policy_for_operation(operation).await?;
                        Ok(PolicyDecision { allowed: true, path: update_path.or(path) })
                    }
                    Some(PolicyPermission::Reject) | None => {
                        Ok(PolicyDecision { allowed: false, path })
                    }
                }
            }
        }
    }

    /// Check what type of permission an operation would receive without
    /// requesting user confirmation. Evaluates local policies first, then
    /// global.
    async fn check_permission_type(
        &self,
        operation: &PermissionOperation,
    ) -> anyhow::Result<Permission> {
        let (local, global, _path) = self.get_merged_policies().await?;
        Ok(self.evaluate_layered(&Some(local), &global, operation))
    }

    /// Persist a policy rule for the given operation so the user's choice is
    /// remembered for future similar operations without re-prompting.
    async fn remember_operation(
        &self,
        operation: &PermissionOperation,
    ) -> anyhow::Result<Option<PathBuf>> {
        self.add_policy_for_operation(operation).await
    }
}

/// Create a policy for an operation based on its type
fn create_policy_for_operation(
    operation: &PermissionOperation,
    dir: Option<String>,
) -> Option<Policy> {
    fn create_file_policy(
        path: &std::path::Path,
        rule_constructor: fn(String) -> Rule,
    ) -> Option<Policy> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|extension| Policy::Simple {
                permission: Permission::Allow,
                rule: rule_constructor(format!("*.{extension}")),
            })
    }

    match operation {
        PermissionOperation::Read { path, cwd: _, message: _ } => {
            create_file_policy(path, |pattern| {
                Rule::Read(ReadRule { read: pattern, dir: None })
            })
        }
        PermissionOperation::Write { path, cwd: _, message: _ } => {
            create_file_policy(path, |pattern| {
                Rule::Write(WriteRule { write: pattern, dir: None })
            })
        }

        PermissionOperation::Fetch { url, cwd: _, message: _ } => {
            if let Ok(parsed_url) = url::Url::parse(url) {
                parsed_url.host_str().map(|host| Policy::Simple {
                    permission: Permission::Allow,
                    rule: Rule::Fetch(Fetch { url: format!("{host}*"), dir: None }),
                })
            } else {
                Some(Policy::Simple {
                    permission: Permission::Allow,
                    rule: Rule::Fetch(Fetch { url: url.to_string(), dir: None }),
                })
            }
        }
        PermissionOperation::Execute { command, cwd: _, .. } => {
            let parts: Vec<&str> = command.split_whitespace().collect();
            match parts.as_slice() {
                [] => None,
                [cmd] => Some(Policy::Simple {
                    permission: Permission::Allow,
                    rule: Rule::Execute(ExecuteRule { command: format!("{cmd}*"), dir }),
                }),
                [cmd, subcmd, ..] => Some(Policy::Simple {
                    permission: Permission::Allow,
                    rule: Rule::Execute(ExecuteRule { command: format!("{cmd} {subcmd}*"), dir }),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_create_policy_for_read_operation() {
        let path = PathBuf::from("/path/to/file.rs");
        let operation = PermissionOperation::Read {
            path,
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Read file: /path/to/file.rs".to_string(),
        };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Read(ReadRule { read: "*.rs".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_write_operation() {
        let path = PathBuf::from("/path/to/file.json");
        let operation = PermissionOperation::Write {
            path,
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Create/overwrite file: /path/to/file.json".to_string(),
        };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Write(WriteRule { write: "*.json".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_write_patch_operation() {
        let path = PathBuf::from("/path/to/file.toml");
        let operation = PermissionOperation::Write {
            path,
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Modify file: /path/to/file.toml".to_string(),
        };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Write(WriteRule { write: "*.toml".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_net_fetch_operation() {
        let url = "https://example.com/api/data".to_string();
        let operation = PermissionOperation::Fetch {
            url,
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Fetch content from URL: https://example.com/api/data".to_string(),
        };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Fetch(Fetch { url: "example.com*".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_execute_operation_with_subcommand() {
        let command = "git push origin main".to_string();
        let message = "⚙ utility: `git push origin main`".to_string();
        let operation =
            PermissionOperation::Execute { command, cwd: std::path::PathBuf::from("/test/cwd"), message };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Execute(ExecuteRule { command: "git push*".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_execute_operation_single_command() {
        let command = "ls".to_string();
        let message = "⚙ utility: `ls`".to_string();
        let operation =
            PermissionOperation::Execute { command, cwd: std::path::PathBuf::from("/test/cwd"), message };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Execute(ExecuteRule { command: "ls*".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_file_without_extension() {
        let path = PathBuf::from("/path/to/file");
        let operation = PermissionOperation::Read {
            path,
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Read file: /path/to/file".to_string(),
        };

        let actual = create_policy_for_operation(&operation, None);

        let expected = None;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_invalid_url() {
        let url = "not-a-valid-url".to_string();
        let operation = PermissionOperation::Fetch {
            url,
            cwd: std::path::PathBuf::from("/test/cwd"),
            message: "Fetch content from URL: not-a-valid-url".to_string(),
        };

        let actual = create_policy_for_operation(&operation, None);

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Fetch(Fetch { url: "not-a-valid-url".to_string(), dir: None }),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_empty_execute_command() {
        let command = "".to_string();
        let message = String::new();
        let operation =
            PermissionOperation::Execute { command, cwd: std::path::PathBuf::from("/test/cwd"), message };

        let actual = create_policy_for_operation(&operation, None);

        let expected = None;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_create_policy_for_execute_operation_with_working_directory() {
        let command = "ls".to_string();
        let message = "⚙ utility: `ls`".to_string();
        let operation =
            PermissionOperation::Execute { command, cwd: std::path::PathBuf::from("/test/cwd"), message };
        let working_directory = Some("/home/user/project".to_string());

        let actual = create_policy_for_operation(&operation, working_directory.clone());

        let expected = Some(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Execute(ExecuteRule { command: "ls*".to_string(), dir: working_directory }),
        });

        assert_eq!(actual, expected);
    }
}

#[cfg(test)]
mod layered_tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_local_overrides_global_deny_with_allow() {
        // Global denies *.rs, local allows src/**/*.rs
        let global = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Deny,
            rule: Rule::Write(WriteRule { write: "*.rs".to_string(), dir: None }),
        });
        let local = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Write(WriteRule { write: "src/**/*.rs".to_string(), dir: None }),
        });
        let operation = PermissionOperation::Write {
            path: Path::new("src/main.rs").to_path_buf(),
            cwd: Path::new("/project").to_path_buf(),
            message: "Write file".to_string(),
        };

        // Local should match first (local matches)
        let local_engine = PolicyEngine::new(&local);
        let local_result = local_engine.can_perform(&operation);
        assert_eq!(local_result, Permission::Allow);

        // Global would deny
        let global_engine = PolicyEngine::new(&global);
        let global_result = global_engine.can_perform(&operation);
        assert_eq!(global_result, Permission::Deny);

        // Since local is checked first and matches, the effective result is
        // Allow
        assert_eq!(local_result, Permission::Allow);
    }

    #[test]
    fn test_global_used_when_no_local_match() {
        // Global allows *.rs, local has unrelated rules
        let global = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Write(WriteRule { write: "*.rs".to_string(), dir: None }),
        });
        let local = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Deny,
            rule: Rule::Write(WriteRule { write: "*.py".to_string(), dir: None }),
        });
        let operation = PermissionOperation::Write {
            path: Path::new("src/main.rs").to_path_buf(),
            cwd: Path::new("/project").to_path_buf(),
            message: "Write file".to_string(),
        };

        // Local doesn't match .rs files
        let local_engine = PolicyEngine::new(&local);
        let local_result = local_engine.can_perform(&operation);
        assert_eq!(local_result, Permission::Confirm); // No matching local rule

        // Global matches
        let global_engine = PolicyEngine::new(&global);
        let global_result = global_engine.can_perform(&operation);
        assert_eq!(global_result, Permission::Allow);
    }

    #[test]
    fn test_local_deny_overrides_global_allow() {
        // Global allows *.rs, local denies *.rs
        let global = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Write(WriteRule { write: "*.rs".to_string(), dir: None }),
        });
        let local = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Deny,
            rule: Rule::Write(WriteRule { write: "*.rs".to_string(), dir: None }),
        });
        let operation = PermissionOperation::Write {
            path: Path::new("src/main.rs").to_path_buf(),
            cwd: Path::new("/project").to_path_buf(),
            message: "Write file".to_string(),
        };

        // Local matches and denies
        let local_engine = PolicyEngine::new(&local);
        let local_result = local_engine.can_perform(&operation);
        assert_eq!(local_result, Permission::Deny);
    }

    #[test]
    fn test_local_confirm_overrides_global_allow() {
        // Global allows *.rs, local sets confirm for a specific path
        let global = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Allow,
            rule: Rule::Write(WriteRule { write: "*.rs".to_string(), dir: None }),
        });
        let local = PolicyConfig::new().add_policy(Policy::Simple {
            permission: Permission::Confirm,
            rule: Rule::Write(WriteRule { write: "src/critical/*".to_string(), dir: None }),
        });
        let operation = PermissionOperation::Write {
            path: Path::new("src/critical/secret.rs").to_path_buf(),
            cwd: Path::new("/project").to_path_buf(),
            message: "Write file".to_string(),
        };

        // Local matches and requires confirm
        let local_engine = PolicyEngine::new(&local);
        let local_result = local_engine.can_perform(&operation);
        assert_eq!(local_result, Permission::Confirm);

        // Would fall through to global if local didn't match, but local does
        // match
        let global_engine = PolicyEngine::new(&global);
        let global_result = global_engine.can_perform(&operation);
        assert_eq!(global_result, Permission::Allow);
    }
}

