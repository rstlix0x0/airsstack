---
name: snapshot-load
description: Use at the start of work on a branch, after /clear, or when the user says "load snapshot" / "load from memory" / "where was I" — reads the memory index and fully reads the memory files relevant to the current git branch (and an optional topic argument), then reports the rehydrated state.
---

# Snapshot Load

Codifies the project's memory-load ceremony — the standardized form of the prompt the author
otherwise hand-types ("load memory relevant to the current branch"). Operates the existing
file-per-fact memory system; it does not replace the harness's automatic injection of
`MEMORY.md` at session start, it builds on top of it.

## Argument

`/snapshot-load [topic]` — optional free-text topic to narrow selection
(e.g. `/snapshot-load phase-6 streaming`).

## Procedure

1. **Read `MEMORY.md`** (the index) from the project memory directory (path is in your
   memory-system instructions — do not hardcode it). The harness already injects this at
   session start; this is a confirm/refresh.

2. **Determine the current branch:** run `git branch --show-current`.

3. **Select relevant files.** Judge relevance from each memory's one-line `description:` in
   the index, against (current branch) + (the optional `topic` arg if given). Prefer the
   latest `*delivered*` / `*status*` project memories and their `[[linked]]` neighbors. Do
   NOT select memories unrelated to the current branch/topic.

4. **Fully read** the selected files (the index gives only one-liners; this step pulls full
   content into context).

5. **Report a concise rehydrated state:** where the work stands, open carryovers/concerns,
   and the next step. Keep it tight — this is orientation, not a transcript.

## Must NOT do

- Read every memory file unconditionally (token waste). Select by branch/topic.
- Load memories unrelated to the current branch/topic unless the user explicitly asks for all.
- Hardcode the absolute memory path — read it from your memory-system instructions.
