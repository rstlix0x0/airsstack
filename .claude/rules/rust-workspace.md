---
paths:
  - "**/Cargo.toml"
  - "**/Cargo.lock"
  - "**/*.rs"
  - "**/rust-toolchain*"
---

# Rust — Workspace Layout & Conventions

Workspace structure follows the official [Cargo Book ch. 14.3](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) plus modern centralization features (`workspace.package`, `workspace.dependencies`, `workspace.lints`). Cross-links: [[rust-microsoft-guidelines]] `M-SMALLER-CRATES`, [[rust-strict-quality]] (lint policy).

## Why a workspace

`airsstack` ships multiple crates (`airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`) that evolve together. A single workspace gives:

- **One `Cargo.lock`** → every crate compiles against the same dep versions. No skew between `core` and `provider-*`.
- **Shared `target/`** at the workspace root → inter-crate deps build once, not per-crate. Big disk + time savings.
- **Centralized metadata, deps, and lints** via `[workspace.*]` tables → bumping `serde` or `tokio` is a one-line change.
- **Atomic refactors** across crates land in one PR.

## Root `Cargo.toml` shape

The workspace root has **no `[package]` section**. Use this template:

```toml
[workspace]
resolver = "3"
members = [
    "crates/airsstack-cli",
    "crates/airsstack-core",
    "crates/provider-claude",
    "crates/provider-openrouter",
    "crates/airsdsp",
]
# Optional: exclude scratch crates from the workspace
# exclude = ["scratch/*"]
# Optional: limit `cargo build` / `cargo test` when run without -p
default-members = ["crates/airsstack-cli"]

[workspace.package]
version      = "0.1.0"
edition      = "2024"
rust-version = "1.85"   # bump in lockstep across all crates
license      = "Apache-2.0"
repository   = "https://github.com/rstlix0x0/airsstack"
authors      = ["rstlix0x0 <rstlix.dev@gmail.com>"]

[workspace.dependencies]
# Pin once, reuse everywhere via `dep.workspace = true`
tokio       = { version = "1", features = ["macros", "rt-multi-thread"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
thiserror   = "2"
anyhow      = "1"
tracing     = "0.1"
reqwest     = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }

# Internal crates referenced by other workspace members
airsstack-core      = { version = "0.1.0", path = "crates/airsstack-core" }
provider-claude     = { version = "0.1.0", path = "crates/provider-claude" }
provider-openrouter = { version = "0.1.0", path = "crates/provider-openrouter" }
airsdsp             = { version = "0.1.0", path = "crates/airsdsp" }

[workspace.lints.rust]
unsafe_code        = "deny"
missing_docs       = "warn"
rust_2018_idioms   = { level = "warn", priority = -1 }

[workspace.lints.clippy]
all      = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery  = { level = "warn", priority = -1 }
cargo    = { level = "warn", priority = -1 }

[profile.release]
lto           = "thin"
codegen-units = 1
strip         = "symbols"
```

`resolver = "3"` is required for Edition 2024 and matches the latest Cargo book guidance. Keep `rust-version` in sync across all crates by inheriting it (`rust-version.workspace = true`).

Profiles (`[profile.dev]`, `[profile.release]`) are **only valid in the workspace root** — Cargo ignores them in member crates.

## Member `Cargo.toml` shape

Every member crate inherits metadata and deps from the root:

```toml
[package]
name         = "airsstack-core"
version.workspace      = true
edition.workspace      = true
rust-version.workspace = true
license.workspace      = true
repository.workspace   = true
authors.workspace      = true
description = "Core agentic framework for airsstack."
readme      = "README.md"

[dependencies]
tokio       = { workspace = true }
serde       = { workspace = true }
thiserror   = { workspace = true }
tracing     = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util", "macros", "rt"] }

[lints]
workspace = true
```

Rules:

