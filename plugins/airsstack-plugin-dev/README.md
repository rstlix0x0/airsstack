# airsstack-plugin-dev

Plugin-development toolkit for the airsstack suite. This is the workshop the other
four plugins are built in.

## v1 — cache-sync

The suite is developed in-tree under `plugins/<plugin>/` but executed from a
per-version cache at `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/`.
All plugins are pinned at `0.1.0` (in-development; no version bumps), so
`/plugin install` short-circuits at the same version and never re-copies the cache —
edits to a plugin file would otherwise require a manual `cp` or an uninstall/reinstall
dance.

This plugin installs a `PostToolUse` hook (on `Edit`, `Write`, `MultiEdit`) that, when
you edit a file under `plugins/<plugin>/`, mirrors just that file into the matching
install cache. The destination is read from
`~/.claude/plugins/installed_plugins.json` (so the version is never hardcoded), gated
to the `airsstack` marketplace, and containment-guarded to the cache root. The hook is
fail-silent and always exits 0 — it never blocks the edit.

### What it does and does not refresh

Claude reads skill **SKILL.md bodies at skill-run time**, so a body you edit goes live
**mid-session with no restart**. **Structural config** — `hooks.json`, agent frontmatter,
newly added skills/agents/commands — is read at **startup**; the hook places the bytes
correctly, but those changes still need a session restart to take effect.

Set `AIRSSTACK_PLUGIN_DEV_DEBUG=1` to emit a one-line stderr trace of each sync or no-op.

## Roadmap

Two more buckets are planned as their own sequenced specs:

- **Validators** — namespace-prefix guard, skill/agent frontmatter schema, marketplace
  source paths, settings enabled keys, SKILL gate quote-consistency.
- **Generators** — scaffold a new plugin / skill / agent / hook.
