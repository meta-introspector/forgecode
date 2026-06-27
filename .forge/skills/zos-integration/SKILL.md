# ZOS Integration Skill

## Overview
This skill documents the integration of the ZOS (Zero‑Overhead Services Orchestration) plugin system into the ForgeCode AI framework. It covers the complete workflow for loading, managing, and executing ZOS plugins within ForgeCode, including task planning, formal verification, and skill discovery.

## Core Components

### 1. ZOS Plugin Architecture

The ZOS system provides a plugin interface for extending ForgeCode functionality. Key aspects include:

- **Plugin Registration**: All plugins register in `~/.dotagents/config.toml` with metadata
- **Plugin Discovery**: Skills scan `~/.dotagents/skills/` and `~/.forge/skills/` for `SKILL.md` files
- **Plugin Loading**: Safe dynamic loading via `libloading` with version checking
- **Plugin Lifecycle**: Each plugin implements `initialize()` and `shutdown()` hooks

### 2. Plugin Development Workflow

#### Step 1: Plugin Structure
```rust
// libexample_plugin.rs
// ZOS plugin entry points
pub fn zos_plugin_name() -> *const c_char { ... }
pub fn zos_plugin_version() -> *const c_char { ... }
pub fn zos_plugin_init() -> bool { ... }
pub fn zos_plugin_shutdown() -> bool { ... }
```

#### Step 2: Cargo.toml Configuration
```toml
[package]
name = "example_plugin"
version = "0.1.0"
edition = "2021"

[lib]
name = "example_plugin"
crate-type = ["cdylib"]

[dependencies]
libloading = "0.8"
anyhow = "1.0"
```

#### Step 3: Plugin Registration
```toml
[[plugins]]
name = "example"
flake = "git+file:./plugins/example"
store_path = "/workspace/target/release"
shared_object = "/workspace/plugins/example.so"
```

### 3. Task Planning Integration

#### 3.1 GOAP Planner Plugin
The task manager plugin implements the GOAP (Goal‑Oriented Action Planning) algorithm:

- **Task Representation**: Each task has `id`, `description`, `status`, `dependencies`
- **Goal Planning**: Generate action sequences to achieve objectives
- **Skill Suggestions**: Use ZKPrologML for context‑aware skill recommendations

#### 3.2 Integration Example
```rust
// In task_manager_plugin.rs
let skills = suggest_skills("goap-planning");
let plan = goap_planner.plan("find new work", &skills);
```

### 4. Formal Verification Pipeline

#### 4.1 Lean4 Integration
The Lean4 prover plugin validates GOAP plans:

```lean
-- Verification theorem
theorem goap_correct : ∀ (task: Task), proved task → valid task :=
begin
  -- Formal proof of plan correctness
end
```

#### 4.2 Verification Workflow
1. Generate GOAP plan
2. Formalize plan in Lean4
3. Run proof verification
4. Validate plan properties

### 5. Skill Discovery and Loading

#### 5.1 Skill Loading Process
```rust
// skill_loader.rs
fn load_skills() -> Vec<Skill> {
    let mut skills = Vec::new();
    
    // Load built‑in skills
    skills.extend(load_builtin_skills());
    
    // Load global skills
    skills.extend(load_skills_from_path("~/.forge/skills/"));
    
    // Load agent skills
    skills.extend(load_skills_from_path("~/.agents/skills/"));
    
    // Load dotagents skills
    skills.extend(load_skills_from_path("~/.dotagents/skills/"));
    
    skills
}
```

#### 5.2 Skill Structure
```rust
struct Skill {
    name: String,
    description: String,
    plugin_name: String,
    version: String,
    metadata: HashMap<String, String>,
}
```

### 6. ZOS Plugin Bridge

#### 6.1 Bridge Implementation
The `forge_infra` crate provides the ZOS plugin bridge:

```rust
// zos_plugin_bridge.rs
pub struct ZosPluginBridge {
    plugins: Vec<Box<dyn Plugin>>,
}

impl ZosPluginBridge {
    pub fn load_all_plugins(&mut self) -> Result<(), BridgeError> {
        // Load and initialize all registered plugins
    }
    
    pub fn execute_plugin_action(&self, plugin_name: &str, action: &str) -> Result<String, BridgeError> {
        // Delegate actions to appropriate plugins
    }
}
```

### 7. System Integration Examples

