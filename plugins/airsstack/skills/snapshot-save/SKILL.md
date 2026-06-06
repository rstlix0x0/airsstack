---
name: snapshot-save
description: Use when ending a work session, before /clear, or when the user says "save a snapshot" / "save to memory" / "snapshot save" — flushes durable session learnings into a project-local per-fact memory store, with a strict durability gate so thin sessions write nothing.
---

# Snapshot Save

Codifies the memory-save ceremony so it runs the same way every time. A "snapshot" is NOT a new
format — it is a file-per-fact memory store kept in the project at `.claude/memory/`, with a
one-line index at `.claude/memory/MEMORY.md`. This skill standardizes the *act* of saving.

## Memory store layout

```
.claude/memory/
├── .gitignore         # ignores the store from within (memory is local scratch)
├── MEMORY.md          # index — one line per memory, loaded for orientation
├── <slug-1>.md        # one fact per file
├── <slug-2>.md
└── ...
```

Created on first use. If `.claude/memory/` does not exist yet, before writing the first memory:

1. Create the directory and an empty `MEMORY.md`.
2. Write `.claude/memory/.gitignore` with exactly:
   ```
   *
   !.gitignore
   ```
   This keeps the store **local** — memory is per-user scratch context, not committed source. The
   dir ignores itself, so the consumer's root `.gitignore` is never touched. A consumer who wants
   to share memory with their team deletes this `.gitignore`.

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

4. **Write/update one file per fact** in `.claude/memory/`, following the schema above.

5. **Update `.claude/memory/MEMORY.md`** — add/adjust one line per new or changed memory:
   `- [Title](file.md) — hook`. Never put memory content in `MEMORY.md` body; it is index only.

6. **Report**: list files written / updated / deleted, or "nothing durable to save."

## Must NOT do

- Invent a new file format or a multi-section checkpoint file (snapshots are per-fact).
- Dump raw session transcript into a memory.
- Save what the repo already records (code structure, past fixes resolved in git, project docs).
- Write to `MEMORY.md` beyond its one-line index entries.
