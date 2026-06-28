# cmux.json Structure

Reference for `~/.config/cmux/cmux.json` — the primary cmux-owned configuration file.
Format: JSONC (JSON with comments). Schema:
`https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json`

Fetch the live schema:
```sh
curl -fsSL https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json
```

**File precedence** (first match wins):
1. `./.cmux/cmux.json` — project-local override
2. `./cmux.json` — project-root override
3. `~/.config/cmux/cmux.json` — global user config (normal edit target)

Legacy fallbacks (`~/.config/cmux/settings.json`,
`~/Library/Application Support/com.cmuxterm.app/settings.json`) supply only keys absent
from the primary.

**Backup rule:** before any edit, copy the file to a timestamped `.bak`:
```sh
cp ~/.config/cmux/cmux.json ~/.config/cmux/cmux.json.$(date +%Y%m%d-%H%M%S).bak
```
Or use `cmux-settings backup-then <cmd>` to do this automatically with validation and
rollback.

---

## Top-level keys

| Key | Purpose |
|-----|---------|
| `$schema` | Schema URL (keep as-is; validated against the live schema) |
| `schemaVersion` | Schema version integer (currently `1`) |
| `actions` | Action registry — custom entries for tab bar, Command Palette, shortcuts, plus-button |
| `ui` | UI wiring — surface tab bar buttons and plus-button context menu |
| `commands` | Custom shell commands available from the workspace context menu |
| `newWorkspaceCommand` | Default shell command run in every new workspace |
| `workspaceGroups` | Per-cwd customization for sidebar workspace groups |
| `surfaceTabBarButtons` | Legacy root-level tab bar buttons (prefer `ui.surfaceTabBar.buttons`) |
| `app` | App-level behavior: appearance, placement, focus, quit policy |
| `terminal` | Terminal behavior: text box, scroll bar, copy-on-select, agent hibernation |
| `notifications` | Notification sound, badges, hooks, and command |
| `sidebar` | Sidebar display: git status, ports, PRs, metadata, workspace descriptions |
| `workspaceColors` | Custom color palette for workspace tinting |
| `sidebarAppearance` | Sidebar tint color and opacity |
| `automation` | Integration settings: Claude Code, Codex, Kiro, Gemini, Cursor, socket auth |
| `browser` | Embedded browser behavior: search engine, localhost allowlist, tab discard |
| `shortcuts` | Key binding overrides (see `cmux shortcuts` for the default map) |
| `markdown` | Markdown viewer font and max-width |
| `canvas` | Canvas settings |
| `fileEditor` | File editor word-wrap |
| `fileExplorer` | File explorer settings |
| `diffViewer` | Diff viewer settings |
| `vault` | Vault integration |

---

## `actions` registry

`actions` is an object whose keys are action identifiers and whose values describe each
action. Actions registered here appear in the Command Palette, can be bound to keyboard
shortcuts, and can be wired into the surface tab bar and plus-button menu via `ui`.

```jsonc
{
  "actions": {
    "my-build": {
      "type": "command",          // command | agent | builtin | workspaceCommand
      "title": "Run build",       // label in Command Palette / tab bar button
      "command": "npm run build", // shell command to run (for type=command)
      "target": "current",        // where to run: current | new | split
      "shortcut": "cmd+shift+b",  // optional keyboard shortcut
      "palette": true,            // show in Command Palette (default true)
      "icon": "play"              // SF Symbol name or built-in icon identifier
    }
  }
}
```

### `type` values

| Value | Meaning |
|-------|---------|
| `command` | Run a shell command in the target surface |
| `agent` | Launch an agent session (Claude Code, Codex, etc.) |
| `builtin` | Invoke one of cmux's built-in commands by name |
| `workspaceCommand` | Run a command scoped to the current workspace cwd |

---

## `commands` array

`commands` is an array of custom shell commands that appear in the workspace context menu
(right-click or plus-button) and the Command Palette.

