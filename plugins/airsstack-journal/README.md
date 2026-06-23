# airsstack-journal

A transparent, note-based experiential memory for the development agent: a
single Obsidian-compatible Markdown vault that the agent writes and the human
reads and edits. It pairs Zettelkasten note mechanics (atomic notes, stable
kebab identifiers, `[[wikilinks]]`) with the Building a Second Brain lifecycle
(Capture, Organize, Distill, Express), so the agent can record what it learns
and recall it later instead of re-deriving it.

This plugin is a member of the in-repository `airsstack` marketplace and
depends on the `airsstack` plugin.

## Phase 1 — Foundation (this release)

Phase 1 ships the storage substrate and provisioning machinery only. It writes
no notes and provides no recall, capture, or review behaviour — those arrive in
later phases.

- **Vault** at `${AIRSSTACK_HOME:-~/.airsstack}/journal/`, a single global,
  HOME-global store (sibling to the snapshot and SDD stores), partitioned by
  note *kind* into `daily/`, `sessions/`, `notes/`, `mocs/`, and a derived
  `.index/`. It is never partitioned by project.
- **Derived index** (`.index/graph.json`, `.index/tags.json`,
  `.index/summaries.tsv`) — a rebuildable cache the recall phase will consume.
  The Markdown corpus is the sole source of truth; the index is fully
  reconstructible from it.
- **`provision.sh`** (POSIX sh) creates the vault directories idempotently.
- **`build-index.py`** (python3, standard library only) scans the corpus and
  writes the derived index.
- **`session-start.sh`** runs on `SessionStart`: it provisions, then rebuilds
  the index only when stale or absent, failing open so a missing `python3` or a
  malformed note never blocks a session.
- **`/airsstack-journal:journal-setup`** explicitly provisions and force-rebuilds
  the index.

The note contract (frontmatter fields and the kebab filename/wikilink
convention) is documented in `references/note-schema.md`.

## Tests

```sh
sh plugins/airsstack-journal/scripts/provision.test.sh
sh plugins/airsstack-journal/scripts/session-start.test.sh
python3 plugins/airsstack-journal/scripts/build-index.test.py
```
