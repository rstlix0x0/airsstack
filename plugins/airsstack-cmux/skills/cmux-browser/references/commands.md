# cmux browser — Command Reference

Full subcommand table for `cmux browser`, grounded on `cmux 0.64.17`.
Pass the surface as the first positional token or via `--surface <ref>`.
`open`/`open-split`/`new`/`identify` can run without an explicit surface.

---

## Navigation

| Subcommand | Purpose |
|---|---|
| `open [url]` | Open a browser surface in the caller workspace (defaults to `$CMUX_WORKSPACE_ID`). `--focus false` by default. |
| `open-split [url]` | Open a browser surface in a new split alongside the current surface. |
| `new [url]` | Alias for `open`; same options. |
| `goto <url>` | Navigate to `<url>`; supports `--snapshot-after`. |
| `navigate <url>` | Alias for `goto`. |
| `back` | Go to the previous page in history; supports `--snapshot-after`. |
| `forward` | Go to the next page in history; supports `--snapshot-after`. |
| `reload` | Reload the current page; supports `--snapshot-after`. |
| `url` | Print the current page URL. |
| `get-url` | Alias for `url`. |
| `focus-webview` | Give the embedded webview keyboard focus. |
| `is-webview-focused` | Exit 0 if the webview has focus, nonzero otherwise. |
| `disable` | Disable the browser surface (hide webview, release resources). |
| `enable` | Re-enable a disabled browser surface. |
| `status` | Print the current enable/disable status of the surface. |
| `focus-mode <enter\|exit\|toggle>` | Enter, exit, or toggle distraction-free focus mode (hides UI chrome). |
| `zoom <in\|out\|reset>` | Zoom the browser surface in, out, or back to default scale. |
| `devtools <toggle\|console>` | Open or toggle the browser DevTools panel; `console` opens directly to the console tab. |
| `react-grab <toggle>` | Toggle React DevTools element-grab mode to inspect React component trees. |

---

## Waiting

| Subcommand | Purpose |
|---|---|
| `wait` | Block until a condition is true. Flags: `--selector <css>` (element present), `--text <text>` (text visible), `--url-contains <text>` / `--url <text>` (URL matches), `--load-state interactive\|complete` (page load phase), `--function <js>` (truthy JS expression), `--timeout-ms <ms>` / `--timeout <seconds>`. |

Prefer `--load-state complete` after navigation and `--selector` before acting on a dynamically
rendered element.

---

## DOM Actions

All DOM action subcommands accept `--selector <css>` (or `<css>` as the first positional) and
support `--snapshot-after` to emit a fresh accessibility snapshot after the action.

| Subcommand | Purpose |
|---|---|
| `click [--selector <css>]` | Left-click an element. |
| `dblclick [--selector <css>]` | Double-click an element. |
| `hover [--selector <css>]` | Move the pointer over an element (no click). |
| `focus [--selector <css>]` | Focus an element without clicking it. |
| `check [--selector <css>]` | Check a checkbox or radio button. |
| `uncheck [--selector <css>]` | Uncheck a checkbox. |
| `scroll-into-view [--selector <css>]` | Scroll an element into the visible viewport. |
| `type [--selector <css>] [--text <text>]` | Append keystrokes to the focused/selected element. |
| `fill [--selector <css>] [--text <text>]` | Replace the element's value with `<text>` (clears first). |
| `press [--key <key>]` | Press a key (e.g. `Enter`, `Tab`, `Escape`). |
| `key [--key <key>]` | Alias for `press`. |
| `keydown [--key <key>]` | Send a keydown event only (no keyup). |
| `keyup [--key <key>]` | Send a keyup event only (no keydown). |
| `select [--selector <css>] [--value <value>]` | Select a `<select>` option by value. |
| `scroll [--selector <css>] [--dx <n>] [--dy <n>]` | Scroll by `dx`/`dy` pixels; omit `--selector` to scroll the viewport. |

---

## Inspection

Read-only commands — no side effects on page state.

| Subcommand | Purpose |
|---|---|
| `snapshot [--interactive\|-i] [--cursor] [--compact] [--max-depth <n>] [--selector <css>]` | Emit an accessibility snapshot of the page. Use `--interactive` for the agent-readable full tree; `--selector` to scope to a subtree; `--compact` for shorter output. |
| `screenshot [--out <path>]` | Capture the webview to a PNG file. |
| `get <url\|title\|text\|html\|value\|attr\|count\|box\|styles>` | Read a page property. `text`/`html`/`value`/`count`/`box`/`styles`/`attr` accept `--selector`; `attr` also takes `--attr <name>`; `styles` takes `--property <name>`. |
| `is <visible\|enabled\|checked> [--selector <css>]` | Boolean check: exit 0 if the condition is true, nonzero if false. |
| `find <role\|text\|label\|placeholder\|alt\|title\|testid\|first\|last\|nth>` | Locate elements. `role` takes `--name` and `--exact`; `text`/`label`/`placeholder`/`alt`/`title`/`testid` take `--exact`; `first`/`last` take `--selector`; `nth` takes `--index` and `--selector`. |
| `highlight [--selector <css>]` | Draw a visual highlight on an element for debugging; has no page-state side effects. |
| `identify [--surface <ref>]` | Print the browser surface's current state (URL, title, enabled, surface ref). |

