# tmux-mcp Skill

Control tmux sessions, windows, panes, buffers, and send keys through the
`tmux-mcp-rs` MCP server (stdio transport, registered in `.mcp.json` as
`"tmux"`).

## Source

| Attribute | Value |
|-----------|-------|
| **Repository** | [bnomei/tmux-mcp](https://github.com/bnomei/tmux-mcp) (Rust, 24★) |
| **Flake input** | `tmux-mcp` → `git+file:///home/mdupont/git/github.com/bnomei/tmux-mcp?ref=forge-integration` |
| **Package** | `nix build .#tmux-mcp-rs` / `tmux-mcp-rs-0.2.1` |
| **App** | `nix run .#tmux-mcp-rs` |
| **Transport** | stdio (spawned on-demand from `.mcp.json`) |
| **Dependencies** | `rmcp` (Rust MCP SDK), `tokio`, `clap`, `serde`, `thiserror`, `tracing` |

## MCP Configuration

The tmux-mcp server is registered in `.mcp.json`:

```json
{
  "mcpServers": {
    "tmux": {
      "command": "tmux-mcp-rs"
    }
  }
}
```

CLI options (passed via `args` array in `.mcp.json`):

| Flag | Default | Description |
|------|---------|-------------|
| `-t,--shell-type` | `bash` | Shell type: `bash`, `zsh`, or `fish` |
| `-c,--config` | — | Path to TOML config file for security policy |
| `-s,--socket` | — | Path to tmux socket (for isolation) |
| `-r,--ssh` | — | SSH connection string for remote tmux |

## Security Policy

By default all operations are allowed. Create a TOML config file to restrict:

```toml
# tmux-mcp-config.toml
[shell]
shell_type = "bash"

[ssh]
remote = "user@host"

[tracking]
enabled = true
max_commands = 50

[search]
mode = "fuzzy"  # or "exact"
max_results = 100
```

Launch with: `tmux-mcp-rs -c /path/to/config.toml`

## Available Tools

### Sessions

| Tool | Description |
|------|-------------|
| `list_sessions` | List all tmux sessions |
| `find_session` | Find sessions matching a name pattern |
| `create_session` | Create a new tmux session |
| `kill_session` | Kill a tmux session |
| `get_current_session` | Get the current tmux session name |
| `rename_session` | Rename a tmux session |

### Windows

| Tool | Description |
|------|-------------|
| `list_windows` | List windows in a session |
| `create_window` | Create a new window |
| `kill_window` | Kill a window |
| `rename_window` | Rename a window |
| `move_window` | Move a window to a different index/session |
| `select_window` | Select (switch to) a window |

### Panes

| Tool | Description |
|------|-------------|
| `list_panes` | List panes in a window |
| `split_pane` | Split a pane (horizontal/vertical) |
| `kill_pane` | Kill a pane |
| `rename_pane` | Rename/retitle a pane |
| `select_pane` | Select (focus) a pane |
| `resize_pane` | Resize a pane (up/down/left/right) |
| `zoom_pane` | Toggle pane zoom |
| `select_layout` | Select a pane layout (`even-horizontal`, `tiled`, etc.) |
| `join_pane` | Join a pane from another window |
| `break_pane` | Break a pane into its own window |
| `swap_pane` | Swap two panes |
| `set_synchronize_panes` | Toggle synchronized input across panes |

### Commands

| Tool | Description |
|------|-------------|
| `execute_command` | Execute a shell command in a pane and get command ID |
| `get_command_result` | Get the result of a previously executed command |

### Input

| Tool | Description |
|------|-------------|
| `send_keys` | Send keystrokes to a pane |
| `send_cancel` | Send Ctrl+C (SIGINT) |
| `send_eof` | Send Ctrl+D (EOF) |
| `send_enter` | Send Enter key |
| `send_tab` | Send Tab key |
| `send_backspace` | Send Backspace key |
| `send_up/down/left/right` | Send arrow keys |
| `send_page_up/page_down` | Send PageUp/PageDown |
| `send_home/end` | Send Home/End |
| `send_escape` | Send Escape |
| `send_special_key` | Send any special key by name |

### Clients

| Tool | Description |
|------|-------------|
| `list_clients` | List tmux client connections |
| `detach_client` | Detach a client |

### Buffers

| Tool | Description |
|------|-------------|
| `list_buffers` | List paste buffers |
| `capture_pane` | Capture pane content (like `tmux capture-pane`) |
| `show_buffer` | Show a buffer's content |
| `save_buffer` | Save buffer to file |
| `load_buffer` | Load a file into a buffer |
| `delete_buffer` | Delete a buffer |
| `set_buffer` | Set buffer contents |
| `append_buffer` | Append text to a buffer |
| `rename_buffer` | Rename a buffer |
| `search_buffer` | Search across buffers |
| `subsearch_buffer` | Search within a buffer's contents |

## Usage Examples

```bash
# List all tmux sessions
# (MCP client sends tools/call for "list_sessions")

# Create a session and run a command
# 1. create_session → session_id
# 2. execute_command → command_id
# 3. get_command_result → output

# Capture pane 0 in window0 of session my-session
# 1. capture_pane(pane_id="my-session:0.0")

# Send keys to a pane
# 1. send_keys(pane_id="my-session:0.0", keys="echo hello\n")
```

## Integration Notes

- **stdio transport** — spawned on-demand by Forge when a tool is called
- **No systemd service** — stdio servers run as child processes of Forge
- **Security policy** — create a TOML config and pass via `args: ["-c", "/path/to/config.toml"]` in `.mcp.json`
- **Remote tmux** — use `--ssh user@host` or set `TMUX_MCP_SSH` env var
