# Standalone CLI Drivers Plan

## Goal
Create clean, reusable CLI driver modules that expose forge's core functionality for standalone usage while maintaining compatibility with the existing architecture.

## Phase 1: Core Driver Module (`drivers.rs`)
- Create `crates/forge_main/src/drivers.rs` with essential driver functions
- Expose `run_cli()` function that handles command execution
- Provide `execute_command()` for programmatic command execution
- Include error handling and logging utilities

## Phase 2: Command-Specific Drivers
- Create drivers for key command categories:
  - `commit_driver.rs`: Git commit message generation
  - `config_driver.rs`: Configuration management
  - `mcp_driver.rs`: MCP server management
  - `workspace_driver.rs`: Workspace indexing and querying

## Phase 3: Integration and Testing
- Add tests for each driver module
- Ensure backward compatibility with existing CLI
- Document usage patterns