# OKF v0.1 — working digest

The Open Knowledge Format contract this plugin implements. This file is the
ONLY spec source skills and agents read — never re-derive rules from
external documents. Source: Google Cloud's OKF v0.1 draft specification.

## Bundle

A bundle is a self-contained directory tree of UTF-8 Markdown files — the
unit of distribution. Directory structure is producer-chosen. This plugin
targets repo-local bundles (default `knowledge/` at the repo root),
committed WITH the repo.

## Concept documents

- One concept = one Markdown file. **Concept ID = bundle-relative path
  minus `.md`** (`tables/orders.md` → `tables/orders`). Identity lives in
  the filesystem; renaming/moving a file changes its ID.
- Anatomy: a YAML frontmatter block (leading `---` line, closing `---`
  line) followed by a free-form Markdown body.

## Frontmatter fields

| Field | Status | Meaning |
| --- | --- | --- |
| `type` | **REQUIRED** | Short descriptive string (`BigQuery Table`, `Metric`, `Playbook`, …). Free-form: no central registry. Consumers MUST tolerate unknown types. |
| `title` | recommended | Display name; consumers may fall back to the filename. |
| `description` | recommended | ONE summarizing sentence; feeds index generation and previews. |
| `resource` | recommended | URI of the underlying asset; OMIT for abstract concepts. |
| `tags` | recommended | YAML list of short strings. |
| `timestamp` | recommended | ISO 8601 datetime of the last MEANINGFUL change. |

Producers may add any other keys. Consumers MUST NOT reject unknown keys
and MUST preserve them when editing a document (round-trip rule).

## Body conventions

Prefer structural Markdown (headings, tables, lists, fenced code) over
prose. Three headings carry conventional meaning — use when applicable:

- `# Schema` — structured description of an asset's columns/fields.
- `# Examples` — concrete usage, usually fenced code.
- `# Citations` — numbered external sources backing the body's claims.

## Cross-links

- Ordinary Markdown links. **Bundle-relative form is recommended**:
  `[customers](/tables/customers.md)` — leading `/` = bundle root.
- Links are UNTYPED; the relationship kind lives in surrounding prose.
- **Broken links are NOT errors** — a dangling target may be
  not-yet-written knowledge. Consumers must tolerate them.

## Reserved files (the only two)

- `index.md` — enumerates a directory's contents for progressive
  disclosure. NO frontmatter — except the bundle-root `index.md`, which
  may carry exactly `okf_version: "0.1"` (this plugin uses that block as
  the bundle marker). In this plugin `index.md` is SCRIPT-OWNED
  (`gen-index.lua`): never hand-edit it; regenerate it.
- `log.md` — newest-first change history: `## YYYY-MM-DD` ISO date
  headings, entries as list items with a bold leading keyword
  (`**Creation**`, `**Update**`, `**Deprecation**` — convention, not
  requirement). Producer-written at write time.

Every other `.md` file in the bundle is a concept document.

## Conformance (v0.1)

Hard bar — all three must hold:

1. Every non-reserved `.md` has a parseable frontmatter block.
2. Every such block has a non-empty `type`.
3. Reserved files follow their structure wherever present.

Everything else is soft. Consumers MUST NOT reject a bundle for: missing
optional fields, unknown `type` values, unknown extra keys, broken
cross-links, or missing `index.md` (synthesize a view by globbing
instead).

## Versioning

`<major>.<minor>`. Minor = backward-compatible additions. Major = may
break (renamed required fields, changed reserved filenames). Unknown
declared versions get best-effort consumption, never refusal.
