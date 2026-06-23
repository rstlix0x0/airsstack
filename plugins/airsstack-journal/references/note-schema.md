# airsstack-journal note schema

The single source of truth for the journal note contract. The index builder
(`scripts/build-index.py`) and every later phase that authors notes both
conform to this file. Notes are Obsidian-compatible Markdown: a YAML-style
frontmatter fence followed by a Markdown body.

## Frontmatter fields

| Field | Type | Meaning |
| --- | --- | --- |
| `title` | string | Human-readable label; shown in Obsidian. |
| `type` | enum | One of `insight`, `decision`, `session`, `daily`, `moc`. |
| `project` | scalar or list | Project key(s) the note pertains to. |
| `domains` | list | Broad subject areas (e.g. `async-rust`). |
| `tags` | list | Fine-grained topic tags (e.g. `tokio`). |
| `created` | timestamp | `YYYY-MM-DD HH:MM` of creation. |
| `updated` | timestamp | `YYYY-MM-DD HH:MM` of last update. |
| `links` | list | Outbound `[[wikilink]]` strings. |
| `helped` | integer | Write-back counter; how often the note aided a solution. Default `0`. |
| `summary` | string | One-line distilled top of the note; returned in a recall pointer. |

Frontmatter is a leading `---` fence of flat `key: value` pairs. A value is
either a scalar or an inline list `[a, b, c]`. Nested structures are not used,
so the parser stays dependency-free.

Example:

```yaml
---
title: Tokio cancellation safety
type: insight
project: clauders
domains: [async-rust, concurrency]
tags: [tokio, cancellation, shutdown]
created: 2026-06-23 14:42
updated: 2026-06-23 14:42
links: ["[[graceful-shutdown]]", "[[structured-concurrency]]"]
helped: 0
summary: await points are cancel points; drop guards still run on cancel
---
```

## Canonical identifiers

A note's stable identifier is its **kebab-case filename stem**, e.g.
`notes/tokio-cancellation-safety.md` → `tokio-cancellation-safety`. Wikilinks
use the same kebab text: `[[tokio-cancellation-safety]]`. The human label lives
in `title:`, not in the filename. Link resolution is a **case-insensitive exact
stem match**: `[[Tokio-Cancellation-Safety]]` resolves to
`tokio-cancellation-safety.md`. A wikilink may carry an Obsidian alias
(`[[stem|alias]]`) or heading (`[[stem#heading]]`); only the stem portion before
`|` or `#` is used for resolution.

## Fields the index builder consumes

The builder reads `title`, `summary`, `project`, `helped`, `updated`, `tags`,
`domains`, and `links`, plus inline `[[wikilink]]` occurrences in the body. It
ignores `type` and `created` (they are part of the note contract but not part of
the derived index). Notes carry their `project` value as data; the builder never
derives a project key — that derivation belongs to the capture phase.
