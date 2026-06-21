# Context Handoff

How delegated agents report to the orchestrator through the filesystem instead of
returning everything inline. An agent is an employee filing a report to its manager (the
main thread): a cheap **summary** goes up; the heavy **detail** stays on disk and is
pulled only by whoever must operate on it. This keeps detail out of the expensive,
long-lived main-thread context unless the main thread itself must reason over it.

The shell side — session lifecycle, the liveness lease, retention — is owned by
`plugins/airsstack/scripts/handoff.sh`. This prose and that script MUST agree on the
path and rules; change one, change the other.

## Path layout

Project-local, git-ignored, rooted at the **worktree root**:

```
<worktree-root>/.airsstack/cc/plugins/airsstack/handoff/<session-id>/<NN>-<agent>-<slug>.md
```

- `<session-id>` = `<YYYYMMDD-HHMMSS>-<rand4>`, minted by `handoff.sh init`.
- `<NN>` = zero-padded spawn sequence within the session, assigned by the orchestrator.
- `<agent>` = agent role; `<slug>` = short kebab task slug, assigned by the orchestrator.

Rooting at the worktree root isolates parallel worktrees — each gets its own physical
handoff tree, no shared state, no cross-worktree prune race. (This differs from the
snapshot store, which uses the common-dir to *share* memory; handoff is ephemeral.)

## File schema

```markdown
---
agent: reviewer
session: 20260621-153012-a1b2
seq: 03
task: <one-line task description>
created: 2026-06-21 15:31:40
---
<summary>
Returned to the main thread. The verdict / index — cheap, scannable. Always present.
</summary>
<detail>
Stays on disk. Full findings, file:line tables, rationale — whatever a downstream agent
or the main thread would need to operate on this work. Omitted when the report is thin.
</detail>
```

`<summary>` is always written; `<detail>` is gated — omit it when the summary already
says everything.

## Return & routing contract

- **Main → subagent (on spawn):** the orchestrator assigns `<NN>`/`<agent>`/`<slug>` and
  passes the **full handoff write-path** in the brief. The agent does not compute it.
- **Subagent → main:** the agent returns its `<summary>` text **plus the relative
  handoff path**. It does NOT return `<detail>`.
- **Main → downstream agent** needing prior detail: the orchestrator passes the upstream
  `handoff:` path **and** a targeted `need:` pointer; the downstream agent reads that
  file itself and pulls only the pointed-at slice into its own context.

The orchestrator stays the sole router: an agent reads another agent's handoff only
because the orchestrator handed it the path. The flat/leaf topology and the user commit
gate are unchanged.

## Report-write mechanism

Every spawn writes exactly one handoff file — its own, never another agent's, never
source. `coder` writes with its `Write` tool. The read-only agents (`explorer`,
`reviewer`, `verifier`) carry `Write` **scoped by instruction** to the handoff directory
only — writing the report is a first-class duty, distinct from mutating source, which
they still must never do.

## Session lifecycle (via handoff.sh)

- `handoff.sh init` — mint a session, write its `.active` lease, prune old sessions,
  print the session dir + id. The orchestrator runs this once at pipeline start.
- `handoff.sh beat <session-dir>` — refresh the `.active` lease; the orchestrator calls
  it on each spawn as a heartbeat.
- `handoff.sh end <session-dir>` — drop the lease at clean session close.

Retention: keep the last `AIRSSTACK_HANDOFF_KEEP` (default 10) session dirs. A dir beyond
that is pruned only once its `.active` lease is absent or older than
`AIRSSTACK_HANDOFF_GRACE` minutes (default 120). So an active (heartbeating) session is
never pruned, and a crashed one self-heals after the grace window. Pruning runs only at
`init`, never mid-run.

## Error handling

- Report write fails → the agent returns its full receipt inline and notes the failure;
  the task is not hard-failed.
- Downstream read fails (path missing) → the agent reports `handoff not found: <path>`;
  the orchestrator re-supplies inline or re-routes.
- No handoff path in the brief (agent run standalone) → the agent returns its receipt
  inline, exactly as without this protocol. Backward-compatible.
