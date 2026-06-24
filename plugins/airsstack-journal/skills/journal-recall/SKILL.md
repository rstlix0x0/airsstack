---
name: journal-recall
description: >
  Recall prior journal notes as cheap ranked pointers instead of re-deriving
  them. Resolves the project floor, then spawns the isolated journal-recall
  subagent (mode=recall) to read ONLY the derived index and return pointers
  (stem · summary · path · why). Use BEFORE re-deriving something you may have
  noted before, or when the user says "recall X" / "/journal-recall X". The
  main thread reads at most the one note it picks.
---

# journal-recall

Recall what the vault already knows about a query, cheaply. Do the deterministic
project resolution here, then hand the index read and ranking to the isolated
`journal-recall` subagent. You never read a note body until — and unless — you
pick one pointer afterward.

## Steps

1. Take the user's `query` (the recall subject, e.g. "tokio cancellation
   safety"). If the user gave no query, ask for one before proceeding.

2. Resolve the project floor:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.sh"
   ```

   Capture stdout as `project`.

3. Spawn the `journal-recall` subagent (Task / Agent tool,
   `subagent_type: journal-recall`), passing `query`, `project`, and
   `mode=recall`. The subagent reads only the derived index and returns a
   capped ranked pointer list.

4. Relay the subagent's pointer list to the user. If a pointer is worth acting
   on, read THAT note's `path` (the first and only note-body read), and — if it
   aided the work — confirm it with `/airsstack-journal:journal-helped <stem>`.

## If the agent is unavailable

If the `journal-recall` subagent cannot be spawned, tell the user and stop. Do
nothing destructive; never fabricate a recall result.
