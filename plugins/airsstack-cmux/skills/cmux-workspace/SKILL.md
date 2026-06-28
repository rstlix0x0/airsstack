---
name: cmux-workspace
description: Use when automating cmux workspaces and workspace groups — creating or selecting workspaces, building multi-pane layouts, or organizing workspaces into collapsible sidebar groups (workspace-group create/add/remove/pin/collapse). Scopes automation to the caller workspace. Builds on cmux-control for the mental model and targeting.
---

# cmux Workspace

Workspace automation for cmux: create and switch workspaces, build multi-pane layouts, and
organize workspaces into collapsible sidebar groups. For the object hierarchy (window → workspace →
pane → surface) and targeting rules, see the `## Mental model` and `## Targeting` sections of
**cmux-control** — those sections are authoritative and are not restated here.

Grounded on `cmux 0.64.17`. Run `cmux --help` or `cmux capabilities` to self-correct if a
command is absent or its flags have changed.

## Scope

By default, commands act on the caller workspace — the workspace identified by `CMUX_WORKSPACE_ID`,
auto-set in every cmux-managed terminal session. Pass `--workspace <id|ref|index>`
explicitly only when you need to act on a different workspace.

## Workspaces

Core workspace lifecycle commands:

```sh
cmux new-workspace --name <title> [--cwd <path>] [--command <cmd>] [--no-focus]
cmux select-workspace --workspace <id|ref|index>
cmux current-workspace
cmux rename-workspace [--workspace <id|ref>] <title>
cmux close-workspace --workspace <id|ref|index>
```

Useful `new-workspace` flags:

- `--name <title>` — workspace sidebar label.
- `--cwd <path>` — working directory for the initial pane.
- `--command <cmd>` — shell command to run in the initial pane.
- `--group <id|ref>` and `--group-placement afterCurrent|top|end` — place the new workspace
  directly into an existing group (see **Groups** below and `references/workspace-groups.md`).
- `--no-focus` — create without stealing focus (recommended when scripting).

## Layouts

Use `${CLAUDE_PLUGIN_ROOT}/skills/cmux-workspace/scripts/cmux-layout` to build a named workspace
with splits and optional per-pane startup commands in one call:

```sh
# Example: 2-pane layout, dev server on the right.
${CLAUDE_PLUGIN_ROOT}/skills/cmux-workspace/scripts/cmux-layout \
  --name dev \
  --split right \
  --cmd "npm run dev"
```

The helper is pure geometry plus optional per-pane commands — `--cmd` runs a shell command in
that pane. It does **not** spawn agent sessions or coordinate agents. See
`references/layouts.md` for more recipes and the equivalent raw-CLI sequences for cases where
the helper is too rigid.

## Groups

Workspace groups are collapsible sidebar collections anchored to a designated workspace. See
`references/workspace-groups.md` for the full group subcommand table and the anchor-workspace
concept.

To add a new workspace directly into a group at creation time:

```sh
cmux new-workspace --name feature-x --group workspace_group:1 --group-placement afterCurrent
```

## Rules

1. **Scope to the caller workspace.** `CMUX_WORKSPACE_ID` is your boundary; act outside it only
   when the task explicitly requires cross-workspace coordination.
2. **Use `--no-focus` during bulk creation.** Creating multiple workspaces or splits without
   `--no-focus` will jump the user's active view; always prefer `--focus false` / `--no-focus`
   when scripting.
3. **Never spawn agent sessions here.** Starting Claude, Codex, or opencode agent sessions
   (`new-surface --type agent-session`, `claude-teams`) is out of scope for this skill — that is
   the deferred super-agent layer's responsibility. Use `--command` / `--cmd` for plain shell
   commands only.

## Related skills

- **cmux-control** (hub) — mental model, targeting, preflight, pane I/O, signals.
- **cmux-browser** — in-cmux browser automation, navigate/wait/snapshot/act loop.
- **cmux-config** — `cmux.json` editing, config doctor/validate, `reload-config`.
