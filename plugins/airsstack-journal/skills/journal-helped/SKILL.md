---
name: journal-helped
description: >
  Confirm that a recalled journal note actually aided the work — increments its
  helped: counter and refreshes the index via bump-helped.sh. Deterministic, no
  subagent. Use AFTER a note returned by /journal-recall helped solve the task,
  or when the user says "that note helped" / "/journal-helped <stem>".
---

# journal-helped

Record that a note proved useful. This is the write-back half of recall: the
`helped` counter is a ranking signal, and it must reflect notes that actually
aided a solution — not mere retrieval.

## Steps

1. Take the `<stem>` of the note that helped (e.g. `tokio-cancellation-safety`).
   If no stem was given, ask which recalled note helped before proceeding.

2. Run the deterministic write-back:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/bump-helped.sh" "<stem>"
   ```

3. Relay the script's one-line receipt (e.g. `bumped helped to 3 in
   notes/<stem>.md`). If the script reports no such note, tell the user the
   stem did not resolve; do not create or edit any note yourself.

## Notes

- This skill writes nothing itself and spawns no subagent — it only invokes
  `bump-helped.sh`, which increments the counter and rebuilds the index.
