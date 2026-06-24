---
name: journal-review
description: >
  Tidy the airsstack-journal vault in one command. Backs the vault up, runs a
  deterministic graph-health report, then spawns the isolated opus
  journal-curator to apply additive-only tidying (MOCs, TL;DR layers, daily
  narrative, typed edges, missing links), and rebuilds the index. Use when the
  user says "review the journal" / "tidy the journal" / "/journal-review". Every
  change is additive and a backup precedes every run.
---

# journal-review

Orchestrate a vault review. You do the deterministic, model-free steps on the
main thread; the judgment-bound tidying is delegated to the isolated
`journal-curator` subagent. You hold no opinions about what to tidy — the
curator does.

`/journal-review` reviews the current project; `/journal-review all` reviews the
whole vault across projects.

## Steps

1. Resolve the project floor (unless the argument is `all`):

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.sh"
   ```

   Capture stdout as `scope` (or use `all` when that argument was given).

2. **Back up first — abort on failure (no review without a restore point):**

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/journal-backup.sh"
   ```

   Capture stdout as the backup archive path. If this exits non-zero, **abort
   the review** and surface the error — write nothing else.

3. Run the deterministic graph-health report and save it to a temp file to hand
   to the curator:

   ```sh
   python3 "${CLAUDE_PLUGIN_ROOT}/scripts/graph-health.py" > "${TMPDIR:-/tmp}/journal-health.md"
   ```

4. Open a Context Handoff session and compute the curator's write-path:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/../airsstack/scripts/handoff.sh" init
   ```

   Assign `NN=01`, `agent=journal-curator`, `slug=review`; the curator's
   handoff file is `<session-dir>/01-journal-curator-review.md`. Call
   `handoff.sh beat <session-dir>` on spawn.

5. Spawn the `journal-curator` subagent (Task / Agent tool,
   `subagent_type: journal-curator`), passing `scope`, the `vault` root, the
   `health_report` temp path, and the `handoff_path`. The curator applies its
   additive edits and returns a one-line summary plus its handoff path.

6. Rebuild the derived index so MOCs, typed edges, and new links take effect:

   ```sh
   python3 "${CLAUDE_PLUGIN_ROOT}/scripts/build-index.py" --force
   ```

   If `python3` is absent, report that the next SessionStart staleness check
   will rebuild — do not fail the review (the curator's edits persist).

7. Close the handoff session and report:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/../airsstack/scripts/handoff.sh" end <session-dir>
   ```

   Relay the curator's summary, then the one-line **restore** hint naming the
   backup from step 2: `restore with: tar xzf <archive> -C <vault>`. If the
   curator deferred missing-link suggestions, point the user at its handoff
   `<detail>` path.

## If the agent is unavailable

If the `journal-curator` subagent cannot be spawned, tell the user and stop. The
backup from step 2 is already safe; do nothing destructive.
