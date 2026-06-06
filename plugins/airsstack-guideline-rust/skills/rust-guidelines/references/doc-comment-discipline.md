# Rust — Doc & Comment Discipline (No Internal-Artifact Leakage)

Code comments and rustdoc address **engineers reading or consuming this crate** — its public surface, invariants, constraints, and behaviour. They do NOT address the project's internal development process. Anything that exists only because of how the repo is *managed* (AI policies, plan documents, phase/task identifiers, internal review cycles) stays out of source files. This rule reinforces `M-FIRST-DOC-SENTENCE`, `M-MODULE-DOCS`, `M-CANONICAL-DOCS`, and `M-DOCUMENTED-MAGIC` from the Microsoft guidelines reference, and complements the mod-rs-export-only reference.

## The rule

Doc comments (`///`, `//!`) and source comments (`//`) MUST explain code, behaviour, or external context. They MUST NOT name or describe internal project artifacts.

### Disallowed in source (rustdoc and `//` comments alike)

- Paths or filenames under internal-only directories (such as out-of-band planning or tooling directories). Example: `// see internal-rules/rust-static-dispatch.md` — **rejected**.
- Names of internal rules, rule-numbered exceptions, or rule shortcodes that only resolve inside the local project. Example: `// project-rule exception #3` — **rejected**.
- Plan / spec / phase / task identifiers. Examples: `// Phase 3 Task 2`, `// per the v0.1 spec §8.4`, `*(lands in Task 3)*`, `// TODO Phase 5` — **rejected**.
- Workflow vocabulary from the development process: `subagent`, `implementer`, `reviewer`, `cavecrew`, `superpowers`, `the plan`, `the spec`, `the brainstorm`. Engineers reading the crate do not have this vocabulary; using it leaks the workflow into the contract.
- AI/agent/model names in code comments: `Claude`, `Opus`, `Sonnet`, `the assistant`. Exception: when the *crate itself* is about LLM APIs and the name appears as a literal model identifier in a public type, identifier, or documented constant (e.g. a `ModelIdentifier::CLAUDE_SONNET` constant in an LLM SDK crate).
- PR / issue / commit references inside source. They belong in git history (commit messages, PR descriptions), not in the file — line numbers and SHAs rot quickly and the next reader does not have the issue tracker open.
- Narration of past or future work in this codebase: `added later`, `as discussed`, `originally written by`, `previously called`, `we decided to`. The diff explains *what changed*; the comment explains *what the code does*.

### Allowed (and encouraged)

- What the code does, the invariants it relies on, the contract it offers callers.
- Constraints and trade-offs the reader cannot derive from the code alone (hidden coupling, performance characteristics, ordering requirements).
- Cross-references via **rustdoc intra-doc links** to public items in this crate (`[`Storage`]`, `[`crate::error::StorageError`]`).
- References to **publicly published external standards and documentation**: RFCs, HTTP spec, API documentation URLs, `docs.rs` links for external crates, well-known industry guidelines.
- Microsoft Pragmatic Rust Guidelines codes (`M-*`) — they are an external public document. Acceptable in commit messages and rustdoc when motivating a design choice readers can look up.
- Repo-local *comment markers* that are grep-able conventions, not artifact references: `// SAFETY:`, `// dyn:`, `// PERF:`. The marker is fine; do NOT chase it with a path back into internal rule files. The reason text after the marker must stand on its own.

### Asking "where does this rule live?"

If a code reviewer cites an internal rule file on a finding, the *commit message* is the right place to acknowledge the rule (e.g. `Per M-DI-HIERARCHY, …`). The source file itself encodes the *decision* — not the bureaucracy that produced it. A reader two years from now opening `storage/seam.rs` cares about why `Storage` is `Send + Sync + 'static`, not which internal markdown file once said so.

## How to translate a leaky comment

Common rewrites:

