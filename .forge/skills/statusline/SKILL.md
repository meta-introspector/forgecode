---
name: statusline
description: >-
  Create and edit the agent statusline extension. Shows persistent
  state at the bottom of the terminal. Use when handling the
  /statusline command or customizing the bottom bar.
---

# Statusline — Bottom Bar Customization

Create or update the agent statusline extension. The statusline shows
persistent state at the bottom of the terminal — branch, model, cost,
task status, etc.

---

## When to Use

- "Customize the statusline"
- "Add a status value to the bottom bar"
- "Change what's shown in the statusline"
- User runs `/statusline`

## Architecture

The statusline is a special extension that renders a single line at
the bottom of the terminal. It uses `letta.capabilities.ui.customStatuslineRenderer`
and `letta.capabilities.ui.statusValues`.

## Basic Statusline Extension

```ts
export default function activate(letta) {
  const disposers = [];

  if (letta.capabilities.ui.customStatuslineRenderer) {
    disposers.push(letta.statusline.register({
      render: (ctx) => {
        return {
          left: [
            { text: `⎇ ${ctx.git?.branch || 'detached'}` },
            { text: `◈ ${ctx.model}` },
          ],
          right: [
            { text: `$${ctx.costUsd?.toFixed(2) || '0.00'}` },
          ]
        };
      }
    }));
  }

  return () => disposers.forEach(d => d());
}
```

## Status Values

For small persistent state that other extensions can read:

```ts
if (letta.capabilities.ui.statusValues) {
  disposers.push(letta.statusValues.register({
    key: "task-status",
    description: "Current task status",
    getValue: () => currentTaskStatus
  }));
}
```

## Render Context

The render function receives a context object with:

| Field | Type | Description |
|-------|------|-------------|
| `ctx.git?.branch` | string | Current git branch |
| `ctx.model` | string | Current model name |
| `ctx.costUsd` | number | Session cost in USD |
| `ctx.cwd` | string | Current working directory |
| `ctx.conversation` | object | Current conversation metadata |

## Layout

The statusline has two segments:

- **Left**: Branch, model, task name
- **Right**: Cost, token count, time

Keep it minimal — the statusline is one line. Don't overload it.

## Guardrails

- Keep the statusline minimal — one line, essential info only
- Use capability guards: `letta.capabilities.ui.customStatuslineRenderer`
- Return a disposer for cleanup
- Don't make network calls in the render function — it runs on every frame
- Don't show sensitive data (API keys, tokens) in the statusline

## Related

- `extensions` — full extension API
- `commands` — for slash commands
- `harness-config` — for model and context settings

## Shmem Cross-References

> Generated: 2026-06-23 10:20:04 | REPL: http://localhost:8156 | Declarations loaded: 366

| Keyword | Shmem Matches | Type |
|---------|--------------|------|
| — | No matches in shmem for 14 keywords | — |
| — | Searchable terms: Architecture, Bar, Basic, Bottom, Customization, Extension, Guardrails, Layout, Related, Render | — |