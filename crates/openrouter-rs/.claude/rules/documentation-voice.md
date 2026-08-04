---
paths:
  - "**/openrouter-rs/docs/**/*.md"
  - "**/openrouter-rs/README.md"
  - "**/openrouter-rs/src/**/*.rs"
  - "**/openrouter-rs/examples/**/*.rs"
---

# Documentation voice

Applies to everything a downstream user reads: `docs/`, `README.md`, rustdoc comments in
`src/`, and the `examples/`. It does not apply to `CLAUDE.md`, `.claude/`, commit messages,
or anything else written for whoever maintains the crate.

## The rule

**Write about the crate, never about the work of building it.** A reader is deciding how to
call an API. The state of the project is not their concern and gives them nothing to act on.

## Never appears in reader-facing text

Development-process vocabulary, in any form:

> phase, development phase, roadmap, milestone, backlog, workstream, action plan, plan,
> spec, specification, spec-driven, SDD, Definition of Done, parity, pivot, sprint,
> iteration, TODO, FIXME, WIP, "in progress", "not yet", "coming soon", "will be added",
> "we will", "eventually", "for now", "at present", "currently", "today", "so far"

Also never:

- **Roadmap hints.** "Reserved for a future X", "a later release will…", "when we add…".
  Anything that tells the reader what has not been built yet.
- **Internal tooling and process names.** Agent names, skill or plugin names, session or
  journal references, commit SHAs, branch names, issue or ticket numbers, internal plan or
  spec file paths.
- **First person about the authors.** "We chose", "our approach", "we decided". Write about
  the code: "the crate does X", "`build()` returns Y".
- **Editorial self-commentary.** "…rather than glossed over", "documented honestly",
  "as noted during review". How the docs got written is not documentation.
- **Sections addressed to maintainers.** No "For contributors", "Internal notes",
  "Implementation notes for the team". If a reader cannot call it, reach it, or see it in
  rustdoc, it does not belong in reader-facing docs at all. A `#[cfg(test)]` or private item
  is invisible downstream — leave it out rather than explaining why it is unavailable.

## Say the same thing without the timeline

State the observable behaviour. Drop the temporal hedge and the implied plan.

| Instead of | Write |
|---|---|
| "does not currently affect requests" | "does not affect requests" |
| "cannot fail in the current implementation" | "cannot fail" |
| "reserved for a future cross-field validation" | "never constructed" |
| "two endpoints exist today" | "the crate has two endpoints" |
| "not modelled in this release" | "not modelled" |
| "one variant in this release" | "one variant" |
| "not yet wired into the request path" | "has no effect on the request path" |

When a design choice genuinely needs justifying, justify it by what it does **for the
reader**, not by what the authors might do later:

- ❌ "`build()` returns `Result` to reserve the failure path for future checks."
- ✅ "`build()` returns `Result` so you propagate with `?`; `BuildError` is
  `#[non_exhaustive]`."

## What must still be said

Removing process vocabulary is not the same as hiding behaviour. These stay, stated flatly:

- **A setter or type that does nothing.** `ClientBuilder::timeout` is stored and readable
  but never applied. A reader who trusts it silently loses their timeout, so the page says
  so and gives the working alternative. Describe the behaviour, not the defect.
- **Fields and forms the crate does not model**, so a reader knows not to look for them.
- **Variants that are declared but never constructed**, since they show up in a `match`.
- **Version pinning.** "As implemented at version 0.1.0" is a fact about the artifact.

Absent surface is documented as absent, never as pending.

## Before committing docs

```bash
grep -rniE "\b(phase|roadmap|milestone|backlog|workstream|sdd|spec|action plan|definition of done|parity|TODO|FIXME|WIP|not yet|coming soon|currently|today|for now|eventually|we will|reserved for)\b" docs/ README.md src/ examples/
```

Every hit is either a rewrite or a deliberate exception you can defend. Legitimate matches
exist — `"anthropic/claude-3-haiku"` in a model id, `Debug` matching a `bug` pattern — so
read each one rather than trusting the count.
