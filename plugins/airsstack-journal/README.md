# airsstack-journal

A transparent, note-based experiential memory for the development agent: a
single Obsidian-compatible Markdown vault that the agent writes and the human
reads and edits. It pairs Zettelkasten note mechanics (atomic notes, stable
kebab identifiers, `[[wikilinks]]`) with the Building a Second Brain lifecycle
(Capture, Organize, Distill, Express), so the agent can record what it learns
and recall it later instead of re-deriving it.

This plugin is a member of the in-repository `airsstack` marketplace and
depends on the `airsstack` plugin.

## Install

Every hook and skill script in this plugin runs on the `airsl` binary; install it first with
`cargo install --git https://github.com/rstlix0x0/airsstack --locked airsl-cli`, then:

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack        # dependency
/plugin install airsstack-journal@airsstack
```

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
- **`provision.lua`** creates the vault directories idempotently.
- **`build-index.lua`** scans the corpus and writes the derived index.
- **`session-start.sh`** is the launcher the `SessionStart` hook runs. It checks
  that `airsl` is installed and then runs `session-start.lua`, which provisions,
  rebuilds the index only when stale or absent, and injects the orientation card
  — failing open so a malformed note never blocks a session.
- **`/airsstack-journal:journal-setup`** explicitly provisions and force-rebuilds
  the index.

The note contract (frontmatter fields and the kebab filename/wikilink
convention) is documented in `references/note-schema.md`.

## Tests

```sh
airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts
```

Every script in this plugin runs on [`airsl`](../../crates/airsl), the embedded
Lua runtime, under `--policy confined`: the authority each one needs is named on
its own command line, so what a script may read, write, or run is visible in the
invocation rather than buried in the script.

## Phase 2 — Capture (manual storytelling subagents)

Phase 2 adds two **manual** capture surfaces. Each is a thin trigger skill that
resolves the session id, transcript path (`scripts/transcript-path.lua`), and
project floor on the main thread, then spawns an **isolated subagent** that
reads the session transcript and writes a single grounded, Obsidian-compatible
storytelling note. Only a one-line receipt returns to the main thread, so the
transcript read never costs the main-thread context. There is no automatic
capture and no SessionEnd hook — the user decides when to write.

- `/airsstack-journal:journal-capture` — write the **session story**
  (`sessions/session-<id8>.md`). Re-run anytime; each run **overwrites** the
  story from the transcript (pure function, no merge).
- `/airsstack-journal:journal-note <topic>` — write/update one **atomic note**
  (`notes/<kebab>.md`) on a topic. Re-running the same topic **updates it in
  place**; the reserved stem `_unresolved` is rejected. Notes carry a
  `session: <id8>` field so the session story can list them under
  "Notes spun off".

Both writers link their note into the day's daily note (`scripts/daily-link.lua`)
and refresh the derived index (`scripts/build-index.lua`). The vault layout and
`.index/` format are unchanged from Phase 1; typed-edge/backlink graph
enrichment is deferred to Phase 3 (Recall).

## Phase 3 — Recall (read the vault back)

Phase 3 lets the agent *read* prior notes instead of re-deriving them — the
payoff of the token-efficiency mandate. It is additive: the Phase-1/2 storage
contract is unchanged, and `build-index.lua` now also emits the enriched
`.index/index.json` (node metadata + structurally-typed edges + backlinks +
unresolved) consumed by recall.

- `/airsstack-journal:journal-recall <query>` — spawns the isolated
  `journal-recall` subagent, which reads ONLY the derived index (never note
  bodies) and returns a capped list of ranked pointers
  (`stem · summary · path · why`). Ranks by tag/domain match, summary/title
  text, the `helped` counter, and graph proximity. The main thread reads at
  most the one note it picks. The skill also auto-triggers before the agent
  re-derives something it may have noted.
- `/airsstack-journal:journal-link <stem-or-topic>` — the same subagent in
  `link` mode: suggests existing notes to `[[link]]` from a subject note
  (subject excluded), reusing the recall machinery.
- `/airsstack-journal:journal-helped <stem>` — confirms a recalled note aided
  the work, incrementing its `helped:` counter (deterministic, no subagent)
  via `scripts/bump-helped.lua` and refreshing the index.
- **SessionStart orientation card** — `scripts/orientation.lua` prints a tight,
  project-scoped recent-activity card (recent sessions + recently-updated
  notes) from `summaries.tsv`; `session-start.sh` injects it as
  `additionalContext`. No model, fail-open.

Typed `depends-on` / `supersedes` edges, MOC promotion, progressive
summarisation, and the daily narrative are deferred to Phase 4 (Review).

### Tests (Phase 3 additions)

Covered by the suite-wide run above — `notes_test.lua` and `orientation_test.lua`.

## Phase 4 — Review

`/journal-review` tidies the vault in one command. It runs deterministic,
model-free steps on the main thread — a vault backup (`journal-backup.lua`) and a
graph-health report (`graph-health.lua`) — then delegates judgment-bound tidying
to the isolated opus `journal-curator` subagent, which applies **additive-only**
changes: MOC index notes, a `## TL;DR` layer on long notes, a `## Narrative` on
daily notes, typed `depends-on`/`supersedes` edges, and high-confidence missing
links. The curator never deletes or overwrites existing prose, never touches
`sessions/`, and never commits; a backup precedes every run, so any run is
reversible (`tar xzf .backups/<ts>.tar.gz -C <vault>`). `build-index.lua` now
also resolves the typed frontmatter fields into typed `index.json` edges.

`/journal-review` reviews the current project; `/journal-review all` reviews the
whole vault.

Run the Phase 4 tests:

```sh
airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts
```
