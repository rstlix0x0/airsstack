# Rust — Workspace Layout & Conventions

Workspace structure follows the official [Cargo Book ch. 14.3](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) plus modern centralization features (`workspace.package`, `workspace.dependencies`, `workspace.lints`). Cross-links: the Microsoft guidelines reference `M-SMALLER-CRATES`, the strict-quality reference (lint policy).

## Why a workspace

A Cargo workspace has one or more members under `crates/` and stays a workspace so new members can be added without restructuring. A workspace gives:

- **One `Cargo.lock`** → every member compiles against the same dep versions. No version skew between members.
- **Shared `target/`** at the workspace root → inter-crate deps build once, not per-crate. Big disk + time savings.
- **Centralized metadata, deps, and lints** via `[workspace.*]` tables → bumping `serde` or `tokio` is a one-line change.
- **Atomic refactors** across crates land in one PR.

## Root `Cargo.toml` shape

The workspace root has **no `[package]` section**; this repo's own root `Cargo.toml` is the authoritative example — `[workspace]` with `resolver = "3"` and `members = ["crates/airs-transport", "crates/clauders", "crates/openrouter-rs"]`, then `[workspace.package]` (`edition = "2024"`, `rust-version = "1.85"`, license/repository/authors, `publish = false`), `[workspace.dependencies]`, `[workspace.lints.rust]` + `[workspace.lints.clippy]`, and `[profile.release]`.

`resolver = "3"` is required for Edition 2024 and matches the latest Cargo book guidance. Keep `rust-version` in sync across all crates by inheriting it (`rust-version.workspace = true`).

Profiles (`[profile.dev]`, `[profile.release]`) are **only valid in the workspace root** — Cargo ignores them in member crates.

## Member `Cargo.toml` shape

Every member crate inherits metadata and deps from the root: `edition.workspace = true` (likewise `rust-version`, `license`, `repository`, `authors`), each dep as `{ workspace = true }`, and `[lints] workspace = true`. Members carry their own `version` — there is no workspace version key.

Rules:

- **Never hard-code a version** in a member crate if the dep is declared in `[workspace.dependencies]`. Use `{ workspace = true }`.
- **Never re-declare `[lints]`** in a member — opt in via `workspace = true` so the strict-quality reference applies uniformly.
- **Per-crate `description`** is required for crates that will be published. `name` is required and must match the directory name.
- Each publishable member has its own `README.md` (Cargo's `readme` field) — `crates.io` renders it on the crate page.

## Directory layout

```
my-project/
├── Cargo.toml              # workspace root
├── Cargo.lock              # one lockfile, committed
├── target/                 # shared build output (gitignored)
├── crates/
│   ├── my-crate/           # first member
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/lib.rs
│   └── my-other-crate/     # second member (example)
│       ├── Cargo.toml
│       ├── README.md
│       └── src/lib.rs
└── ...
```

Put all members under `crates/`. Reasons:

- Top-level stays scannable (workspace root, tooling dirs, `docs/`, `crates/`).
- `members = ["crates/*"]` glob keeps the root `Cargo.toml` short as crates are added.
- Encourages adding new crates rather than dumping modules into existing ones (`M-SMALLER-CRATES`).

## Naming convention

- Pick a name that says what the crate *is*; a short standalone name is fine. An umbrella prefix is available if a future crate reads better grouped, but is not required.
- Directory name MUST equal crate `name`. No `crates/foo-bar/` with `name = "fooBar"`.
- Crate names use kebab-case; the corresponding Rust import is snake_case (`some-lib` → `use some_lib;`).

## Inter-crate dependencies

Once a member depends on another, two valid styles exist. **Prefer the workspace-deps style** because it pins the version once:

```toml
# Member Cargo.toml — preferred
[dependencies]
some-lib = { workspace = true }
```

vs the bare path dep (acceptable for early prototyping; convert to workspace-deps before publishing):

```toml
[dependencies]
some-lib = { path = "../some-lib" }
```

For `crates.io`-publishable members, the workspace-deps form must include both `version` and `path` (Cargo uses `path` for local builds, `version` for the published crate).

**Gotcha — `path` in `[workspace.dependencies]` is relative to the workspace root, not the member.** Declare it as `some-lib = { path = "crates/some-lib" }` (from the root), *not* `../some-lib` (which is what a bare member-level path dep uses, relative to the member). Cargo resolves the inherited `path` from the directory of the file that *defines* it — the workspace root. Mixing the two up makes `cargo metadata` fail to resolve the member. Only the crates actually depended upon need an entry; a top-level crate that nothing else imports needs none.

The build/lint/test gate lives in `../SKILL.md` § Definition of Done; `cargo` commands without `-p` operate on `default-members` (if set) or the whole workspace.

## Publishing order

Publishing a crate that depends on another workspace crate requires the dependency to already be on `crates.io`. With one member there is no ordering constraint. Once multiple members exist, publish dependency-first: leaf libraries → crates that depend on them → binaries last.

Use `cargo release` or `cargo workspaces publish` to automate version bumps + ordered publish.

## Things to AVOID

- **Per-crate `Cargo.lock`** — members must not commit their own lockfile. The workspace root owns it.
- **`[workspace]` table inside a member** — only the root has it. Cargo errors otherwise, but agents sometimes paste it in by accident.
- **Mixing `path` and `version` mismatches** — if a dependency crate is `0.2.0` but a sibling lists `version = "0.1"` for it, `cargo publish` fails. Keep the workspace-dep version in sync with the dependency's actual version.
- **Member-relative `path` in `[workspace.dependencies]`** — writing `../some-lib` there (member-relative) instead of `crates/some-lib` (root-relative) makes `cargo metadata` fail to resolve. Inherited paths resolve from the workspace root.
- **Duplicating dep versions** — every `serde = "1.0.X"` re-declaration is a future divergence bug. Always `{ workspace = true }`.
- **Putting `[profile.*]` in a member** — silently ignored. Edit the workspace root.
- **Globbing in `members` without an `exclude`** — `members = ["*"]` will pick up `docs/`, tooling dirs, etc. Use `crates/*` instead.

## Definition of Done (workspace-touching changes)

Add to the standard checklist from the strict-quality reference:

- `cargo metadata --format-version 1 > /dev/null` succeeds (validates the workspace graph).
- `cargo tree -d` reports no unexpected duplicate versions.
- Every new member is added to `members`, has `[lints] workspace = true`, and inherits metadata via `*.workspace = true`.