| Leaky                                                                                       | Clean                                                                                                    |
| ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `// Per internal-rules/rust-static-dispatch.md exception #1.`                              | `// dyn: heterogeneous concrete body-stream types across transport implementations.`                     |
| `//! Static-dispatch policy in internal-rules/rust-static-dispatch.md.`                    | *(delete the line; the `// dyn:` justification at the use site already explains the trade-off)*           |
| `//! Layout follows the export-only mod.rs rule.`                                          | `//! mod.rs re-exports only; concrete items live in sibling files.`                                      |
| `*(lands in Phase 3 Task 3)*`                                                               | *(delete; the type either exists or does not — future plans belong outside the source tree)*              |
| `// TODO Phase 5: support streaming.`                                                       | `// TODO: streaming support is unimplemented.` *(better: track in an issue and link the issue in commit)* |
| `// Decided in 2026-05-28 brainstorm session to keep this unbounded.`                       | `// Server enforces the upper bound; SDK-side cap would age badly across model releases.`                |
| `//! MockStorage (gated __test-mocks) — lands in Task 5.`                                  | *(delete the temporal qualifier; describe what exists now, period)*                                       |

The pattern: replace "where the decision came from" with "what the decision *is* and *why it makes engineering sense*".

## Why

- **The crate's public docs are its contract.** `cargo doc` output gets read by downstream consumers, search engines, and AI agents indexing crates. Internal repo paths in that output are noise at best and confusing at worst — readers have no way to follow internal tooling paths and shouldn't try.
- **Internal artifacts churn.** Rules get renumbered, plans get superseded, phases get re-ordered. Source files that name them go stale silently. The diff in a code review can show that a rule was renamed; the diff cannot show that twenty rustdoc comments now point at a moved file.
- **Mixing process with product is a smell.** The reader of `seam.rs` is trying to use `Storage`. They are not trying to learn how the team coordinates work. Process commentary in source files breaks the reading flow without paying it back.
- **AI agents writing code tend to leak workflow vocabulary** ("as the plan describes", "Phase 3 Task 2 below"). The fact that this comes naturally to agents is the precise reason the rule must be explicit — otherwise leakage compounds with every generated file.

## Boundary: where the artifact reference *does* belong

- **Commit messages**: cite rule codes, plan rationale, and review findings here. `Per M-LINT-OVERRIDE-EXPECT, switched #[allow] to #[expect]` is fine in a commit body.
- **PR descriptions**: link to specs, plans, design docs, prior incidents — these are project-management surfaces.
- **Project rules files and planning files**: cross-link freely. These files are *for* the project process.
- **Out-of-band planning notes** (gitignored scratch, internal specs/plans): free-form, never read by `rustdoc`, not shipped — narrate however helps you.

The forbidden zone is **the source tree under `crates/*/src/`, `crates/*/tests/`, `crates/*/examples/`, and any `README.md` shipped with a crate** — anything `cargo doc` reads, anything downstream consumers see, anything that ends up in the published crate.

## Things to AVOID

- Quoting the `# Examples` / `# Errors` / `# Panics` doc sections of an internal rule file by name in a source comment. Just state the constraint.
- Naming an internal rule file even in a `//` comment "for traceability". Reviewer rejects.
- Including in a doctest a reference to an internal planning artifact to explain why the test exists. Tests describe the behaviour under test, not the planning that produced them.
- README.md text that walks the reader through internal phases ("This crate is in Phase 3 of the implementation plan…"). The README is for users.
- Module docs that read like a journal entry ("Originally we wanted X but pivoted to Y after…"). Describe the *current* design.

## Definition of Done (rule additions)

In addition to the strict-quality reference DoD and the mod-rs-export-only reference DoD:

- Reviewer greps the touched files for internal planning path patterns, `Phase `, `Task `, `Step `, `subagent`, `implementer`, `the plan`, `the spec`, `the brainstorm` and rejects matches in source / rustdoc / shipped README.
- Source comments naming an internal rule file path or internal rule number are rejected even when factually correct — the rule's content is what matters, the file is internal.
- Newly written rustdoc explains the type/function/module on its own terms without requiring the reader to open any file outside the published crate.
