---
name: cmux-browser
description: Use when automating a browser inside cmux — opening or navigating a webview surface, waiting for selectors/text/load state, snapshotting the DOM, finding elements by role/text/label/testid, clicking/typing/filling, evaluating JS, screenshotting, or reading console/errors/cookies/storage. Follow the navigate→wait→snapshot→act→re-snapshot loop. Builds on cmux-control for surface targeting.
---

# cmux Browser

The `cmux-browser` skill drives the webview surfaces that cmux embeds in panes — open a URL,
navigate, wait for the page to settle, snapshot the accessibility tree, act (click, fill, press),
and re-snapshot to confirm the outcome. Every browser surface is a cmux surface first, so the
targeting and preflight conventions from `cmux-control` apply here too.

Grounded on `cmux 0.64.17`. If the installed version differs, run `cmux browser --help` and
treat that output as the source of truth.

For surface targeting — UUIDs, short refs (`surface:2`), `CMUX_SURFACE_ID`, socket path — see
`cmux-control` skill, `## Targeting` section.

## Default rule

Operate on the caller workspace's browser surface. Pass the surface as the first positional
argument (`<surface>`) or via `--surface <ref>` to target a specific one:

```sh
cmux browser surface:2 snapshot --interactive
cmux browser --surface surface:2 get url
```

When `CMUX_SURFACE_ID` is set (it is in every cmux-managed terminal session), `open`/`open-split`/
`new` default to the caller's workspace for `--workspace`; other subcommands require an explicit
surface ref.

## Core loop

The reliable browser-automation loop is: **navigate → wait → snapshot → act → re-snapshot**.
Never act on a stale snapshot; always re-snapshot after any action that changes the page.

```sh
# 1. Navigate and wait for the page to be interactive
cmux browser surface:2 goto https://example.com
cmux browser surface:2 wait --load-state complete

# 2. Snapshot the accessibility tree before acting
cmux browser surface:2 snapshot --interactive

# 3. Act — click appends --snapshot-after automatically
cmux browser surface:2 click "button[type=submit]" --snapshot-after

# 4. Verify the outcome
cmux browser surface:2 get url
cmux browser surface:2 get title
```

The packaged snapshot-then-act helper does steps 2–3 in one call:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-browser/scripts/cmux-snap surface:2 click "button[type=submit]"
```

`cmux-snap` emits a `snapshot --interactive` first, then the action with `--snapshot-after`, so
the agent always sees the view before and after acting.

## Command groups

The full `cmux browser` subcommand table is in `references/commands.md`, organized into groups:
navigation, waiting, DOM actions, inspection, JavaScript, frames and dialogs, state and session,
tabs, diagnostics, and network/device emulation.

## Common patterns

Worked recipes are in `references/recipes.md`: navigate-wait-inspect, form fill with verification,
debug capture, session save/restore, and durable screenshot.

## Rules

1. **Re-snapshot before acting on a changed view.** A stale accessibility tree causes wrong
   selectors. Always call `snapshot --interactive` (or use `cmux-snap`) when the page may have
   changed since the last snapshot.
2. **Verify URL after navigate.** Call `cmux browser <surface> get url` after every `goto`/
   `navigate`; redirects and SSO flows are common failure modes.
3. **This is a cmux surface, not a standalone browser.** Focus it with `focus-webview` when
   keyboard input is needed; check `is-webview-focused` before `type`/`press`.
4. **Never trigger blocking JS dialogs.** Use `cmux browser <surface> console list` and `errors
   list` for diagnostic output instead of `alert()`/`confirm()` — blocking dialogs freeze the
   surface until dismissed and break the automation loop.

## Related skills

- **cmux-control** (hub) — surface/pane/workspace lifecycle, targeting, and preflight.
- **cmux-workspace** — multi-pane layouts and workspace groups.
- **cmux-config** — `cmux.json` editing, config doctor/validate, `reload-config`.
