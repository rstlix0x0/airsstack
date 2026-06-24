---
name: journal-note
description: >
  Manually capture one atomic, standalone topic note (insight|decision) into
  the airsstack-journal vault. Resolves session id, transcript path, and
  project floor, then spawns the isolated journal-note subagent to write/update
  notes/<kebab>.md, stamping the session join field. Update-in-place on a
  repeated topic. Use when the user says "save a note on X" / "/journal-note X".
---

# journal-note

Capture one atomic note about a specific topic. Do the cheap deterministic
resolution here, then hand the transcript read and note authoring to the
isolated `journal-note` subagent.

## Steps

1. Take the user's `topic` (the note subject, e.g. "auth rbac architecture").
   If the user gave no topic, ask for one before proceeding.

2. Resolve the session id: `session_id` = `${CLAUDE_SESSION_ID}`; `id8` = its
   first eight characters. (If empty, pass an empty `id8`; the note simply
   carries no `session:` join value.)

3. Resolve the transcript path:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/transcript-path.sh" "${CLAUDE_SESSION_ID}"
   ```

   Capture stdout as `tpath` (empty/non-zero ⇒ the agent degrades).

4. Resolve the project floor:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.sh"
   ```

   Capture stdout as `project`.

5. Provision the vault (idempotent):

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/provision.sh"
   ```

6. Spawn the `journal-note` subagent (Task / Agent tool,
   `subagent_type: journal-note`), passing `topic`, `session_id`, `id8`,
   `tpath`, and `project`. The agent derives the kebab stem (rejecting
   `_unresolved`), creates or updates `notes/<stem>.md`, links the daily note,
   refreshes the index, and returns a one-line receipt.

7. Relay the agent's receipt to the user. Do not write any note file yourself.

## If the agent is unavailable

If the `journal-note` subagent cannot be spawned, tell the user and stop. Do
nothing destructive.
