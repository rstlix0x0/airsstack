---
name: snapshot-load
description: Use at the start of work on a branch, after /clear, or when the user says "load snapshot" / "load from memory" / "where was I" — reads the project-local snapshot index and fully reads the snapshots relevant to the current git branch (and an optional topic argument), then reports the rehydrated state.
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

`/snapshot-load [topic]` — optional free-text topic to narrow selection
(e.g. `/snapshot-load streaming parser`).

## Procedure

1. **Resolve the store dir** (see `snapshot-save` for the `<project-key>` rule) and **read its
   `index.md`** (the index). If absent → report "no snapshot store yet" and stop.

2. **Determine the current branch:** run `git branch --show-current`.

3. **Select relevant snapshots.** From the index lines (`date · branch · summary · file`), prefer
   snapshots matching the current branch, newest first; also weigh the optional `topic` arg against
   each `summary`. Pick the most recent relevant snapshot(s) — usually the latest 1–2 for the
   branch. Do NOT select snapshots unrelated to the current branch/topic.

4. **Fully read** the selected snapshot files (the index gives only one-liners; this step pulls full
   content into context).

5. **Report a concise rehydrated state:** where the work stands, open carryovers/concerns, and the
   next step. Keep it tight — this is orientation, not a transcript.

## Must NOT do

- Read Claude's native memory store. Only this airsstack store.
- Read every snapshot file unconditionally (token waste). Select by branch/topic + recency.
- Load snapshots unrelated to the current branch/topic unless the user explicitly asks for all.