```jsonc
{
  "commands": [
    {
      "name": "Run tests",          // display label
      "command": "npm test",        // shell command
      "confirm": false,             // ask for confirmation before running (default false)
      "keywords": ["test", "jest"]  // extra search keywords for the Command Palette
    }
  ]
}
```

---

## UI integration points (`ui`)

`ui` wires actions into specific UI surfaces. Two main integration points:

### `ui.surfaceTabBar.buttons`

Array of action identifiers (strings) or inline action objects to display as buttons in
the surface tab bar (the row of tab pills above each pane).

```jsonc
{
  "ui": {
    "surfaceTabBar": {
      "buttons": ["my-build", "my-test"]
    }
  }
}
```

### `ui.newWorkspace.contextMenu`

Array of action identifiers to add to the new-workspace plus-button context menu.

```jsonc
{
  "ui": {
    "newWorkspace": {
      "contextMenu": ["my-build"]
    }
  }
}
```

> **Legacy:** `surfaceTabBarButtons` at the root level is equivalent to
> `ui.surfaceTabBar.buttons` but deprecated. Prefer `ui.surfaceTabBar.buttons` for new
> configs.

---

## Sidebar and workspace-group config

### `sidebar`

Controls which metadata columns appear in the workspace sidebar:

```jsonc
{
  "sidebar": {
    "showSSH": true,
    "showPorts": true,
    "showPullRequests": true,
    "watchGitStatus": true,
    "showBranchDirectory": true,
    "showProgress": true,
    "showLog": true,
    "showCustomMetadata": true,
    "showNotificationMessage": true,
    "showWorkspaceDescription": true,
    "wrapWorkspaceTitles": false,
    "pathLastSegmentOnly": false,
    "branchLayout": "vertical",
    "openPortLinksInCmuxBrowser": true,
    "openPullRequestLinksInCmuxBrowser": true,
    "makePullRequestsClickable": true,
    "hideAllDetails": false,
    "stackBranchDirectory": false,
    "showLog": true
  }
}
```

### `workspaceGroups`

Per-cwd customization for sidebar workspace groups. The anchor workspace's cwd is matched
against keys in `byCwd`; longest-match wins. Keys with `*` or `?` are treated as fnmatch
globs (`~` is expanded); other keys are path prefixes.

```jsonc
{
  "workspaceGroups": {
    "newWorkspacePlacement": "afterCurrent",  // afterCurrent | top | end
    "byCwd": {
      "~/Projects/myapp": {
        "groupName": "myapp",
        "color": "#1565C0",
        "icon": "folder"
      },
      "~/Projects/*": {
        "groupName": "Projects"
      }
    }
  }
}
```

`newWorkspacePlacement` controls where `Cmd-N` inside a group, the group header plus-button,
and configured group actions place the new workspace:
- `afterCurrent` — after the active in-group workspace (default)
- `top` — second slot, just after the anchor
- `end` — after the last member

---

## `shortcuts`

Override the default key bindings. The binding map uses action
identifier strings as keys and key combo strings (or arrays for chord sequences) as values.

```jsonc
{
  "shortcuts": {
    "bindings": {
      "commandPalette": "cmd+shift+p",
      "newTab": "cmd+n",
      "splitRight": "cmd+d",
      "splitDown": "cmd+shift+d",
      "closeTab": "cmd+w",
      "closeWorkspace": "cmd+shift+w",
      "renameWorkspace": "cmd+shift+r",
      "reloadConfiguration": "cmd+shift+,",
      "diffViewerScrollToTop": ["g", "g"]   // chord: two sequential keypresses
    }
  }
}
```

Run `cmux shortcuts` to see the full default binding map before overriding.

---

## Reload flow

After editing cmux.json:

```sh
cmux config validate    # verify JSONC syntax + schema compliance
cmux reload-config      # reload cmux.json AND ~/.config/ghostty/config in place
```

No app restart is needed. `reload-config` refreshes all terminals immediately.
