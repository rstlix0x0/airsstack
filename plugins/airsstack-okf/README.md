# airsstack-okf

Open Knowledge Format (OKF) v0.1 producer+consumer toolkit for Claude
Code: provision a repo-local knowledge bundle, author and batch-enrich
concept documents, recall with progressive disclosure, and lint
conformance deterministically.

OKF is Google Cloud's open specification for portable, agent- and
human-readable knowledge: a directory tree of Markdown files with YAML
frontmatter, one required field (`type`), untyped cross-links, and two
reserved filenames (`index.md`, `log.md`). This plugin implements the
full lifecycle against the v0.1 draft; the working contract ships as
`references/okf-spec.md`.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack        # dependency
/plugin install airsstack-okf@airsstack
```

## Components

| Component | What it does |
| --- | --- |
| `/airsstack-okf:okf-setup` (command) | Provision the bundle root (default `knowledge/`): marker `index.md` (`okf_version: "0.1"`) + empty `log.md`. Idempotent. |
| `okf-concept` (skill) | Author/update ONE concept document, append the `log.md` entry, regenerate `index.md`. |
| `okf-enrich` (skill) | Batch-produce concepts for a source scope via the isolated `okf-enricher` agent, then regenerate the index and lint. |
| `okf-recall` (skill) | Answer questions index-first; broad queries go through the read-only `okf-recall` agent (pointers, ≤10 concept reads). |
| `okf-lint` (skill) | Deterministic v0.1 conformance check: hard bar fails, everything else warns. |
| `scripts/` | `okf-root.sh` (bundle detection), `gen-index.sh` (byte-reproducible index regeneration), `okf-lint.sh` — each with a sibling `.test.sh`. |

## Operating rules

- **Strict division of labor.** Agents write concepts and `log.md`;
  scripts own `index.md` and conformance; skills orchestrate. Nobody
  hand-edits `index.md`.
- **Permissive consumption.** Broken links, unknown types, and missing
  recommended fields are warnings, never errors — per the spec.
- **Bundle ships with the repo.** The bundle (default `knowledge/`) is
  committed, not gitignored; the `okf_version` block in the root
  `index.md` doubles as the detection marker.
- **No commits.** Every component leaves committing to the user.

## Out of scope (v1)

Typed relationship extensions, derived graph indexes, journal↔OKF
conversion, per-subdirectory index generation, hooks.

## Dependencies

Requires the `airsstack` plugin (agent conventions). Scripts are
dependency-free POSIX sh/awk — no Python, no jq/yq; agents use only
harness built-ins (Read/Glob/Grep, with Grep backed by Claude Code's
bundled ripgrep).
