---
description: Explain why the enforcement dispatcher does or does not fire for a given file path
---

Diagnose the airsstack rule-enforcement dispatcher for the path `$ARGUMENTS`.

Run exactly this, from the current working directory:

```
python3 "${CLAUDE_PLUGIN_ROOT}/hooks/enforce.py" --explain "$ARGUMENTS"
```

If `$ARGUMENTS` is empty, ask which file path to diagnose and stop; do not guess one.

Then read the trace back to the user, in this order:

1. **The outcome.** `outcome: 0 pointer(s)` means the dispatcher would stay silent
   for this path; anything higher means it would inject that many guideline pointers.
2. **The stage that ended it.** The framework has several paths that all end in
   silence and are indistinguishable from outside, which is how a delivery bug
   survived for weeks. The trace names exactly one:
   - `no @airsstack plugins in the registry` — nothing installed from this marketplace.
   - `zero manifests loaded` — plugins are installed but none carries a readable
     `enforcement.json`. This is the delivery failure; check the `parity:` section.
   - `GATE 1` — the plugin's registry record is bound to a different project.
   - `GATE 2` — no `detect` marker sits at or above the file's directory.
   - `GATE 3` — no `match` glob hit the file's repo-relative path.
   - `already claimed` — the pointer already fired once for this stack, phase,
     session and agent context. Expected, not a fault.
   A line reading `using <installPath>` names the registry record the doctor
   read for that plugin — the first thing to check when repo and cache
   diverge, since a project may have more than one record for the same key.
3. **The parity section**, when present. It appears only inside the plugin source
   repo. `MISSING from cache` means the dispatcher cannot see that file at all,
   because it runs from the install cache and not from the repo. When you see
   `MISSING from cache` for **every** file under one plugin, its cache directory
   does not exist, or exists but is empty — both produce identical output, and
   either way it is the most complete form of the delivery failure. A partial
   list of `MISSING`/`DIFFERS` lines mixed with files that are NOT reported
   means the cache dir exists and has some content but is stale or incomplete.
   A line reading `source tree unreadable, parity unknown` means the doctor
   could not even read the plugin's source directory to compare it —
   permissions, not a delivery gap; fix the source directory's permissions and
   re-run.

Do not paste the whole trace unless the user asks for it. Report the outcome, the
deciding stage, and — when the cause is fixable — the one action that fixes it.

Two things about the trace that are easy to misread:

- **`sentinels held: ...` names stacks and phases, not who claimed them.** A
  second `--explain` of the same path in the same session reports `already
  claimed` for a sentinel the doctor's own first run set, not one a real hook
  set — the doctor runs under its own dedicated session/agent identity, so
  running it twice in a row on the same path is expected to look "used up" the
  second time. That is the doctor observing itself, not the framework.
- **A missing `parity:` section is not a clean bill of health.** It only
  appears when invoked from inside the plugin source repo (a `plugins/`
  directory must exist at the git toplevel). Outside the source repo, the
  trace simply has no parity section at all — that is not the same claim as
  `parity: repo and cache agree`, which only appears when the check ran and
  found nothing wrong.
