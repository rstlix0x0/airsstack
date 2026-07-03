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

## Usage

The lifecycle is **provision → enrich → lint → recall**.

1. **Provision the bundle** (once per repo):

   ```
   /airsstack-okf:okf-setup
   ```

   Creates `knowledge/` with the `okf_version: "0.1"` marker `index.md`
   and an empty `log.md`. Idempotent — safe to re-run.

2. **Produce knowledge.** Batch-enrich a whole source scope — any
   package, module, service, or docs directory in any language:

   ```
   /okf-enrich <source-scope>
   ```

   e.g. `/okf-enrich src/auth`, `/okf-enrich packages/api`,
   `/okf-enrich docs/architecture`. The isolated `okf-enricher` agent
   writes one concept document per public asset (with cross-links and
   citations), appends one `log.md` entry each, then the skill
   regenerates `index.md` and lints. For a single document instead,
   use `/okf-concept <topic>`.

3. **Check conformance** anytime:

   ```
   /okf-lint
   ```

   Hard failures (spec violations) are listed separately from
   permissive warnings (broken links, missing recommended fields).

4. **Consume.** Ask the bundle instead of re-reading source:

   ```
   /okf-recall <question>
   ```

   Targeted questions (answer plausibly in ≤3 concepts) are answered
   inline, index-first. Broad questions spawn the read-only
   `okf-recall` agent, which returns compact pointers plus a grounded,
   cited summary under a hard cap of 10 concept reads. Questions the
   bundle does not cover get an honest "not in bundle", never
   fabricated content.

### Recall triggering

The recall skill auto-triggers only when a prompt clearly refers to the
knowledge bundle (e.g. "what does the knowledge bundle say about X").
Plain free-text questions about covered code will usually be answered
from source instead. To route them through the bundle, either invoke
`/okf-recall` explicitly or add one line to the repo's `CLAUDE.md`:

```
Questions about code covered by `knowledge/` — consult /okf-recall
before reading source.
```

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
