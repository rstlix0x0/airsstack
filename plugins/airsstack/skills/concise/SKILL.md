---
name: concise
description: Use when the user wants shorter, denser responses — triggers on "concise mode", "be terse", "be brief", "/concise", or any request for less verbose output. Provides a clean professional terse style at lite, full, or ultra levels that persists across the session.
---

# Concise

Clean, professional terseness. Cut wordiness; keep every piece of technical
substance. This is NOT caveman-speak — output stays readable prose, just dense.

## Levels

| Level | What changes |
| --- | --- |
| **lite** | Drop filler (just/really/basically/actually/simply), hedging, and pleasantries (sure/of course/happy to). Keep articles and complete sentences. |
| **full** | Everything in lite, plus: drop articles where unambiguous, fragments OK, prefer short synonyms (use "fix" not "implement a solution for"). |
| **ultra** | Everything in full, plus: telegraphic. Maximal compression — fragments, bullets, minimal connective words. |

Default level when none is given: **full**.

## Always preserved (every level)

- Code blocks, shell commands, and error text — **verbatim**, never compressed.
- Technical terms — exact, never substituted for a looser word.
- **Write normally** (clarity beats brevity) for: security warnings,
  irreversible-action confirmations, and ordered multi-step instructions where a
  dropped word changes the meaning. Resume terse after the careful part.

## Activate / switch / off

- Slash: `/concise` (full), `/concise lite|full|ultra`, `/concise off`.
- Natural language: "concise mode", "be terse", "make it terser", "ultra concise";
  turn off with "normal mode", "verbose mode", or "stop concise".

## Persistence

The active level is stored at `$HOME/.airsstack/cc/concise.json` (override the
root with `$AIRSSTACK_HOME`). The plugin's `UserPromptSubmit` hook reads it every
turn and re-injects the level's directive, so terse mode holds for the whole
session instead of drifting back to verbose. Deleting the file (or `/concise off`)
returns to normal verbosity.

For a one-off, non-persistent terse reply, use the native output style instead:
`/output-style terse`.
