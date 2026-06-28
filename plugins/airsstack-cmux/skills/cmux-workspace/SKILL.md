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
cmux new-workspace --name <title> [--cwd <path>] [--command <cmd>] [--focus false]
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
- `--focus false` — create without stealing focus (recommended when scripting). There is
  **no** `--no-focus` flag on `new-workspace`; use `--focus false`.

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
that pane. It does **not** spawn agent sessions or coordinate agents. On success it prints the
new workspace ref (e.g. `workspace:5`) so you can `cmux select-workspace --workspace <ref>`
afterwards. Internally it pins every split and command to a captured surface ref — it never
relies on the caller's focus or `$CMUX_SURFACE_ID`, so the layout always lands in the new
workspace and is never leaked into the caller's surface. See `references/layouts.md` for more
recipes and the equivalent raw-CLI sequences for cases where the helper is too rigid.

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
2. **Use `--focus false` during bulk creation.** Creating multiple workspaces or splits while
   focusing them jumps the user's active view; always pass `--focus false` when scripting.
   (`new-workspace` has no `--no-focus` flag — it is `--focus false`.)
3. **Target surfaces explicitly; never rely on focus.** cmux CLI commands default
   `--workspace`/`--surface` to the caller's `$CMUX_WORKSPACE_ID`/`$CMUX_SURFACE_ID`, NOT to
   whatever is focused. A bare `cmux send "..."` goes to the caller's own surface — inside Claude
   Code that injects text into the agent prompt. When acting on another workspace, capture its
   surface refs and pass `--surface`/`--workspace` on every call. Note: a backgrounded
   workspace's surface cannot be split via `--surface` (`not_found`); split it with `--workspace`.
4. **Never spawn agent sessions here.** Starting Claude, Codex, or opencode agent sessions
   (`new-surface --type agent-session`, `claude-teams`) is out of scope for this skill — that is
   the deferred super-agent layer's responsibility. Use `--command` / `--cmd` for plain shell
   commands only.

## Related skills

- **cmux-control** (hub) — mental model, targeting, preflight, pane I/O, signals.
- **cmux-browser** — in-cmux browser automation, navigate/wait/snapshot/act loop.
- **cmux-config** — `cmux.json` editing, config doctor/validate, `reload-config`.
