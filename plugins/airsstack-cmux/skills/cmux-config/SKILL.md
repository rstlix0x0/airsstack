---
name: cmux-config
description: Use when inspecting or editing cmux configuration — ~/.config/cmux/cmux.json, custom commands and JSON actions, sidebar/workspace-group config, keyboard shortcuts, or running cmux config doctor/check/validate and reload-config. Always back up before editing. Builds on cmux-control for the preflight convention.
---

# cmux Config

The `cmux-config` skill covers inspecting and editing cmux's owned configuration —
`~/.config/cmux/cmux.json`, custom commands and JSON actions, sidebar and workspace-group
settings, keyboard shortcuts, and the `config doctor/check/validate/reload` family.

Grounded on `cmux 0.64.17`. If the installed version differs, run `cmux --help` or
`cmux docs settings` and treat that output as source of truth.

## Safe editing

cmux.json is **JSONC** (JSON with comments), and the default file cmux writes is a heavily
commented template — every key ships commented out with an "uncomment to enable" note. **Edit it
in place as text** (uncomment/modify the relevant block) so those comments survive. Do **not**
pipe it through `jq`: `jq` parses strict JSON only, so it errors on the commented template
(`Invalid numeric literal …`), and even on a comment-free config it rewrites the file as plain
JSON, **destroying every comment**. See the `jq` caveat below.

**Always back up before editing.** Take the timestamped `.bak` first, then edit in place, then
validate and reload:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-config/scripts/cmux-settings backup   # writes cmux.json.<ts>.bak
# …edit ~/.config/cmux/cmux.json in place (text editor / Edit tool), preserving comments…
${CLAUDE_PLUGIN_ROOT}/skills/cmux-config/scripts/cmux-settings validate # cmux config validate; nonzero on bad config
cmux reload-config
```

If `validate` fails, restore the most recent `.bak` by hand (`cp …bak ~/.config/cmux/cmux.json`)
and re-edit.

For a fully **scripted, non-interactive** edit, the helper's `backup-then <cmd>` form backs up,
runs `<cmd>`, then validates and **auto-restores the backup on failure** — the config is never
left broken:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-config/scripts/cmux-settings backup-then <cmd>
```

`<cmd>` must be a JSONC-aware edit (e.g. `sed`/`patch` on the commented file, or a JSONC tool) —
**not** a bare `jq` rewrite, for the reasons above. Only reach for `jq` if you have first
confirmed the target config has no comments and you accept that any that exist will be stripped.

Read-only commands (`cmux settings path`, `cmux config path`, `cmux config doctor`) inspect the
current config without modifying it; no backup is needed before running them.

> **`config validate` / `config check` are aliases for `config doctor`** in cmux 0.64.17 — they
> print the same full doctor report (header `cmux config doctor`) rather than a terse pass/fail,
> but still exit 0 on a valid config and nonzero on an invalid one, so they remain script-safe.

For the preflight convention (asserting the socket is live before automation), see the
cmux-control hub skill.

## Config file

Primary location: `~/.config/cmux/cmux.json`. Precedence:

1. `./.cmux/cmux.json` (project-local, if present)
2. `./cmux.json` (project-root, if present)
3. `~/.config/cmux/cmux.json` (global user config — the normal edit target)

Legacy fallback files (`~/.config/cmux/settings.json`,
`~/Library/Application Support/com.cmuxterm.app/settings.json`) are read only for keys
absent from the primary. Do not edit the legacy files; edit the primary.

The format is JSONC (JSON with comments). The schema is available at:
`https://raw.githubusercontent.com/manaflow-ai/cmux/main/web/data/cmux.schema.json`

See `references/cmux-json.md` for the key structure, `actions` registry, `commands` array,
UI wiring points, and sidebar/workspace-group config.

## Validate & reload

After any edit, validate then reload:

```sh
cmux config validate      # JSONC syntax check + schema check; exits 0 on success
cmux reload-config        # reloads cmux.json AND ~/.config/ghostty/config in place;
                          # no app restart needed
```

The full config command family (`doctor`, `check`, `validate`, `path`, `paths`, `docs`,
`reload`) and the `settings` and `shortcuts` commands are documented in
`references/config-commands.md`.

## Rules

1. **Back up before every edit.** Run `cmux-settings backup` (manual in-place edit) or
   `cmux-settings backup-then <cmd>` (scripted edit) for any mutation; never edit cmux.json
   without a `.bak` copy. Edit the JSONC in place to preserve comments — never `jq`-rewrite it
   (see "Safe editing").
2. **Validate after every edit.** Run `cmux config validate` (or rely on `cmux-settings`,
   which does this automatically) before reloading.
3. **Prefer Ghostty config for terminal behavior.** Font, cursor, theme, scrollback,
   background transparency (`background-opacity`), and blur (`background-blur`) belong in
   `~/.config/ghostty/config`, not in cmux.json.
4. **Reload with `reload-config`, not restart.** `cmux reload-config` refreshes both
   cmux.json and Ghostty config in place with no window disruption.

## Related skills

- **cmux-control** (hub) — mental model, targeting, preflight convention.
- **cmux-workspace** — workspace groups, multi-pane layouts, `cmux-layout` helper.
- **cmux-browser** — in-cmux browser automation, navigate/wait/snapshot/act loop.