- **Never hard-code a version** in a member crate if the dep is declared in `[workspace.dependencies]`. Use `{ workspace = true }`.
- **Never re-declare `[lints]`** in a member — opt in via `workspace = true` so [[rust-strict-quality]] applies uniformly.
- **Per-crate `description`** is required for crates that will be published. `name` is required and must match the directory name.
- Each publishable member has its own `README.md` (Cargo's `readme` field) — `crates.io` renders it on the crate page.

## Directory layout

```
airsstack/
├── Cargo.toml              # workspace root
├── Cargo.lock              # one lockfile, committed
├── target/                 # shared build output (gitignored)
├── crates/
│   ├── airsstack-cli/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/main.rs
│   ├── airsstack-core/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/lib.rs
│   ├── provider-claude/
│   │   └── ...
│   ├── provider-openrouter/
│   │   └── ...
│   └── airsdsp/
│       └── ...
└── ...
```

Put all members under `crates/`. Reasons:

- Top-level stays scannable (workspace root, `.claude/`, `.superpowers/`, `docs/`, `crates/`).
- `members = ["crates/*"]` glob keeps the root `Cargo.toml` short as crates are added.
- Encourages adding new crates rather than dumping modules into existing ones (`M-SMALLER-CRATES`).

## Naming convention

- CLI / app binaries: `airsstack-<thing>` (e.g. `airsstack-cli`).
- Library framework crates: `airsstack-<thing>` (e.g. `airsstack-core`).
- Provider implementations: `provider-<service>` (e.g. `provider-claude`, `provider-openrouter`).
- Experiments / specialized: short distinct names (`airsdsp`).
- Directory name MUST equal crate `name`. No `crates/foo-bar/` with `name = "fooBar"`.
- Crate names use kebab-case; the corresponding Rust import is snake_case (`provider-claude` → `use provider_claude;`).

## Inter-crate dependencies

Two valid styles. **Prefer the workspace-deps style** because it pins the version once:

```toml
# Member Cargo.toml — preferred
[dependencies]
airsstack-core = { workspace = true }
```

vs the bare path dep (acceptable for early prototyping; convert to workspace-deps before publishing):

```toml
[dependencies]
airsstack-core = { path = "../airsstack-core" }
```

For `crates.io`-publishable members, the workspace-deps form must include both `version` and `path` (Cargo uses `path` for local builds, `version` for the published crate). Already shown in the root template above.

## Common commands

```bash
# Build / check the whole workspace
cargo build
cargo check --workspace --all-targets --all-features

# Build / test one crate
cargo build -p airsstack-core
cargo test  -p airsstack-core

# Run a binary crate
cargo run -p airsstack-cli -- <args>

# Apply lint / format policy uniformly
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Publish (one crate at a time; dependents first)
cargo publish -p airsstack-core
cargo publish -p provider-claude
cargo publish -p airsstack-cli
```

`cargo` commands without `-p` operate on `default-members` (if set) or the whole workspace.

## Publishing order

Publishing a crate that depends on another workspace crate requires the dependency to already be on `crates.io`. Order:

1. Leaf libraries first: `airsstack-core`, `airsdsp`.
2. Providers next: `provider-claude`, `provider-openrouter`.
3. Binaries last: `airsstack-cli`.

Use `cargo release` or `cargo workspaces publish` to automate version bumps + ordered publish.

## Things to AVOID

- **Per-crate `Cargo.lock`** — members must not commit their own lockfile. The workspace root owns it.
- **`[workspace]` table inside a member** — only the root has it. Cargo errors otherwise, but agents sometimes paste it in by accident.
- **Mixing `path` and `version` mismatches** — if `airsstack-core` is `0.2.0` at root but a sibling lists `version = "0.1"`, `cargo publish` fails. Inherit via `workspace = true`.
- **Duplicating dep versions** — every `serde = "1.0.X"` re-declaration is a future divergence bug. Always `{ workspace = true }`.
- **Putting `[profile.*]` in a member** — silently ignored. Edit the workspace root.
- **Globbing in `members` without an `exclude`** — `members = ["*"]` will pick up `docs/`, `.claude/`, etc. Use `crates/*` instead.

## Definition of Done (workspace-touching changes)

Add to the standard checklist from [[rust-strict-quality]]:

- `cargo metadata --format-version 1 > /dev/null` succeeds (validates the workspace graph).
- `cargo tree -d` reports no unexpected duplicate versions.
- Every new member is added to `members`, has `[lints] workspace = true`, and inherits metadata via `*.workspace = true`.