#### 7.1 Complete Workflow
```bash
# 1. Load all plugins
 cargo run -p forge_infra -- zos_plugin_bridge --load-all-plugins

# 2. Get skill suggestions for a context
 cargo run -p forge_infra -- zos_plugin_bridge --suggest-skills "goap-planning"

# 3. Execute GOAP planning
 cargo run -p forge_infra -- zos_plugin_bridge --plan-goal "find new work"

# 4. Verify plan with Lean4
 cargo run -p forge_infra -- zos_plugin_bridge --verify-plan "plan.json"
```

#### 7.2 Error Handling
```rust
match plugin_result {
    Ok(result) => println!("Plugin executed successfully: {}", result),
    Err(e) => eprintln!("Plugin execution failed: {}", e),
}
```

### 8. Testing and Validation

#### 8.1 Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_loading() {
        let mut bridge = ZosPluginBridge::new();
        let result = bridge.load_all_plugins();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_skill_suggestion() {
        let skills = suggest_skills("task-planning");
        assert!(!skills.is_empty());
        assert!(skills.iter().any(|s| s.contains("task-manager")));
    }
}
```

#### 8.2 Integration Tests
```bash
# Run all ZOS plugin tests
cargo test -p forge_infra zos_plugin_bridge -- --nocapture

# Run specific plugin tests
cargo test -p forge_task_manager_plugin
cargo test -p zkprologml_plugin
```

### 9. Configuration Management

#### 9.1 Plugin Configuration
```toml
[plugins.example]
name = "example"
version = "0.1.0"
description = "Example ZOS plugin"

[plugins.example.settings]
max_memory = "100MB"
timeout = "30s"
```

#### 9.2 Environment Variables
```bash
export ZOS_PLUGIN_PATH="~/.dotagents/plugins"
export ZOS_CONFIG_PATH="~/.dotagents/config.toml"
export ZOS_LOG_LEVEL="info"
```

### 10. Best Practices

#### 10.1 Plugin Design
- Use `libloading` for safe dynamic loading
- Implement proper error handling with `anyhow`
- Follow the ZOS plugin lifecycle correctly
- Ensure thread safety for concurrent access

#### 10.2 Skill Management
- Use YAML frontmatter in `SKILL.md` files
- Implement skill caching for performance
- Validate skill dependencies before loading
- Provide clear skill metadata

#### 10.3 Formal Verification
- Write Lean4 theorems for critical properties
- Use Minizinc for optimization
- Integrate ZKPrologML for reasoning
- Maintain audit trails of all verifications

### 11. Troubleshooting

#### 11.1 Common Issues
- **Plugin not found**: Check `~/.dotagents/config.toml` and plugin paths
- **Plugin initialization failure**: Verify plugin exports ZOS symbols
- **Skill loading errors**: Check YAML syntax in `SKILL.md` files
- **Verification failures**: Review Lean4 proof obligations

#### 11.2 Debug Commands
```bash
# List loaded plugins
cargo run -p forge_infra -- zos_plugin_bridge --list-plugins

# Check plugin status
cargo run -p forge_infra -- zos_plugin_bridge --status

# View plugin logs
cargo run -p forge_infra -- zos_plugin_bridge --logs
```

### 12. Future Extensions

#### 12.1 New Plugin Types
- **Data Plugins**: Handle structured data exchange
- **API Plugins**: Provide REST/GraphQL interfaces
- **Security Plugins**: Implement authentication and authorization

#### 12.2 Enhanced Features
- **Plugin Composition**: Combine multiple plugins for complex workflows
- **Hot‑Reload**: Support for runtime plugin updates
- **Distributed Plugins**: Network‑aware plugin discovery

## References

- ZOS Documentation: `https://github.com/zerOverheadServices/zos`
- ForgeCode Skills System: `crates/forge_repo/src/skill.rs`
- DOTAgents Plugin Registry: `~/.dotagents/config.toml`
- Lean4 Prover Plugin: `crates/lean4_prover_plugin/`
- ZKPrologML Skill Suggestions: `crates/zkprologml_plugin/`
- GOAP Planner: `crates/forge_task_manager_plugin/`

## See Also

- [DOTAgents Plugin Development Guide](https://github.com/zerOverheadServices/dotagents)
- [ForgeCode Plugin Architecture](https://github.com/forgecode/forgecode/tree/main/crates/forge_infra)
- [ZOS Plugin Registry](~/.dotagents/config.toml)
- [Task Planning System](https://github.com/forgecode/forgecode/tree/main/crates/forge_task_manager_plugin)

---

*This skill was automatically generated as part of the ZOS integration effort in ForgeCode.*
