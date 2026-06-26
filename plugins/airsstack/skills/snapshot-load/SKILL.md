---
name: snapshot-load
description: Use at the start of work on a branch, after /clear, or when the user says "load snapshot" / "load from memory" / "where was I" — reads the project-local snapshot index and fully reads the relevant snapshots, then reports the rehydrated state. No-arg loads the current branch's latest; an explicit topic argument switches to a branch-agnostic topic search across all snapshots (e.g. pull a snapshot from another session/branch by keyword).
---

# Snapshot Load

Codifies the snapshot-load ceremony — the standardized "load the snapshot relevant to what I'm
working on now." Operates the timestamped snapshot store written by `snapshot-save` (see that skill
for the layout, filename, and schema). Pulls the relevant snapshots into context and reports where
work stands.

This reads the **airsstack** snapshot store only — **not** Claude's native memory tool. The native
store has size limits we are working around; the airsstack store lives outside the repo at the path
below.

## Snapshot store

The store lives **outside the repo**, namespaced per project, at
`${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/`. `index.md` there is the index (one line per
snapshot — NOT Claude's `MEMORY.md`); the timestamped snapshot files live alongside it. See
`snapshot-save` for the layout and the exact `<project-key>` resolution — compute it the **same way**
(from `git rev-parse --git-common-dir`, so every worktree of one repo loads the same store). If the
store dir does not exist yet, there is nothing to load — report that and stop.

## Argument

`/snapshot-load [topic]` — optional free-text topic. The argument also switches
the selection **mode**:

- **No topic (default):** current-branch orientation. Load the most recent
  snapshot(s) for the **current branch** — the "where was I on this branch" case.
- **Explicit topic:** **branch-agnostic topic search.** Rank **all** snapshots in
  the store by how well their `summary` matches the topic, regardless of branch;
  branch only breaks ties (prefer current branch among comparable matches). This
  is the cross-session case — pull a snapshot from another session/branch about a
  topic into the current one (e.g. `/snapshot-load streaming parser`).

## Procedure

1. **Resolve the store dir** (see `snapshot-save` for the `<project-key>` rule) and **read its
   `index.md`** (the index). If absent → report "no snapshot store yet" and stop.

2. **Determine the current branch:** run `git branch --show-current`.

3. **Select relevant snapshots** from the index lines, by mode. Newer snapshots carry a `topic`
   slot: `date · branch · topic · summary · file` (`-` when saved with no topic). Older snapshots
   predate that slot and have the 4-field form `date · branch · summary · file` — tolerate both;
   when there is no `topic` slot, fall back to matching the `summary`.

   - **No topic given:** prefer snapshots matching the **current branch**, newest first. Pick the
     most recent relevant snapshot(s) — usually the latest 1–2 for the branch. Do NOT select
     snapshots from other branches.
   - **Topic given:** rank **all** snapshots **branch-agnostic**. Match the loader topic against the
     snapshot's `topic` slot **first** (the explicit key the saver stamped), then fall back to its
     `summary` for snapshots that have no topic. A strong topic match on another branch outranks an
     unrelated current-branch snapshot; use the current branch only to break ties between comparable
     matches. Pick the best 1–2 matches; if nothing meaningfully matches the topic, say so rather
     than falling back to unrelated snapshots.

4. **Fully read** the selected snapshot files (the index gives only one-liners; this step pulls full
   content into context).

5. **Report a concise rehydrated state:** where the work stands, open carryovers/concerns, and the
   next step. Keep it tight — this is orientation, not a transcript.

## Must NOT do

- Read Claude's native memory store. Only this airsstack store.
- Read every snapshot file unconditionally (token waste). Select by branch/topic + recency.
- In **no-topic** mode, load snapshots from other branches (stay on the current branch).
- In **topic** mode, ignore a strong cross-branch topic match just because it is off the current
  branch — topic mode is deliberately branch-agnostic.
- Either mode: load snapshots that match neither the branch nor the topic, unless the user
  explicitly asks for all.
