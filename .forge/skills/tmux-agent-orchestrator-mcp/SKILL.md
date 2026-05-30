# tmux-agent-orchestrator-mcp Skill

Orchestrate multiple AI agents — each running in an isolated tmux session.
Spawn agents, capture their output, send them keystrokes, kill them, and
transfer text files in/out.

## Architecture

```
┌─────────────────────────┐
│   tmux-agent-orchestrator-mcp  │  ← stdio MCP server
│                         │
│  Shared State (Arc<RwLock<HashMap>)  │
│  maps agent_label → { session, pid } │
│                         │
│  Tools:                 │
│  ├ spawn_agent          │  → creates tmux session + runs command
│  ├ capture_agent_output │  → reads pane contents (-S -)
│  ├ send_keys_to_agent   │  → send-keys into session
│  ├ list_agents          │  → all tracked agents + alive status
│  ├ kill_agent           │  → tmux kill-session + remove metadata
│  ├ agent_send_file      │  → paste text into session via send-keys
│  └ agent_read_file      │  → capture pane output and save to file
└─────────────────────────┘
```

## MCP Configuration

Registered in `.mcp.json` as `"tmux-agent-orchestrator"`:

```json
{
  "mcpServers": {
    "tmux-agent-orchestrator": {
      "command": "forge-tmux-agent-orchestrator-mcp"
    }
  }
}
```

## Tools

### `spawn_agent`

Create a new isolated tmux session running an arbitrary command.

**Arguments:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `label` | string | yes | — | Unique agent label (used as tmux session name) |
| `command` | string | yes | — | Shell command to run in the session |
| `cwd` | string | no | agent's CWD | Working directory for the command |
| `timeout_secs` | integer | no | 300 | Max run time before auto-kill |
| `env` | array | no | `[]` | Additional `KEY=VALUE` env pairs |

**Returns:** JSON with `label`, `session`, `pid`, `timeout_secs`, `alive`.

**Example:**
```
label=crawler-1  command="cargo run --bin deep-scanner -- --root /repo"
```

### `capture_agent_output`

Read the current contents of an agent's tmux pane.

**Arguments:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `session` | string | yes | — | Agent session/label to capture |
| `lines` | integer | no | 50 | Max lines to capture (0 = all) |

**Returns:** JSON with `session`, `lines`, `output` (raw text).

### `send_keys_to_agent`

Send keystrokes into an agent's tmux session.

**Arguments:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `session` | string | yes | — | Agent session label |
| `keys` | string | yes | — | Keystrokes to send (literal text + Enter) |

### `list_agents`

List all tracked agent sessions.

**Returns:** JSON array with `label`, `alive`, `pid`, `created_at` for each agent.

### `kill_agent`

Kill an agent's tmux session and remove its metadata.

**Arguments:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `session` | string | yes | Agent session to kill |

### `agent_send_file`

Read a local text file and paste its contents into an agent's tmux session
(via `send-keys`).

**Arguments:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `session` | string | yes | Agent session label |
| `path` | string | yes | Local file path to read and send |

### `agent_read_file`

Capture an agent's pane output and write it to a local file.

**Arguments:**

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `session` | string | yes | — | Agent session to capture |
| `path` | string | yes | — | Output file path to write |
| `lines` | integer | no | 50 | Max lines to capture |

## Source

| Attribute | Value |
|-----------|-------|
| **Repository** | forgecode `crates/forge_tmux_agent_orchestrator_mcp/` |
| **Package** | `nix build .#forge-tmux-agent-orchestrator-mcp` |
| **App** | `nix run .#forge-tmux-agent-orchestrator-mcp` |
| **Transport** | stdio (spawned on-demand from `.mcp.json`) |
| **Dependencies** | `rmcp`, `tokio`, `serde_json`, `chrono`, `regex` |

## Usage

```bash
# Spawn a long-running scan
nix run .#forge-tmux-agent-orchestrator-mcp <<< '{"method":"spawn_agent","label":"scan-1","command":"nix flake check --no-build"}'

# Check output
nix run .#forge-tmux-agent-orchestrator-mcp <<< '{"method":"capture_agent_output","session":"scan-1","lines":20}'

# Send interrupt
nix run .#forge-tmux-agent-orchestrator-mcp <<< '{"method":"send_keys_to_agent","session":"scan-1","keys":"^C"}'

# Clean up
nix run .#forge-tmux-agent-orchestrator-mcp <<< '{"method":"kill_agent","session":"scan-1"}'
```
