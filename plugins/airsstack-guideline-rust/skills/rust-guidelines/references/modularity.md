# Rust — Modularity: One Responsibility, One Home

Every type, module, and function owns **one** clearly-nameable responsibility, and every concept has
**exactly one** canonical type. This rule targets two specific anti-patterns — the **God object** and
the **duplicate type** — that reviewers must actively hunt for. It complements `mod-rs-export-only`
(file layout), `strong-types` (how a domain value is modeled), and the Microsoft `M-SMALLER-CRATES` /
`M-DESIGN-FOR-AI` references.

The two are opposite failure modes of the same axis. A God object collapses many responsibilities into
one unit; a duplicate type splits one concept across many units. Both destroy the "one concept ⇄ one
name ⇄ one file" mapping that makes a codebase navigable.

## Rule 1 — No God object

A **God object** is a type, module, or function that has accreted responsibilities it should not own.
Symptoms a reviewer rejects:

- A `struct`/`enum` whose fields serve several unrelated concerns (config **and** live connection
  state **and** a cache **and** metrics), so almost every change touches it.
- A module file mixing several distinct concept clusters — e.g. a permission module holding both the
  *decision* vocabulary and an unrelated *rule-persistence* cluster. Split the clusters into sibling
  files under a folder module (per `mod-rs-export-only`), one concept per file.
- A function longer than it needs to be because it does resolution **and** I/O **and** formatting
  **and** error mapping. Extract the phases into named helpers each independently testable.
- A "manager", "handler", "util", "helper", or "context" type that grew a grab-bag of methods with no
  single load-bearing reason to be one type. Generic names (`M-CONCISE-NAMES`) invite this — name a
  unit after the *one* thing it owns; if you cannot, it owns too much.

The litmus test is the module doc's **"why it exists"** sentence (`mod-rs-export-only`): if you cannot
state a single load-bearing responsibility without the word "and" joining two unrelated concerns, the
unit is a God object — split it.

### Splitting is behavior-preserving

When a file grows a second concept cluster, convert it to a folder module and move each cluster into a
sibling file. Because the `mod.rs` re-exports every item (`pub use`), external import paths
(`crate::foo::Bar`) stay stable — the split touches only internal file boundaries, not consumers. A
God object is far cheaper to prevent than to un-mix later, so **split at the moment a second unrelated
responsibility appears**, not "when it gets big."

## Rule 2 — No duplicate types

A **duplicate type** is two (or more) types that model the *same concept*. It forces every consumer to
convert between them, lets the two drift out of sync, and doubles the surface a reader must learn.
Reviewers reject:

- Two enums/structs with the same variants/fields and the same meaning, differing only by name or by a
  field one carries and the other drops (e.g. an engine returning a private `Gate { Allow, Deny }` that
  mirrors the public `Decision { Allow{..}, Deny{..} }`). **Reuse the one canonical type**; if an
  intermediate stage genuinely needs less data, that is usually a sign to drain/transform the canonical
  value in place, not to mint a parallel type.
- A bare discriminant re-declared per consumer (`FooBehavior`, `BarBehavior`, both `{ Allow, Deny }`).
  Declare **one** shared discriminant enum and reuse it. A payload-carrying enum and a bare
  discriminant enum are *not* duplicates — they have different shapes and roles — but two bare
  discriminants with identical variants are.
- A "wire" struct and a "domain" struct that are field-for-field identical with no transformation
  between them. If they never diverge, they are one type. (They are legitimately **two** only when the
  wire shape and the domain shape carry different invariants — e.g. a validated newtype vs its raw
  string form, or a serde mirror with a genuinely different field set. Document the divergence in each
  type's doc so the split is not mistaken for duplication.)
- Re-deriving a concept the crate (or a workspace dependency) already exports. Before adding a type,
  grep for an existing one; reuse or re-export it rather than declaring a twin.

### The canonical-type test

For any concept, there is exactly one type that *is* that concept; everything else **references** it.
When tempted to add a type, ask: "Does an existing type already mean this?" If yes, reuse it. If it
means *almost* this, decide whether the difference is a real invariant (keep both, document why) or
mere convenience (reuse the one, transform in place).

## Why

- **Navigability / `M-DESIGN-FOR-AI`.** One concept ⇄ one name ⇄ one file lets a human or agent predict
  where a thing lives and read only that file. God objects and duplicate types both break the mapping.
- **Honest diffs, small blast radius.** A change to one responsibility touches one file. A God object
  makes every change touch the same hot file; a duplicate type makes one logical change edit two places
  (and risks editing only one).
- **Testability.** A single-responsibility unit is testable in isolation (`unit-test-mandate`). A God
  object needs the whole world stood up; a duplicate type needs its converters tested too.
- **No drift.** One canonical type cannot fall out of sync with itself.

## When these rules do NOT apply

- **Cohesive multi-item modules.** A file may hold two or three types that form *one* concept and are
  always read together (a decision enum plus the context struct it consumes, plus that enum's
  constructors). Cohesion, not raw count, is the test — do not fragment one concept into five
  one-line files (`mod-rs-export-only` warns against premature splitting: wait for a real second
  concept).
- **Genuinely-distinct look-alikes.** Two types that happen to share a shape but carry different
  invariants or evolve independently are not duplicates. Keep them and document the divergence.

## Things to AVOID

- A struct that is edited by nearly every feature (hot-file smell) — decompose its responsibilities.
- A `*Manager` / `*Util` / `*Helper` / `*Context` type accumulating unrelated methods.
- A private enum in one module that mirrors a public enum in another — reuse the public one.
- A second bare `{ Allow, Deny }`-style discriminant when one already exists — share it.
- A wire/domain type pair that is field-identical with an identity conversion between them — collapse
  to one.
- Adding a type without first grepping for an existing one that means the same thing.

## Definition of Done (rule additions)

In addition to the strict-quality reference DoD:

- Reviewer rejects any type/module/function whose single responsibility cannot be stated without an
  "and" joining unrelated concerns (God-object gate). The `mod-rs-export-only` "why exists" sentence is
  the check.
- Reviewer rejects a newly-introduced type that duplicates the concept of an existing type in the crate
  or a workspace dependency. Reuse, re-export, or transform-in-place instead. A look-alike kept on
  purpose must document, in its doc comment, the invariant that distinguishes it from its twin.
- When a change adds a second unrelated responsibility to an existing file, the reviewer expects the
  file split into sibling concept-files (behavior-preserving via `mod.rs` re-export) in the same change,
  not deferred.
