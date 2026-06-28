# cmux Signal Primitives

Low-level coordination substrate built into cmux. These primitives let scripts, agent sessions,
and external processes communicate asynchronously — event streams, barriers, status slots,
notifications, a shared buffer, and lifecycle hooks. Grounded on `cmux 0.64.17`.

> **Scope note:** This skill documents the primitives and their syntax. No coordination policy
> is shipped here; the future super-agent layer is the planned home for orchestration built on
> top of these primitives.

---

## Event stream

```sh
cmux events [--after <seq>] [--cursor-file <path>] [--name <event>] \
            [--category <category>] [--reconnect] [--limit <n>]
```

Opens a push stream of lifecycle events emitted by cmux. Each event carries a monotonically
increasing sequence number (`seq`).

| Flag | Purpose |
|---|---|
| `--after <seq>` | Resume from a known sequence number (catch-up replay). |
| `--cursor-file <path>` | Persist the last-seen seq to a file; auto-resumes across restarts. |
| `--name <event>` | Filter to one event name. |
| `--category <category>` | Filter to one event category. |
| `--reconnect` | Automatically reconnect on disconnect. |
| `--limit <n>` | Stop after receiving n events. |

---

## Signal barrier

```sh
cmux wait-for [-S|--signal] <name> [--timeout <seconds>]
```

Block until a named signal is emitted (or the timeout elapses). Exit 0 on signal received;
nonzero on timeout.

Use as a lightweight synchronisation point between panes or scripts — one side completes work
and sends the signal (out-of-band or via a `set-hook`); the waiter unblocks and proceeds.

---

## Status indicators

```sh
cmux set-status <key> <value> [--workspace <ref>] [--icon <name>] [--color <#hex>] [--priority <n>]
cmux clear-status <key> [--workspace <ref>]
cmux list-status [--workspace <ref>]
```

Write a keyed indicator to the workspace status bar. Multiple indicators are keyed independently
so different scripts can coexist without overwriting each other. Use `--priority` to control
display order; higher numbers appear first.

---

## Progress bar

```sh
cmux set-progress <0.0-1.0> [--label <text>] [--workspace <ref>]
cmux clear-progress [--workspace <ref>]
```

Display a progress bar in the workspace header (0.0 = empty, 1.0 = full). One active bar per
workspace; subsequent `set-progress` calls overwrite. Always `clear-progress` when the task
completes to avoid a stale indicator.

---

## Notifications

```sh
cmux notify --title <text> [--subtitle <text>] [--body <text>] \
            [--workspace <ref>] [--surface <ref>]
```

Send a macOS notification anchored to a workspace or surface. Clicking the notification jumps
the user to that context. Use sparingly — prefer `set-status` for persistent state that the
user monitors at a glance.

---

## Shared buffer

```sh
cmux set-buffer [--name <name>] <text>
cmux paste-buffer [--name <name>] [--workspace <ref>] [--surface <ref>]
cmux list-buffers
```

A named clipboard buffer shared across all surfaces. Write with `set-buffer`, paste into a
terminal surface with `paste-buffer`. Multiple named buffers are supported simultaneously.
Modelled on the tmux buffer API.

---

## Lifecycle hooks

```sh
cmux set-hook <event> <command>
cmux set-hook --unset <event>
cmux set-hook --list
```

Register a shell command to run when a lifecycle event fires (e.g. workspace created or closed,
surface focused, agent session started). The hook command runs in the background and can itself
call back into `cmux` to drive further state changes or signal waiting processes.

---

## Workspace log

```sh
cmux log [--level <level>] [--source <name>] [--workspace <ref>] <message>
cmux list-log [--workspace <ref>] [--limit <n>]
```

Append structured log messages to the workspace log. Useful for leaving an audit trail when
multiple scripts or agent sessions operate concurrently on the same workspace. `list-log` reads
the log in reverse-chronological order; `--limit` caps the result count.

---

## Substrate note

The primitives above are the foundation a future super-agent coordination runtime will build on.
`cmux-control` ships no policy — it documents the substrate so the agent can use the primitives
directly until the higher-level runtime is available.
