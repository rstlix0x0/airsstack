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

**Always back up before editing.** Use the `cmux-settings` helper to do this automatically:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-config/scripts/cmux-settings backup-then <cmd>
```

The helper backs up `~/.config/cmux/cmux.json` to a timestamped `.bak` copy, runs
`<cmd>`, then validates with `cmux config validate`. If validation fails it restores
the backup and exits nonzero — the config is never left broken.

Example — append a custom command via `jq`:

```sh
${CLAUDE_PLUGIN_ROOT}/skills/cmux-config/scripts/cmux-settings backup-then \
  sh -c 'jq ".commands += [{\"name\":\"build\",\"command\":\"npm run build\"}]" \
    ~/.config/cmux/cmux.json > /tmp/cmux.json.new && \
    mv /tmp/cmux.json.new ~/.config/cmux/cmux.json'
```

Read-only commands (`cmux settings path`, `cmux config path`, `cmux config doctor`) inspect the
current config without modifying it; no backup is needed before running them.

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

1. **Back up before every edit.** Use `cmux-settings backup-then <cmd>` for any mutating
   command; never edit cmux.json in-place without a `.bak` copy.
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
