# Rust — Modularity: One Responsibility, One Home

Every type, module, and function owns **one** clearly-nameable responsibility, and every concept has
**exactly one** canonical type. Two opposite failure modes of the same axis: a **God object** collapses
many responsibilities into one unit; a **duplicate type** splits one concept across many units. Both
break the "one concept ⇄ one name ⇄ one file" mapping. Complements `mod-rs-export-only` (file layout),
`strong-types` (how a domain value is modeled), `M-SMALLER-CRATES`, `M-DESIGN-FOR-AI`.

## Rule 1 — No God object

A type, module, or function that has accreted responsibilities it should not own. Reviewers reject:

- A `struct`/`enum` whose fields serve unrelated concerns (config **and** connection state **and** a
  cache **and** metrics), so almost every change touches it.
- A module file mixing distinct concept clusters — e.g. a permission module holding both the *decision*
  vocabulary and an unrelated *rule-persistence* cluster. Split into sibling files under a folder
  module, one concept per file.
- A function doing resolution **and** I/O **and** formatting **and** error mapping. Extract each phase
  into a named, independently testable helper.
- A `*Manager` / `*Util` / `*Helper` / `*Context` type accumulating a grab-bag of methods. Generic names
  (`M-CONCISE-NAMES`) invite this — name a unit after the *one* thing it owns; if you cannot, it owns
  too much.

**Litmus test:** the module doc's "why it exists" sentence (`mod-rs-export-only`). If you cannot state a
single load-bearing responsibility without an "and" joining unrelated concerns, split it.

Splitting is behavior-preserving: `mod.rs` re-exports every item, so external paths (`crate::foo::Bar`)
stay stable. Split **the moment a second unrelated responsibility appears**, not "when it gets big" —
but cohesion, not raw count, is the test. Two or three types forming *one* concept and always read
together belong in one file; do not fragment a concept into five one-line files.

## Rule 2 — No duplicate types

Two or more types modeling the *same concept*. Forces converters, drifts out of sync, doubles the
surface a reader learns. Reviewers reject:

- Two enums/structs with the same variants and meaning, differing only by name or a dropped field (a
  private `Gate { Allow, Deny }` mirroring the public `Decision { Allow{..}, Deny{..} }`). Reuse the
  canonical type; if a stage needs less data, transform the canonical value in place.
- A bare discriminant re-declared per consumer (`FooBehavior`, `BarBehavior`, both `{ Allow, Deny }`).
  Declare one shared enum. A payload-carrying enum and a bare discriminant are *not* duplicates —
  different shapes, different roles — but two bare discriminants with identical variants are.
- A "wire" struct and a "domain" struct that are field-identical with an identity conversion. They are
  legitimately two only when they carry different invariants (a validated newtype vs its raw string
  form, a serde mirror with a different field set) — document that divergence in each type's doc.
- Re-deriving a concept the crate or a workspace dependency already exports. Grep before declaring.

**The canonical-type test.** For any concept there is exactly one type that *is* that concept;
everything else references it. Before adding a type, ask: does an existing type already mean this? If
yes, reuse. If it means *almost* this, decide whether the difference is a real invariant (keep both,
document why) or mere convenience (reuse, transform in place).

## Definition of Done (rule additions)

In addition to the strict-quality reference DoD:

- Reviewer rejects any type/module/function whose single responsibility cannot be stated without an
  "and" joining unrelated concerns.
- Reviewer rejects a new type duplicating an existing concept in the crate or a workspace dependency. A
  look-alike kept on purpose documents, in its doc comment, the invariant distinguishing it from its twin.
- A change adding a second unrelated responsibility to a file splits that file into sibling concept-files
  in the same change, not deferred.
