---
name: snapshot-save
description: Use when ending a work session, before /clear, or when the user says "save a snapshot" / "save to memory" / "snapshot save" — flushes durable session learnings into the project's existing per-fact memory files using the established schema, with a strict durability gate so thin sessions write nothing.
---

# Snapshot Save

Codifies the project's memory-save ceremony. A "snapshot" is NOT a new format — it is the
existing file-per-fact memory system (schema, directory, and `MEMORY.md` index described in
your memory-system instructions). This skill standardizes the *act* of saving so it runs the
same way every time.

## Procedure

1. **Review the session** for facts worth persisting, classified by the existing schema:
   - `user` — who the author is (role, expertise, durable preferences)
   - `feedback` — how you should work (corrections, confirmed approaches); include **Why:** and **How to apply:**
   - `project` — ongoing work, goals, constraints NOT derivable from code/git; convert relative dates to absolute
   - `reference` — pointers to external resources (URLs, dashboards, tickets)

2. **Durability gate (MANDATORY).** If nothing in the session is durable — a thin session,
   only transient detail, or only things already recorded in the repo / git history /
   `CLAUDE.md` — write **nothing** and report "nothing durable to save." Do not invent
   memories to look productive. This gate is what keeps the auto-save hook quiet on thin
   sessions.

3. **Dedupe before writing.** For each durable fact, check `MEMORY.md` and the memory dir for
   an existing file that already covers it. **Update** that file instead of creating a
   duplicate. Delete any memory the session proved wrong.

4. **Write/update one file per fact** in the project memory directory (path is in your
   memory-system instructions — do not hardcode it). Each file:
   - frontmatter: `name` (short kebab-case slug), `description` (one-line, used for recall),
     `metadata.type` (one of the four above)
   - body: the fact. For `feedback`/`project`, follow with **Why:** and **How to apply:** lines.
   - link related memories with `[[name]]` (a not-yet-existing target is fine — it marks
     future work).

5. **Update `MEMORY.md`** — add/adjust one line per new or changed memory:
   `- [Title](file.md) — hook`. Never put memory content in `MEMORY.md` body; it is index only.

6. **Report**: list files written / updated / deleted, or "nothing durable to save."

## Must NOT do

- Invent a new file format or a multi-section checkpoint file (snapshots are per-fact).
- Dump raw session transcript into a memory.
- Save what the repo already records (code structure, past fixes resolved in git, CLAUDE.md content).
- Write to `MEMORY.md` beyond its one-line index entries.
- Hardcode the absolute memory path — read it from your memory-system instructions.
