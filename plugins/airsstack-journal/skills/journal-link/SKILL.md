---
name: journal-link
description: >
  Suggest existing journal notes to link from a note or topic, reusing the
  recall machinery. Given a stem, the main thread reads that ONE note to form
  the query; given free text, it uses that as the topic. Spawns the
  journal-recall subagent in mode=link (subject excluded) and returns link
  candidates (stem · summary · path · why). Use when the user says
  "suggest links for X" / "/journal-link X". The agent then edits the note to
  add the [[links]] it chooses.
---

# journal-link

Find existing notes worth linking from a subject note or topic. This is the
recall subagent pointed at a note: same index read, same pointer shape, ranked
for link-worthiness.

## Steps

1. Take the argument `<stem-or-topic>`.

   - If it resolves to an existing `notes/<stem>.md` under
     `${AIRSSTACK_HOME:-$HOME/.airsstack}/journal`, read THAT single note and
     form the `query` from its `title`, `summary`, `tags`, and `domains`; set
     `subject_stem` to its stem.
   - Otherwise treat the argument as a free-text `topic` query; leave
     `subject_stem` empty.

2. Resolve the project floor:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.sh"
   ```

   Capture stdout as `project`.

3. Spawn the `journal-recall` subagent (Task / Agent tool,
   `subagent_type: journal-recall`), passing `query`, `project`, `mode=link`,
   and `subject_stem`. The subagent excludes the subject and ranks plausible
   `[[link]]` targets.

4. Relay the candidates. If the user wants them applied, edit the subject note
   to add the chosen `[[wikilinks]]`; do not edit any other note.

## If the agent is unavailable

If the `journal-recall` subagent cannot be spawned, tell the user and stop. Do
nothing destructive.
