use chrono::{DateTime, Local, Utc};
use forge_domain::Conversation;
use std::path::Path;

/// Computes the current git tree hash (HEAD^{tree}) for the given workspace
/// root. Returns `None` if git is unavailable, the directory is not a git
/// repo, or the command fails.
pub fn compute_git_tree_hash(workspace_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(workspace_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// Initializes conversation metrics with start time and git tree hash
#[derive(Debug, Clone)]
pub struct InitConversationMetrics {
    current_time: DateTime<Local>,
    git_tree_hash: Option<String>,
}

impl InitConversationMetrics {
    pub fn new(current_time: DateTime<Local>, git_tree_hash: Option<String>) -> Self {
        Self { current_time, git_tree_hash }
    }

    pub fn apply(self, mut conversation: Conversation) -> Conversation {
        conversation.metrics.started_at = Some(self.current_time.with_timezone(&Utc));
        conversation.metrics.git_tree_hash = self.git_tree_hash;
        conversation
    }
}

#[cfg(test)]
mod tests {
    use forge_domain::ConversationId;

    use super::*;

    #[test]
    fn test_sets_started_at() {
        let current_time = Local::now();
        let conversation = Conversation::new(ConversationId::generate());

        let actual = InitConversationMetrics::new(current_time, None).apply(conversation);

        assert!(actual.metrics.started_at.is_some());
        let expected_time = current_time.with_timezone(&Utc);
        let actual_time = actual.metrics.started_at.unwrap();

        // Compare timestamps with some tolerance (1 second)
        let diff = (actual_time - expected_time).num_seconds().abs();
        assert!(diff < 1, "Timestamps should be within 1 second");
    }

    #[test]
    fn test_sets_git_tree_hash() {
        use pretty_assertions::assert_eq;
        let current_time = Local::now();
        let conversation = Conversation::new(ConversationId::generate());

        let actual =
            InitConversationMetrics::new(current_time, Some("abc123".to_string()))
                .apply(conversation);

        assert_eq!(actual.metrics.git_tree_hash, Some("abc123".to_string()));
    }
}
