---
name: snapshot-save
description: Use when ending a work session, before /clear, or when the user says "save a snapshot" / "save to memory" / "snapshot save" — flushes durable session learnings into a project-local per-fact memory store, with a strict durability gate so thin sessions write nothing.
---

# Snapshot Save

Codifies the memory-save ceremony so it runs the same way every time. A "snapshot" is NOT a new
format — it is a file-per-fact memory store kept **outside the repo** in the user-global airsstack
home, namespaced per project, with a one-line index `MEMORY.md`. This skill standardizes the *act*
of saving.

## Memory store location

The store lives at:

```
${AIRSSTACK_HOME:-~/.airsstack}/memory/<project-key>/
├── MEMORY.md          # index — one line per memory, loaded for orientation
├── <slug-1>.md        # one fact per file
├── <slug-2>.md
└── ...
```

Why outside the repo: memory is per-user **local persistence** — it must survive worktree teardown,
branch churn, `target/` cleans, and `/clear`, and must never be accidentally committed. Keeping it in
`~/.airsstack` (same root the `concise` hook uses) gives one user-global state location and makes it
shared across every worktree of the same repo. It is intentionally NOT shareable via git.

### Resolving `<project-key>` (MANDATORY — stable across worktrees)

Compute it the same way every time so all worktrees of one repo map to one store:

1. Run `git rev-parse --git-common-dir` and resolve it to an absolute path. This resolves to the
   **main** repo's `.git` from every linked worktree → one key per repo, no per-worktree
   fragmentation.
2. `project-key` = `<repo-basename>-<hash8>` where:
   - `<repo-basename>` = basename of the common-dir's parent (the repo dir name), for greppability.
   - `<hash8>` = first 8 hex chars of a hash of the absolute common-dir path, for collision safety.
   - Example: `airsstack-3f9a2c1b`.
3. **No git repo** (command fails): fall back to hashing the absolute `cwd`, key
   `<cwd-basename>-<hash8>`.

Concretely:

```sh
common_dir=$(git rev-parse --git-common-dir 2>/dev/null) \
  && abs=$(cd "$(dirname "$common_dir")" && pwd)/$(basename "$common_dir") \
  || abs="$(pwd)"
base=$(basename "$(dirname "$abs")")
hash8=$(printf '%s' "$abs" | shasum | cut -c1-8)
key="${base}-${hash8}"
```

Created on first use: make `${AIRSSTACK_HOME:-~/.airsstack}/memory/<project-key>/` and an empty
`MEMORY.md`. No `.gitignore` ceremony — the store is outside the repo, so nothing can leak into a
commit.

## Per-fact file schema

Each memory is ONE file holding ONE fact:

```markdown
---
name: <short-kebab-case-slug>
description: <one-line summary — used to decide relevance during recall>
metadata:
  type: user | feedback | project | reference
---

<the fact. For feedback/project, follow with **Why:** and **How to apply:** lines.
Link related memories with [[their-slug]].>
```

The four types:

- `user` — who the user is (role, expertise, durable preferences).
- `feedback` — how you should work (corrections, confirmed approaches). Include **Why:** and
  **How to apply:** lines.
- `project` — ongoing work, goals, constraints NOT derivable from code/git. Convert relative dates
  to absolute. Include **Why:** and **How to apply:** lines.
- `reference` — pointers to external resources (URLs, dashboards, tickets).

Link related memories with `[[slug]]` (a not-yet-existing target is fine — it marks future work).

## Procedure

1. **Review the session** for facts worth persisting, classified by the four types above.

2. **Durability gate (MANDATORY).** If nothing in the session is durable — a thin session, only
   transient detail, or only things already recorded in the repo / git history / project
   instructions — write **nothing** and report "nothing durable to save." Do not invent memories
   to look productive. This gate is what keeps an auto-save hook quiet on thin sessions.

3. **Dedupe before writing.** For each durable fact, check `MEMORY.md` and the memory dir for an
   existing file that already covers it. **Update** that file instead of creating a duplicate.
   Delete any memory the session proved wrong.

4. **Write/update one file per fact** in the resolved store dir, following the schema above.

5. **Update the store's `MEMORY.md`** — add/adjust one line per new or changed memory:
   `- [Title](file.md) — hook`. Never put memory content in `MEMORY.md` body; it is index only.

6. **Report**: list files written / updated / deleted, or "nothing durable to save."

## Must NOT do

- Invent a new file format or a multi-section checkpoint file (snapshots are per-fact).
- Dump raw session transcript into a memory.
- Save what the repo already records (code structure, past fixes resolved in git, project docs).
- Write to `MEMORY.md` beyond its one-line index entries.
