---
name: snapshot-save
description: Use when ending a work session, before /clear, or when the user says "save a snapshot" / "save to memory" / "snapshot save" — captures a conversation snapshot (session summary + key snippets) into the project-local snapshot store, with a light durability gate so thin sessions write nothing. No-arg captures the whole session; an explicit topic argument focuses the capture on that topic and stamps it as a match key for topic-load.
---

# Snapshot Save

Codifies the snapshot-save ceremony so it runs the same way every time. A **snapshot** is a short
capture of the current conversation — a curated session summary plus a few key snippets — written as
one timestamped file in the project-local snapshot store **outside the repo**. This is the
airsstack memory; it is **deliberately separate from Claude's native memory tool**, whose store has
size limits we are working around.

## Argument

`/snapshot-save [topic]` — optional free-text topic. The argument switches the
capture **mode** (mirrors `snapshot-load`):

- **No topic (default):** whole-session orientation. Capture what a future reader
  needs to resume the session as a whole. Leave `topic:` empty in the schema.
- **Explicit topic:** **topic-focused capture.** Bias the summary, key snippets,
  and carryovers toward that topic (what was decided/done/pending *about it*),
  and stamp the topic as an explicit `topic:` key in frontmatter and the index
  line. This is the half that makes branch-agnostic topic-load resolve cleanly:
  the saver tags the focus, the loader matches it. Other threads from the session
  may be summarized in one line, but the snapshot is *about* the topic.

The topic only labels and focuses the snapshot; it never changes the store
location or filename (still `<date>-<time>-<branch>.md`).

## Snapshot store location

The store lives at:

```
${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/
├── index.md                          # custom index — one line per snapshot (NOT Claude's MEMORY.md)
├── 2026-06-15-143012-<branch>.md     # one snapshot per save, timestamped
├── 2026-06-14-090530-<branch>.md
└── ...
```

Why outside the repo: snapshots are per-user **local persistence** — they must survive worktree
teardown, branch churn, `target/` cleans, and `/clear`, and must never be accidentally committed.
Keeping them in `~/.airsstack` (same root the `concise` hook uses) gives one user-global state
location, shared across every worktree of the same repo. Intentionally NOT shareable via git.

The index file is named `index.md` **on purpose** — never name it `MEMORY.md`. `MEMORY.md` is the
filename Claude's native memory tool drives; reusing it would collide with native memory and defeat
the point of this store.

### Resolving `<project-key>` (MANDATORY — stable across worktrees)

Compute it the same way every time so all worktrees of one repo map to one store:

1. Run `git rev-parse --git-common-dir` and resolve it to an absolute path with **`pwd -P`**
   (physical canonicalization). This resolves to the **main** repo's `.git` from every linked
   worktree → one key per repo, no per-worktree fragmentation. `pwd -P` is load-bearing: on
   symlinked paths (e.g. macOS `/var` → `/private/var`) a linked worktree's common-dir comes back
   already resolved while the main worktree's stays logical — without `-P` the two hash differently
   and fragment the store.
2. `project-key` = `<repo-basename>-<hash8>` where:
   - `<repo-basename>` = basename of the common-dir's parent (the repo dir name), for greppability,
     **sanitized** to `[A-Za-z0-9._-]` (any other byte → `-`) so the key is filesystem-safe.
   - `<hash8>` = first 8 hex chars of a hash of the absolute common-dir path, for collision safety.
     Computed from the full (unsanitized) path, so it keeps keys unique even if sanitization
     collapses distinct names.
   - Example: `airsstack-3f9a2c1b`.
3. **No git repo** (command fails): fall back to hashing the absolute `cwd` (also `pwd -P`), key
   `<cwd-basename>-<hash8>`.

Concretely (byte-identical to `airsstack-sdd/hooks/ensure-layout.sh` — the three stores share one
key, so keep these in sync):

```sh
if common_dir=$(git rev-parse --git-common-dir 2>/dev/null); then
  abs=$(cd "$(dirname "$common_dir")" 2>/dev/null && pwd -P)/$(basename "$common_dir")
  base=$(basename "$(dirname "$abs")")
else
  abs=$(pwd -P)
  base=$(basename "$abs")
fi
base=$(printf '%s' "$base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
hash8=$(printf '%s' "$abs" | shasum | cut -c1-8)
key="${base}-${hash8}"
```

Created on first use: make `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` and an empty
`index.md`. No `.gitignore` ceremony — the store is outside the repo, so nothing can leak into a
commit.

## Snapshot filename

`<YYYY-MM-DD>-<HHMMSS>-<branch>.md`, e.g. `2026-06-15-143012-feature-memory.md`.

- Date/time from `date +%Y-%m-%d-%H%M%S` (local). Seconds included to avoid same-minute collisions.
- `<branch>` from `git branch --show-current`; sanitize `/` → `-` (e.g. `feature/x` → `feature-x`).
  No branch (detached / no git) → use `nogit`.

## Snapshot file schema

```markdown
---
date: 2026-06-15 14:30:12
branch: <branch>
project-key: <project-key>
topic: <the topic arg, verbatim — omit or leave empty when no topic was given>
summary: <one-line summary — what this session (or, in topic mode, the topic) was about>
---

# Snapshot — <date> — <branch>

## Session summary
<Curated prose: what was done, decisions made, current state, and the next step.
Tight — orientation, not a transcript.>

## Key snippets
<A few quoted excerpts pulled from the session that matter for resuming: a decision,
a command, a code fragment, an error message. Quote verbatim where fidelity matters.
Omit this section if nothing is worth quoting.>

## Open carryovers
<Unfinished work, concerns, the immediate next step. Omit if none.>
```

## Procedure

1. **Review the session** for what a future reader needs to resume: decisions, current state, key
   commands/snippets, open carryovers. **If a topic arg was given,** scope this review to that
   topic — pull the decisions/state/snippets/carryovers *about it*, and let it drive the `summary`
   and `topic:` fields. With no topic, capture the session as a whole.

2. **Durability gate.** If nothing meaningful happened — a thin session, only transient chatter, or
   only things already recorded in the repo / git history / project instructions — write **nothing**
   and report "nothing durable to save." Do not invent a snapshot to look productive. This keeps an
   auto-save hook quiet on thin sessions.

3. **Resolve the store dir** (the `<project-key>` rule above). Create it and an empty `index.md` if
   absent.

4. **Write one snapshot file** with the timestamped filename and the schema above.

5. **Update `index.md`** — append one line:
   `- <date> · <branch> · <topic> · <summary> · [file](<filename>.md)`. The `<topic>` slot carries
   the topic arg, or `-` when none was given (keeps the column stable for the loader). Index is one
   line per snapshot, never snapshot bodies.

6. **Report**: the snapshot file written, or "nothing durable to save."

## Must NOT do

- Write to Claude's native memory store (`~/.claude/projects/.../memory/` or any `MEMORY.md`). This
  skill owns the airsstack store only.
- Dump the raw session transcript — capture a curated summary + key snippets, not everything.
- Save what the repo already records (code structure, fixes already in git, project docs).
- Put snapshot content in `index.md` beyond its one-line entries.
