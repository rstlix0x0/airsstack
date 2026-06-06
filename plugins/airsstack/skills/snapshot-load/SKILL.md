---
name: snapshot-load
description: Use at the start of work on a branch, after /clear, or when the user says "load snapshot" / "load from memory" / "where was I" — reads the project-local memory index and fully reads the memory files relevant to the current git branch (and an optional topic argument), then reports the rehydrated state.
---

# Snapshot Load

Codifies the memory-load ceremony — the standardized form of "load the memory relevant to what I'm
working on now." Operates the file-per-fact memory store written by `snapshot-save` (see that skill
for the layout and schema). Pulls the relevant facts into context and reports where work stands.

## Memory store

`.claude/memory/MEMORY.md` is the index (one line per memory). The per-fact files live alongside it
in `.claude/memory/`. If `.claude/memory/` does not exist yet, there is nothing to load — report
that and stop.

## Argument

`/snapshot-load [topic]` — optional free-text topic to narrow selection
(e.g. `/snapshot-load streaming parser`).

## Procedure

1. **Read `.claude/memory/MEMORY.md`** (the index). If absent → report "no memory store yet" and stop.

2. **Determine the current branch:** run `git branch --show-current`.

3. **Select relevant files.** Judge relevance from each memory's one-line `description:` in the
   index, against (current branch) + (the optional `topic` arg if given). Prefer the latest
   `*delivered*` / `*status*` project memories and their `[[linked]]` neighbors. Do NOT select
   memories unrelated to the current branch/topic.

4. **Fully read** the selected files (the index gives only one-liners; this step pulls full content
   into context).

5. **Report a concise rehydrated state:** where the work stands, open carryovers/concerns, and the
   next step. Keep it tight — this is orientation, not a transcript.

## Must NOT do

- Read every memory file unconditionally (token waste). Select by branch/topic.
- Load memories unrelated to the current branch/topic unless the user explicitly asks for all.
