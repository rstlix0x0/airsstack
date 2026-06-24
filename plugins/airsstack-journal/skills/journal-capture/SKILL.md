---
name: journal-capture
description: >
  Manually capture the current session as a grounded storytelling note in the
  airsstack-journal vault. Resolves the session id, transcript path, and
  project floor, then spawns the isolated journal-capture subagent to write
  sessions/session-<id8>.md by full overwrite. Re-run anytime to refresh the
  story. Use when the user says "capture this session" / "/journal-capture".
---

# journal-capture

Capture the current session as a session story. You do the cheap deterministic
resolution here on the main thread, then hand the heavy transcript read and
storytelling to the isolated `journal-capture` subagent so it never costs the
main-thread context.

## Steps

1. Resolve the session id and stem. The session id is `${CLAUDE_SESSION_ID}`.
   Compute `id8` = its first eight characters and `stem` = `session-<id8>`.
   If `${CLAUDE_SESSION_ID}` is empty, fall back to `stem` =
   `session-$(date +%Y%m%d-%H%M%S)` and note that the story cannot be keyed to
   the session id.

2. Resolve the transcript path:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/transcript-path.sh" "${CLAUDE_SESSION_ID}"
   ```

   Capture stdout as `tpath`. An empty result (non-zero exit) means no
   transcript was found — pass an empty `tpath` and let the agent degrade.

3. Resolve the project floor:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.sh"
   ```

   Capture stdout as `project`.

4. Provision the vault (idempotent, cheap):

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/provision.sh"
   ```

5. Spawn the `journal-capture` subagent (Task / Agent tool,
   `subagent_type: journal-capture`), passing `session_id`, `id8`, `stem`,
   `tpath`, and `project`. The agent reads the transcript, overwrites
   `sessions/<stem>.md`, links the daily note, refreshes the index, and
   returns a one-line receipt.

6. Relay the agent's receipt to the user. Do not write any note file yourself.

## If the agent is unavailable

If the `journal-capture` subagent cannot be spawned (the plugin's agents are
not installed), tell the user and stop. Do nothing destructive.
