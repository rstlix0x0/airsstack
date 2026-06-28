---
name: cmux-control
description: Use when controlling the cmux terminal — creating or focusing windows/workspaces/panes/surfaces, sending input or reading output from a pane (send, send-key, read-screen, capture-pane, pipe-pane), inspecting layout (tree, top, identify, list-*), or using cmux coordination signals (events, wait-for, set-status, set-progress, notify, set-buffer, set-hook). The hub skill other cmux skills build on. Run cmux-preflight first.
---

# cmux Control

The `cmux-control` skill is the hub for all terminal-control operations inside cmux — creating
and focusing windows, workspaces, panes, and surfaces; sending input and reading output; inspecting
layout; and using the coordination signal primitives. All other cmux skills (`cmux-workspace`,
`cmux-browser`, `cmux-config`) build on the mental model and targeting conventions defined here.

Grounded on `cmux 0.64.17`. If the installed version differs, run `cmux --help` or
`cmux capabilities` and use that output as source of truth; the commands listed here are a
starting point, not a contract.

## Preflight

Before any automation, assert the control surface is live:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-control/scripts/cmux-preflight --json
```

A zero exit means the preflight is healthy; nonzero means stop — the socket is absent or the
daemon is not responding. Pass `--json` for a machine-readable status object. Never proceed past
a nonzero preflight.

## Mental model

```
window
└── workspace  (sidebar tab — one active at a time per window)
    └── pane   (split region inside the workspace)
        └── surface  (tab within a pane — terminal, browser, or agent-session)
            └── panel  (optional sub-split within a surface, e.g. right-sidebar)
```

- **window** — the top-level OS window (usually one).
- **workspace** — a sidebar tab; each workspace is a named group of panes. `CMUX_WORKSPACE_ID`
  identifies the caller's workspace.
- **pane** — a split region; splitting a pane produces sibling panes.
- **surface** — a tab within a pane. Terminals, browser webviews, and agent sessions are all
  surfaces. `CMUX_SURFACE_ID` identifies the caller's surface.
- **panel** — a sub-split of a surface (e.g. the right sidebar panel). Addressed with `--panel`.

## Targeting

Targets are UUIDs, short refs (`window:1`/`workspace:2`/`pane:3`/`surface:4`), or integer
indexes. See `references/targeting.md` for the full targeting rules, `--id-format`, socket path
configuration, and raw-socket access via `cmux rpc`.

`CMUX_WORKSPACE_ID` and `CMUX_SURFACE_ID` are auto-set in every cmux-managed terminal session;
most commands use them as defaults for `--workspace` and `--surface` so you rarely need to pass
those flags explicitly when acting on the caller's own context.

## Safe first commands

Read-only inspection — no side effects, safe to run any time:

```sh
cmux identify --json          # caller's window, workspace, pane, surface IDs
cmux tree                     # workspace hierarchy in the caller's window
cmux list-workspaces          # all workspaces in the caller's window
cmux list-pane-surfaces       # all surfaces inside each pane of the caller's workspace
cmux current-workspace        # ref and name of the focused workspace
```

## Common workflows

### (a) Create a split and run a command

```sh
cmux new-split right                          # open a right split in the caller workspace
cmux send "npm run dev"                       # send text to the focused surface
cmux send-key enter                           # confirm with Enter
```

### (b) Read a pane's output

```sh
cmux read-screen --lines 50                   # last 50 visible lines (no scrollback)
cmux capture-pane --scrollback                # full scrollback buffer (tmux-compat)
```

### (c) Stream a pane to a command

```sh
cmux pipe-pane --command "tee /tmp/pane.log"
```

### (d) Focus and close lifecycle

```sh
cmux focus-pane --pane pane:2                         # focus a specific pane
cmux close-surface                                    # close the caller's current surface
cmux close-workspace --workspace workspace:3          # close a workspace by ref
```

## Signals

A set of coordination primitives is built into cmux — event streams, signal barriers, status
indicators, notifications, a shared clipboard buffer, and lifecycle hooks. These are low-level
substrate; this skill documents their syntax but ships no coordination policy (that is deferred
to the future super-agent layer).

See `references/signals.md` for the full primitive table with syntax and one-line use.

## Command families

A grouped inventory of all commands covered by this skill is in
`references/command-families.md`. Use it as a quick-reference lookup before reaching for
`cmux --help`.

## Rules

1. **Don't steal focus when scripting.** Pass `--no-focus` (or `--focus false`) to creation and
   move commands; focus changes are visible to the user and disrupt their work.
2. **Scope to the caller workspace.** `CMUX_WORKSPACE_ID` is your scope boundary; act outside it
   only when the task explicitly requires cross-workspace coordination.
3. **Inspect before you act.** Run `cmux identify --json` and `cmux tree` first; acting on stale
   refs is a common failure mode.
4. **Treat the grounding version as advisory.** If a command is missing or its flags differ, run
   `cmux --help` or `cmux capabilities` and adapt; never assume the docs are more accurate than
   the live binary.

## Related skills

- **cmux-workspace** — workspace groups, multi-pane layouts, `cmux-layout` helper.
- **cmux-browser** — in-cmux browser automation, navigate/wait/snapshot/act loop.
- **cmux-config** — `cmux.json` editing, config doctor/validate, `reload-config`.
