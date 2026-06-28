# airsstack-cmux

Native [cmux](https://cmux.com) terminal control for Claude Code, delivered as four
lazily-loaded skills. Each skill is a thin, accurate guide over the real `cmux` CLI plus a
small helper script; they load only when a task matches, so they cost no context otherwise.

Grounded on `cmux 0.64.17`. When the installed cmux differs, the skills tell the agent to
run `cmux --help` / `cmux capabilities` to self-correct.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack-cmux@airsstack
```

Requires a cmux install on the machine (the skills drive the `cmux` binary; they do not
bundle it).

## Skills

- **cmux-control** (hub) — window/workspace/pane/surface lifecycle, `send`/`read-screen`/
  `capture-pane`/`pipe-pane`, inspection (`tree`/`top`/`identify`), and the coordination
  signal primitives (`events`/`wait-for`/`set-status`/`set-progress`/`notify`/`set-buffer`/
  `set-hook`). Ships `cmux-preflight`, a health guard to run before automation.
- **cmux-workspace** — caller-scoped automation, workspace groups, and multi-pane layouts.
  Ships `cmux-layout` (geometry + optional per-pane command; no agent spawning).
- **cmux-browser** — in-cmux browser automation (navigate → wait → snapshot → act →
  re-snapshot). Ships `cmux-snap`, a snapshot-then-act wrapper.
- **cmux-config** — `~/.config/cmux/cmux.json`, custom commands/actions, sidebars,
  shortcuts, and the `config doctor/check/validate` family. Ships `cmux-settings`, a
  backup→edit→validate safe-edit wrapper.

## Non-goals

The cmux super-agent (spawning and coordinating fresh Claude instances across panes via
`cmux claude-teams`) is a separate, future scope. This plugin documents the signal
primitives but ships no orchestration runtime.

## License

Apache-2.0. See [LICENSE](./LICENSE).
