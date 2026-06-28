# cmux Targeting

How to address windows, workspaces, panes, and surfaces in the cmux CLI.
Grounded on `cmux 0.64.17`.

---

## Ref formats

Three ref formats are accepted wherever a `--window`, `--workspace`, `--pane`, or `--surface`
flag is expected:

| Format | Example | Notes |
|---|---|---|
| UUID | `3f1a2b4c-…` | Stable across restarts. Preferred for long-lived scripts. |
| Short ref | `window:1`, `workspace:2`, `pane:3`, `surface:4` | 1-based index within the parent context. Compact and readable. |
| Integer index | `1`, `2` | Positional; same as the `N` in short refs. Order follows creation sequence. |

To include UUIDs in output, pass `--id-format uuids` (UUIDs only) or `--id-format both`
(short ref + UUID). Default output uses short refs.

---

## Environment defaults

In every cmux-managed terminal session these env vars are auto-set and used as flag defaults:

| Variable | Used as default for |
|---|---|
| `CMUX_WORKSPACE_ID` | `--workspace` in nearly all commands (`send`, `new-split`, `notify`, etc.) |
| `CMUX_SURFACE_ID` | `--surface` in I/O and inspection commands |
| `CMUX_TAB_ID` | `--tab` in `tab-action` and `rename-tab` |
| `CMUX_SOCKET_PATH` | Unix socket path (see below) |

Pass `--workspace` or `--surface` explicitly only when targeting a *different* workspace or
surface than the caller's own.

---

## Socket path

The CLI reaches the running process via a Unix socket:

```
~/.local/state/cmux/cmux.sock       # default
${CMUX_SOCKET_PATH}                 # override
```

Override `CMUX_SOCKET_PATH` to target a debug or tagged socket. The cmux-preflight helper
(`${CLAUDE_PLUGIN_ROOT}/skills/cmux-control/scripts/cmux-preflight`) validates that the socket
is present and `cmux ping` succeeds before automation begins.

---

## JSON output

Most commands accept `--json` to emit machine-readable structured output. Prefer `--json`
in scripts to avoid parsing human-formatted text:

```sh
cmux identify --json
cmux list-workspaces --json
```

---

## Raw socket access

For operations not exposed as CLI subcommands, use the JSON-RPC socket directly:

```sh
cmux rpc <method> [json-params]
```

`cmux capabilities` prints the full method list supported by the running process. Prefer
CLI subcommands first; reach for `cmux rpc` only when a required method has no CLI surface.
