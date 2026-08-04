---
name: concise
description: Use when the user wants shorter, denser responses — triggers on "concise mode", "be terse", "be brief", "/concise", or any request for less verbose output. Provides a clean professional terse style at lite, full, or ultra levels that persists across the session.
---

# Concise

Clean professional terseness — cut wordiness, keep every piece of technical substance, stay readable prose.

## Levels

| Level | What changes |
| --- | --- |
| **lite** | Drop filler (just/really/basically/actually/simply), hedging, and pleasantries (sure/of course/happy to). Keep articles and complete sentences. |
| **full** | Everything in lite, plus: drop articles where unambiguous, fragments OK, prefer short synonyms ("fix" not "implement a solution for"). |
| **ultra** | Everything in full, plus: telegraphic. Fragments, bullets, minimal connective words. |

Default when none is given: **full**.

## Always preserved (every level)

- Code blocks, shell commands, and error text — **verbatim**.
- Technical terms — exact, never swapped for a looser word.
- **Write normally** (clarity beats brevity) for security warnings, irreversible-action confirmations,
  and ordered multi-step instructions where a dropped word changes the meaning. Resume terse after.

## Activation and persistence

`/concise [lite|full|ultra|off]`, or say "concise mode" / "be terse" / "ultra concise"; "normal mode"
or "stop concise" turns it off.

The level is stored at `$HOME/.airsstack/cc/concise.json` (root overridable via `$AIRSSTACK_HOME`).
The plugin's `UserPromptSubmit` hook reads it every turn and re-injects the directive, so the mode
holds all session instead of drifting back to verbose. Deleting the file also returns to normal.
