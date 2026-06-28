# cmux Command Families

Grouped lookup of CLI commands, grounded on `cmux 0.64.17`. Each entry has a one-line purpose.
Run `cmux --help` to self-correct if a command is missing or its flags have changed.

---

## Windows

| Command | Purpose |
|---|---|
| `new-window` | Open a new OS window. |
| `list-windows` | List all open windows. |
| `current-window` | Show the ref/ID of the focused window. |
| `focus-window` | Bring a window to the front by ID. |
| `close-window` | Close a window. |
| `next-window` | Move focus to the next window. |
| `previous-window` | Move focus to the previous window. |
| `last-window` | Move focus to the most recently used window. |

---

## Workspaces

| Command | Purpose |
|---|---|
| `new-workspace` | Create a workspace (sidebar tab) with optional name, cwd, command, or group. |
| `list-workspaces` | List all workspaces in a window. |
| `select-workspace` | Switch to a workspace by ref or index. |
| `current-workspace` | Show the ref and name of the active workspace. |
| `rename-workspace` | Rename a workspace. |
| `close-workspace` | Close a workspace. |
| `reorder-workspace` | Move a workspace to a new index position or before/after another. |
| `move-workspace-to-window` | Move a workspace from one window to another. |

---

## Panes and Surfaces

| Command | Purpose |
|---|---|
| `new-split` | Split the current pane in a direction (`left`/`right`/`up`/`down`). |
| `new-pane` | Create a new pane (terminal or browser type) in a workspace. |
| `new-surface` | Create a new surface tab (terminal, browser, or agent-session). |
| `close-surface` | Close a surface tab. |
| `move-surface` | Move a surface to a different pane or workspace. |
| `split-off` | Detach a surface into its own new split pane. |
| `focus-pane` | Focus a pane by ref. |
| `list-panes` | List all panes in a workspace. |
| `list-pane-surfaces` | List all surface tabs inside a pane. |
| `swap-pane` | Swap the positions of two panes. |
| `break-pane` | Break a pane out of its current workspace into a new split position. |
| `join-pane` | Join a pane into a target pane. |
| `resize-pane` | Resize a pane left/right/up/down by n cells. |

---

## I/O

| Command | Purpose |
|---|---|
| `send` | Send text to the active surface (inserts as if typed). |
| `send-key` | Send a single key (e.g. `enter`, `tab`, `ctrl-c`) to a surface. |
| `send-panel` | Send text to a panel (sub-split) of a surface. |
| `send-key-panel` | Send a key to a panel. |
| `read-screen` | Read the visible lines of a surface; `--scrollback` for the full buffer. |
| `capture-pane` | Capture pane output — tmux-compatible alias for `read-screen`. |
| `pipe-pane` | Pipe all subsequent pane output to an external shell command. |

---

## Inspection

| Command | Purpose |
|---|---|
| `identify` | Show the caller's window/workspace/pane/surface IDs; `--json` for structured output. |
| `tree` | Print a hierarchy tree of workspaces and panes. |
| `top` | Show live resource usage (CPU, memory, process count) per pane. |
| `memory` | Report memory usage by workspace group. |
| `ping` | Check reachability via the control socket; exits 0 when healthy. |
| `version` | Print the cmux version string. |
| `capabilities` | List the full JSON-RPC method set supported by the running process. |

---

## Status and Notifications

| Command | Purpose |
|---|---|
| `notify` | Send a macOS notification attached to a workspace or surface. |
| `list-notifications` | List pending notifications. |
| `set-status` | Write a keyed status indicator (icon, colour, priority) to the workspace status bar. |
| `clear-status` | Remove a status indicator by key. |
| `list-status` | List all active status indicators in a workspace. |
| `set-progress` | Set a workspace progress bar (0.0–1.0) with an optional label. |
| `clear-progress` | Remove the workspace progress bar. |
| `log` | Append a structured message to the workspace log. |
| `list-log` | Read the workspace log (most-recent first; `--limit` to cap). |
