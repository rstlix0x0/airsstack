---
name: okf-concept
description: >
  Author or update ONE OKF concept document in the repo's knowledge
  bundle: resolve the bundle root, write the concept (type required;
  title/description/tags/timestamp recommended; bundle-relative links),
  append a log.md entry, and regenerate index.md. Use when the user says
  "document X in the knowledge bundle" / "/okf-concept X".
---

# okf-concept

Produce one concept document, keep the reserved files honest, report.
Reserved-file rule: log.md is yours to append; index.md is NEVER
hand-edited — the script regenerates it.

## Steps

1. Resolve the bundle root (optional explicit path from the user):

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/okf-root.sh" [explicit-path]
   ```

   Exit 2 → relay the stderr message (it names `/airsstack-okf:okf-setup`
   or asks for an explicit path) and STOP.

2. Read `${CLAUDE_PLUGIN_ROOT}/references/okf-spec.md` once per session;
   conform to it exactly.

3. Choose the concept path: kebab-case filename, grouped into a
   subdirectory by kind (e.g. `crates/`, `metrics/`, `playbooks/`). The
   path IS the concept ID — pick it to last. If the concept already
   exists, this is an update: preserve every frontmatter key you do not
   understand and refresh `timestamp`.

4. Write the document per the digest: required `type`; recommended
   `title`, `description` (one sentence), `tags`, `timestamp`
   (ISO 8601); `resource` only for physical assets. Structural body;
   `# Schema` / `# Examples` / `# Citations` when applicable;
   bundle-relative links (`/path/file.md`) — linking to a not-yet-written
   concept is fine.

5. Append to `<root>/log.md`: ensure today's `## YYYY-MM-DD` heading sits
   at the TOP (newest-first), add
   `- **Creation|Update|Deprecation** — <concept ID>: <one line>`.

6. Regenerate the index:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/scripts/gen-index.sh" "<root>"
   ```

7. Report the concept ID and files touched. Do NOT commit — that is the
   user's call.