---

## JavaScript

| Subcommand | Purpose |
|---|---|
| `eval [--script <js> \| <js>]` | Evaluate a JS expression in the page context and print the result. |
| `addinitscript [--script <js> \| <js>]` | Inject a JS snippet that runs before every page load (persists for the session). |
| `addscript [--script <js> \| <js>]` | Inject a JS snippet into the currently loaded page. |
| `addstyle [--css <css> \| <css>]` | Inject a CSS snippet into the current page. |

---

## Frames and Dialogs

| Subcommand | Purpose |
|---|---|
| `frame <main\|selector> [--selector <css>]` | Switch the active context to `main` (top frame) or to the iframe matched by `--selector`. All subsequent DOM commands target that frame until `frame main` is called. |
| `dialog <accept\|dismiss> [text]` | Accept or dismiss a native browser `alert`/`confirm`/`prompt`. Use `text` to supply the prompt value for `accept`. Prefer `console list` + `errors list` over triggering dialogs in automation. |

---

## State and Session

| Subcommand | Purpose |
|---|---|
| `cookies <get\|set\|clear>` | Read, write, or clear cookies. `get`/`set` accept `--name`, `--value`, `--url`, `--domain`, `--path`, `--expires`, `--secure`, `--all`. |
| `storage <local\|session> <get\|set\|clear>` | Read, write, or clear `localStorage` or `sessionStorage`. |
| `state <save\|load> <path>` | Save or restore full browser session state (cookies + storage) to/from a file. |
| `profiles <list\|add\|rename\|clear\|delete>` | Manage named browser profiles. |
| `import` | Import cookies/storage from another browser. Flags: `--from <browser>`, `--profile <name>`, `--all-profiles`, `--to-profile <name\|uuid>`, `--create-profile`, `--domain <domain>`. Use `--interactive` / `--yes` to control prompts. |
| `history clear --force` | Clear the default profile's browsing history (mirrors the View menu). `--force` is required. |
| `download [wait]` | Wait for a download to complete; `--path <path>` to set the target, `--timeout-ms` / `--timeout` to control the wait. |

---

## Tabs

| Subcommand | Purpose |
|---|---|
| `tab <new\|list\|switch\|close\|<index>>` | Manage browser tabs inside the surface: `new` opens a tab, `list` enumerates tabs, `switch` / `<index>` activates a tab by index, `close` closes the active tab. |

---

## Diagnostics

| Subcommand | Purpose |
|---|---|
| `console <list\|clear>` | List or clear browser console messages (log, warn, error). Use instead of `alert()` for diagnostic output. |
| `errors <list\|clear>` | List or clear uncaught JS errors on the page. |

---

## Network and Device Emulation

| Subcommand | Purpose |
|---|---|
| `viewport <width> <height>` | Set the webview viewport dimensions in pixels. |
| `geolocation <latitude> <longitude>` / `geo <lat> <lon>` | Override the page's geolocation. |
| `offline <true\|false>` | Toggle network offline mode for the surface. |
| `trace <start\|stop> [path]` | Start or stop a Playwright-style network trace; optionally write to `path`. |
| `network route <pattern> [--abort] [--body <text>]` | Intercept and fulfill or abort requests matching `pattern`. |
| `network unroute <pattern>` | Remove a route interception. |
| `network requests` | List captured network requests. |
| `screencast <start\|stop>` | Start or stop screen recording of the surface. |
| `input <mouse\|keyboard\|touch> [args...]` | Inject raw input events (mouse move/click, keyboard, touch). Aliases: `input_mouse`, `input_keyboard`, `input_touch`. |

---

## Notes

- `--snapshot-after` is accepted by all DOM action subcommands (`goto`, `navigate`, `back`,
  `forward`, `reload`, `click`, `dblclick`, `hover`, `focus`, `check`, `uncheck`,
  `scroll-into-view`, `type`, `fill`, `press`, `key`, `keydown`, `keyup`, `select`, `scroll`)
  and emits a fresh accessibility snapshot after the action completes.
- `find` locators: `role` (ARIA role, optional `--name`/`--exact`), `text` (visible text),
  `label` (form label), `placeholder`, `alt`, `title`, `testid` (data-testid), `first`, `last`,
  `nth` (index-based, `--index <n>`).
- Surface syntax: positional `surface:2` or `--surface surface:2`; UUIDs are also accepted.
  See `cmux-control` `## Targeting` for the full targeting rules.
