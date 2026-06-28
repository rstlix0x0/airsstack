# Config Command Family

Reference for the `config`, `settings`, `shortcuts`, and `reload-config` command families
in `cmux 0.64.17`. All entries are grounded on `cmux --help`.

---

## `cmux config`

```
cmux config <doctor|check|validate|path|paths|docs|documentation|reload>
```

| Subcommand | Purpose |
|------------|---------|
| `doctor` | Full health report: locate primary config file, report byte size and top-level keys, verify JSONC syntax, and print docs/schema URLs. Use this first when diagnosing config issues. |
| `check` | Alias for `doctor`. |
| `validate` | JSONC syntax and schema compliance check only; exits 0 on success, nonzero on error. Use in scripts before calling `reload-config`. |
| `path` | Print the path of the primary config file (`~/.config/cmux/cmux.json`). |
| `paths` | Print all config file paths in precedence order (primary, legacy, app-support). |
| `docs` | Print the docs URL, schema URL, and useful commands. Same as `cmux docs settings`. |
| `documentation` | Alias for `docs`. |
| `reload` | Alias for `cmux reload-config` (see below). |

Usage patterns:

```sh
cmux config doctor          # diagnose: location, size, keys, syntax
cmux config validate        # syntax + schema check only (scriptable)
cmux config path            # print primary config path
cmux config paths           # print all config paths in order
cmux config docs            # print docs and schema URLs
```

---

## `cmux settings`

```
cmux settings [open [target]|path|docs|<target>]
```

| Form | Purpose |
|------|---------|
| `cmux settings` | Open the Settings UI in cmux. |
| `cmux settings open` | Open the Settings UI (same as bare `cmux settings`). |
| `cmux settings open <target>` | Open Settings UI scrolled to a named section. |
| `cmux settings path` | Print all config file paths (equivalent to `cmux config paths`). |
| `cmux settings docs` | Print settings documentation, schema URL, and useful commands. |
| `cmux settings <target>` | Jump to or inspect a named settings target (e.g. `cmux-json`, `shortcuts`). |
| `cmux settings cmux-json` | Print the template cmux.json with all keys commented out — use as a starting point. |

Usage patterns:

```sh
cmux settings               # open Settings UI
cmux settings path          # print config file paths
cmux settings docs          # print settings documentation
cmux settings cmux-json     # print cmux.json template (all keys commented out)
```

---

## `cmux reload-config`

```
cmux reload-config
```

Reloads **both** `~/.config/cmux/cmux.json` and `~/.config/ghostty/config` in place and
refreshes all terminals immediately. No app restart needed. Run this after every successful
edit and validation.

```sh
cmux config validate && cmux reload-config
```

---

## `cmux shortcuts`

```
cmux shortcuts
```

Prints the full default key binding map (action identifier → key combo). Use this before
adding `shortcuts.bindings` overrides in cmux.json to see what is already bound.

---

## `cmux sidebar`

```
cmux sidebar <validate|reload|select|open> [name]
```

| Subcommand | Purpose |
|------------|---------|
| `validate` | Validate the sidebar configuration. |
| `reload` | Reload sidebar configuration in place. |
| `select` | Select (focus) a named sidebar. |
| `open` | Open a named sidebar by name. |

---

## `cmux docs`

```
cmux docs [settings|shortcuts|api|browser|agents|dock|sidebars]
```

| Subcommand | Purpose |
|------------|---------|
| `docs settings` | Print settings documentation, schema URL, and cmux.json paths. |
| `docs shortcuts` | Print shortcuts documentation. |
| `docs api` | Print API/RPC documentation. |
| `docs browser` | Print browser automation documentation. |
| `docs agents` | Print agent integration documentation. |
| `docs dock` | Print Dock controls documentation. |
| `docs sidebars` | Print sidebar customization documentation. |

---

## Safe-edit workflow (summary)

```sh
# 1. Inspect — no backup needed
cmux config doctor
cmux settings path

# 2. Back up + edit + validate (use the helper)
${CLAUDE_PLUGIN_ROOT}/skills/cmux-config/scripts/cmux-settings backup-then <edit-command>

# 3. Reload
cmux reload-config

# 4. Verify
cmux config doctor
```

If `cmux-settings backup-then` exits nonzero, the backup has been restored automatically —
no manual cleanup required.
