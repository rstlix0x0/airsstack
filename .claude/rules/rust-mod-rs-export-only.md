---
paths:
  - "**/mod.rs"
  - "**/lib.rs"
  - "**/*.rs"
---

# Rust — `mod.rs` Is Export-Only

In this repo, `mod.rs` (and `lib.rs`) act as a **table of contents** for their module: module-level docs, submodule declarations, and re-exports. **No implementation lives in `mod.rs`.** Implementation lives in sibling files, one logical unit per file. This rule complements [[rust-microsoft-guidelines]] `M-SMALLER-CRATES`, [[rust-strict-quality]], and [[rust-strong-types]].

## The rule

A `mod.rs` (or `lib.rs`) MUST contain only:

1. **Module-level doc comments** (`//!`) — see [Required module documentation](#required-module-documentation).
2. **Inner attributes** scoped to the module (e.g. `#![cfg_attr(docsrs, …)]`, `#![forbid(unsafe_code)]` at crate root).
3. **Submodule declarations**: `mod foo;`, `pub mod foo;`, optionally feature-gated and `#[doc(cfg(…))]`-annotated.
4. **Re-exports**: `pub use submodule::Item;` to flatten the public surface for callers.
5. **Use statements** required by the above (rare — usually only at crate root if the lib re-exports something using a trait bound, otherwise unnecessary).

A `mod.rs` (or `lib.rs`) MUST NOT contain:

- `struct`, `enum`, `union`, `trait`, `impl` blocks.
- `fn` or `const` or `static` items (other than what a derive or proc-macro injects, which doesn't count).
- `type` aliases with non-trivial right-hand sides (anything beyond `pub type X = OtherCrate::Y;` shorthand for re-export ergonomics).
- Macro invocations that expand to items (`mockall::mock!`, `paste!`, etc.).
- Tests (`#[cfg(test)] mod tests { … }` inline). Use a sibling `tests.rs` (declared as `#[cfg(test)] mod tests;`) or `tests/` subdirectory.

## Required module documentation

Every module file (`mod.rs`, `lib.rs`, and every sibling `*.rs` that contains a `mod` of its own or holds a publicly-exported item) MUST open with a `//!` block that answers four questions, in this order:

1. **What this module is** — one-sentence summary suitable for `cargo doc`'s module index (`M-FIRST-DOC-SENTENCE`). Concrete, not generic. Bad: *"Transport stuff."* Good: *"HTTP transport boundary the SDK sends every Anthropic request through."*
2. **Why it exists** — the *load-bearing reason* the module is separate from its siblings. Could be a constraint (a feature gate, a trait-object exception, a third-party-dep isolation, a layering rule). If you cannot articulate why this module is not just merged into its parent, the module probably shouldn't exist.
3. **Responsibilities** — bulleted list of what the module owns. Names types, traits, or behaviours the module is the home for. A reader scanning the list should be able to predict which sibling file a given concept lives in.
4. **Non-responsibilities** *(when non-obvious)* — what the module **does not** do, especially where readers might mistakenly look here first. Particularly important for boundary modules (transport, codec, parser) where the layer above interprets results. Skip when the boundary is genuinely self-evident.

Optional but encouraged:

- **Entry points** — the one or two types/functions a typical caller reaches for first. Helps agents and humans skip past internal plumbing.
- **Feature gates and cross-references** — when items are conditionally compiled or when the module's contract is reinforced by a `.claude/rules/*.md` rule, link the rule explicitly so readers can find the policy without grep.

Keep the block tight. Aim for 5–15 lines of doc, not a treatise. The goal is *orientation* — readers absorb the shape, then open the sibling file or the rule for depth.

### Canonical doc block

```rust
//! HTTP transport boundary the SDK sends every Anthropic request through.
//!
//! Exists as its own module so the trait can sit behind a feature flag
//! (`transport-reqwest`) and so test code can swap a mock implementation
//! at compile time without paying for dyn dispatch on every request.
//! The trait is one of the few documented `Box<dyn …>` exceptions in this
//! crate — see `.claude/rules/rust-static-dispatch.md`.
//!
//! Responsibilities:
//! - Define [`HttpTransport`] (the user-extension seam) and [`BodyStream`]
//!   (the incremental response body type all implementations return).
//! - Ship the default [`ReqwestTransport`] implementation behind the
//!   `transport-reqwest` feature.
//! - Provide the `mockall`-generated `MockHttpTransport` behind the
//!   private `__test-mocks` feature.
//!
//! Not responsible for:
//! - Interpreting HTTP status codes — 4xx/5xx responses surface as `Ok`;
//!   the layer above maps them to API errors.
//! - Retry, backoff, or rate-limit handling — those live in `client::retry`.
//!
//! Entry point: [`HttpTransport::send`].
```

The block answers all four required questions, links the rule that justifies its trait-object exception, and points the reader at the one method that matters. A new contributor reading just this comment knows what the module is, why it isn't merged into `client/`, where to find each concrete item, and what *not* to expect from it.

### When the rule does not apply

Skip the "why exists / responsibilities" structure on:

- A pure re-export module whose only purpose is path flattening — a one-line `//! Re-exports from [`other_crate`].` is sufficient.
- A test-only sibling (`#[cfg(test)] mod tests;`) — the file's role is self-evident from the cfg.

Every other module owes the reader the four-question block.

## Why

- **Ownership clarity.** Opening `transport/mod.rs` answers *"what does this module export?"* in seconds — no scrolling past 200 lines of trait body to find the third item.
- **Smaller blast radius for diffs.** A change to the `BodyStream` typedef edits `transport/body.rs`. A change to the trait edits `transport/seam.rs`. The diff is honest about what moved.
- **Per-file lint scope stays tight.** Feature-gated implementation lives in its own file with its own `#[cfg(feature = "…")]` on the `mod` line — you don't sprinkle `#[cfg]` across paragraphs of one big file.
- **Discoverability for AI agents and new contributors** (compounds with `M-DESIGN-FOR-AI`). The table-of-contents shape maps directly to a directory listing; an agent reads `mod.rs` first, sees the menu, and only opens the file it needs.
- **Aligns with the Rust ecosystem.** `tokio`, `hyper`, `reqwest`, `axum`, `serde` all follow this layout. Matching idiom reduces friction for outside readers.

## When the rule does NOT apply

Skip the split when the module is a **leaf with no real body**. Two cases:

- A module that is itself only re-exports of *external* crate items (`pub use other_crate::Foo;` × N). No sibling helps; the module file IS the export list.
- A *trivial* module — strictly under ~10 lines, one item, no plausible second item ever joining it. Wait for a second item before splitting. The split has a cost (one more file, one more `use`); pay it when there is something to gain.

When in doubt, split. Re-merging later is mechanical; un-mixing a 400-line `mod.rs` is not.

## Canonical shape

### Before (rejected by review)

```rust
// crates/foo/src/transport/mod.rs
//! HTTP transport.

use bytes::Bytes;
use futures_core::Stream;
use http::{Request, Response};
use std::pin::Pin;
use crate::error::TransportError;

pub type BodyStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send + 'static>>;

#[async_trait::async_trait]
pub trait HttpTransport: Send + Sync + 'static {
    async fn send(&self, req: Request<Bytes>)
        -> Result<Response<BodyStream>, TransportError>;
}

#[cfg(feature = "__test-mocks")]
mockall::mock! { /* ... lots of macro body ... */ }
```

Three concerns (typedef, trait, mock) crammed into one file. `mod.rs` is doing concrete work.

### After (correct)

```rust
// crates/foo/src/transport/mod.rs
//! HTTP transport.
//!
//! `HttpTransport` is the user-extension seam; `BodyStream` is the
//! incremental body type all implementations return.

pub mod body;
pub mod seam;

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub mod reqwest_impl;

#[cfg(feature = "__test-mocks")]
#[cfg_attr(docsrs, doc(cfg(feature = "__test-mocks")))]
pub mod mock;

pub use body::BodyStream;
pub use seam::HttpTransport;

#[cfg(feature = "transport-reqwest")]
pub use reqwest_impl::ReqwestTransport;
```

```rust
// crates/foo/src/transport/body.rs — single typedef + its imports.
// crates/foo/src/transport/seam.rs — the trait + its imports.
// crates/foo/src/transport/mock.rs — the mockall::mock! invocation.
// crates/foo/src/transport/reqwest_impl.rs — the concrete impl.
```

The `mod.rs` is a five-second read. Each sibling owns one concept.

## How this interacts with `pub use` flattening

Re-exports in `mod.rs` SHOULD flatten the public surface so callers write `clauders::transport::HttpTransport`, not `clauders::transport::seam::HttpTransport`. The submodule is an organizational tool, not a path users navigate. Annotate flattening re-exports with `#[doc(inline)]` when the type's docs should appear on the parent module page (`M-DOC-INLINE`).

```rust
pub mod seam;
#[doc(inline)]
pub use seam::HttpTransport;
```

Submodules may be `pub mod` (visible in rustdoc as a separate page) or `pub(crate) mod` (internal organization only). Prefer `pub mod` for public-API modules so users can navigate to the file containing the item — `mod.rs` then becomes a navigation aid, not a hiding place.

## Naming siblings

- One concrete type / typedef / trait per file — name the file after the item in `snake_case` (`body.rs`, `seam.rs`, `client.rs`).
- Avoid generic names (`utils.rs`, `helpers.rs`, `common.rs`) — they invite mixed-concern dumping (`M-CONCISE-NAMES`).
- Avoid filenames that collide with widely-imported external crates (`http.rs` clashes visually with `use http::...` — pick `seam.rs` or `transport_trait.rs` instead).
- A test module that pairs with one source file goes inline at the **bottom of that source file** under `#[cfg(test)] mod tests { … }` — this is the documented exception: tests belong with the thing they test. The "no inline tests" prohibition above applies only to `mod.rs` / `lib.rs`.

## Things to AVOID

- Putting a `trait`, `struct`, `enum`, `impl`, or `fn` definition directly in `mod.rs`. Move it.
- Putting a 50-line `mockall::mock!` invocation in `mod.rs`. Move it.
- Hiding a sibling submodule behind `pub(crate) mod` *just* because the file is "small" — visibility is a contract, not a tidying preference.
- Naming a sibling `mod_impl.rs`, `internal.rs`, `impl.rs` — these are anti-names. Name the file after what it contains.
- Catch-all re-exports at the crate root (`pub use foo::*`) — violates `M-NO-GLOB-REEXPORTS`. Re-export explicitly.

## Definition of Done (rule additions)

In addition to [[rust-strict-quality]] DoD:

- Reviewer rejects any `mod.rs` or `lib.rs` containing a `struct`, `enum`, `trait`, `impl`, `fn`, `const`, `static`, item-emitting macro invocation, or non-trivial `type` alias.
- Inline `#[cfg(test)] mod tests { … }` blocks are forbidden in `mod.rs` / `lib.rs`. Tests inline with the source file they cover; integration tests live under `crates/<name>/tests/`.
- A new submodule introduced by a change must own exactly one logical concept; reviewer questions any sibling whose name doesn't describe its contents.
- Every module file ships the four-question `//!` block (what / why exists / responsibilities / non-responsibilities-when-non-obvious). Reviewer rejects `//! Some module.`-style stubs. Exceptions: pure re-export modules and `#[cfg(test)]` test siblings as noted above.
