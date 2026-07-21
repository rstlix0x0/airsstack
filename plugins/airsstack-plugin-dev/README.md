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

This plugin installs a `PostToolUse` hook (on `Edit`, `Write`) that, when
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

## v2 — the cache delivery guard

The `PostToolUse` cache-sync hook mirrors files **as you edit them**. It has a
blind spot by construction: anything committed before the hook was installed,
anything arriving via a branch switch or a pull, and anything edited in a
session where the plugin was not active never reaches the install cache.

That blind spot shipped a real outage. `plugins/airsstack-guideline-rust/enforcement.json`
was added in `ced7091`; the cache-sync hook was installed afterwards; the file
therefore existed in the repo and in none of the seven install caches, and the
enforcement dispatcher — which discovers guideline plugins through exactly that
file — loaded zero manifests and did nothing on every edit for weeks.

`hooks/cache-guard.sh` → `hooks/cache_guard.py` closes it. On
`SessionStart(startup|resume|clear)` it:

- **Activates only in the main worktree** of a repo hosting the `airsstack`
  marketplace. `git rev-parse --show-toplevel` succeeds from every linked
  worktree, and several worktrees at different commits share one version-keyed
  cache — so without this gate the cache converges on a union of branches.
  Linked worktrees still report; they never write.
- **Backfills add-and-update only.** A source file missing or differing in the
  cache is copied. A cache-only file is reported and **never deleted**: an
  unreferenced leftover is inert, while an unattended hook holding delete
  authority over Claude Code's data directory is the larger risk. `.in_use`,
  `.DS_Store` and `.git` are ignored on both sides. Every write is bounded to
  the install cache root.
- **Reports version drift by comparing version values.** The last *bump* is the
  newest commit where `plugin.json`'s `version` differs from its parent's; any
  plugin content committed after it is stale. Comparing commit boundaries
  instead misses a manifest edited without a version change and misses a squash
  merge that collapses bump and content into one commit — and this repo's
  history is entirely squash merges. Staleness is reported as a single
  aggregate count, not one line per plugin: content newer than the last bump is
  the normal state of a plugin under development (5 of 7 here), so per-plugin
  lines would print a wall on every session start and train you to skim past
  the whole report.
- **Names what it cannot fix.** Backfilled `hooks.json` and `commands/` take
  effect only at plugin-load time, so the report tells you to restart the
  session; and user-scope installs pull the marketplace from GitHub, so a
  version bump reaches them only once the commit is pushed to `main`.

It is fail-open throughout — a broken module, a missing `python3`, garbage on
stdin or an unexpected exception all exit 0 without disturbing the session.

### Why the mirror hook does not trigger on `Read`

The enforcement dispatcher in the `airsstack` plugin matches `Read|Edit|Write`
so a guideline lands before the design decision, not at the moment of writing.
This mirror hook deliberately does **not** follow it. Reads are incidental —
every exploration sweep, every grep-then-read — so triggering on `Read` would
let merely *looking* at a plugin file from a linked worktree push that branch's
bytes into the shared version-keyed cache: exactly the cross-branch convergence
the guard's main-worktree gate exists to prevent.

## Roadmap

Two more buckets are planned as their own sequenced specs:

- **Validators** — namespace-prefix guard, skill/agent frontmatter schema, marketplace
  source paths, settings enabled keys, SKILL gate quote-consistency.
- **Generators** — scaffold a new plugin / skill / agent / hook.
