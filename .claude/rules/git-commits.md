# Git Commit Convention

All commits in this repo follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) with a workspace-aware scope. Loads unconditionally — commits cut across every file.

## Format

```
<type>(<scope>): <short summary>

<optional body>

<optional footer(s)>
```

- **Subject line ≤ 72 chars** (target ≤ 50 where possible). Imperative mood ("add", not "added"/"adds"). No trailing period.
- **Blank line** between subject, body, and footers.
- **Body** explains the *why*, not the *what* — the diff shows the what. Wrap at 72 chars.
- **Footers**: `BREAKING CHANGE: ...`, `Refs: #123`, `Co-Authored-By: ...`, etc.

## Allowed types

| Type        | When to use                                                                 |
|-------------|------------------------------------------------------------------------------|
| `feat`      | New user-visible feature or capability                                       |
| `fix`       | Bug fix                                                                      |
| `perf`      | Performance improvement, no behavior change                                  |
| `refactor`  | Internal restructure, no behavior change, no new feature                     |
| `docs`      | Documentation only (rustdoc, README, `.claude/rules/`, `CLAUDE.md`)          |
| `test`      | Adding or fixing tests; no production code change                            |
| `build`     | Build system, `Cargo.toml`, workspace config, `rust-toolchain.toml`          |
| `ci`        | CI config (`.github/workflows/**`, scripts run by CI)                        |
| `chore`     | Maintenance not covered above (deps bump that isn't `build`, cleanup)        |
| `style`     | Formatting, whitespace, rustfmt — never logic                                |
| `revert`    | Revert of a prior commit; body MUST cite the reverted commit hash            |

If a change spans multiple types, split into separate commits. If genuinely inseparable, pick the dominant type.

## Scope — workspace-aware

Scope is **required** for any change that touches a specific crate. Format:

```
<crate-name>[/<area>]
```

- `<crate-name>` is the workspace member name verbatim (kebab-case, matches the directory under `crates/`): `airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`.
- `<area>` is optional, kebab-case, identifies the sub-module / feature / file group inside the crate. Pick something a reader will recognize without grep.

Examples:

```
fix(airsstack-core/error-handling): map provider timeouts to Retryable
feat(airsstack-cli/repl): add /clear command
perf(provider-claude/streaming): reuse SSE parser allocation across events
refactor(provider-openrouter/auth): extract bearer-token builder
docs(airsstack-core/api): document Provider trait associated types
test(airsdsp/tokenizer): add property tests for BPE round-trip
build(workspace): bump tokio to 1.42 in workspace.dependencies
ci(github): run cargo hack on PRs touching crates/**
chore(deps): cargo update — non-breaking patch bumps
```

### Choosing scope when a commit touches multiple crates

In priority order:

1. **Single crate touched** → use that crate's scope: `fix(airsstack-core/...)`.
2. **Workspace-level files only** (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, root configs) → scope = `workspace`. Example: `build(workspace): enable resolver v3`.
3. **Two or three crates touched for one logical change** → list them joined with `+` (no spaces): `refactor(airsstack-core+provider-claude): rename Provider::request to invoke`. Cap at three; beyond that, split the commit or use the broader scope below.
4. **Sweeping change across all/most crates** → scope = `workspace` and explain breadth in the body.
5. **Tooling / repo meta** (`.claude/rules/`, `.gitignore`, `.github/`, docs at repo root) → scope = `repo`. Example: `docs(repo): add rust-workspace rule`.

### When to omit scope

Only for changes with no meaningful scope: initial commit, license file, top-level README typo. Prefer `chore(repo): ...` over an unscoped commit.

## Breaking changes

Two ways, either is accepted; if both apply, use both:

1. `!` after type/scope: `feat(airsstack-core/api)!: rename Provider::send to invoke`
2. Footer: `BREAKING CHANGE: <description and migration note>`

The footer body MUST explain how a downstream consumer migrates. Breaking changes during pre-`1.0.0` are still flagged — version policy will be looser, but consumers (and `cargo` semver checks) still need the signal.

## Body content rules

- Explain motivation, constraints, alternatives considered. Skip narration of the diff.
- Reference issues/PRs in footers, not the subject: `Refs: #42`, `Closes: #42`.
- If the change is motivated by a specific upstream guideline, cite it: `Per M-SMALLER-CRATES, split provider crate into ...`.
- No emojis. No marketing language. Plain technical prose.
- No `Generated with Claude Code` style trailers unless the user explicitly asks for them. `Co-Authored-By:` is acceptable when accurate.

## Anti-patterns (rejected in review)

- `update code`, `fix stuff`, `wip` — not Conventional, not informative.
- `feat: lots of changes` — missing scope, vague subject.
- `fix(core): ...` — `core` is not a crate name in this repo; use `airsstack-core`.
- `Feat(...)` — type is lowercase.
- Subject lines over 72 chars.
- Mixing unrelated changes in one commit ("feat + drive-by refactor + rename").
- Commits that fail the [[rust-strict-quality]] Definition of Done. Every commit on `main` must be green on its own — no "fix lint" follow-up commits.

## Tooling

Recommended (configure once, enforce always):

- **`cargo install committed`** or **`commitlint`** with the workspace member list — fails CI on non-conforming subjects.
- **`cargo install git-cliff`** — generates `CHANGELOG.md` per-crate from Conventional Commits. Works well with workspace scopes.
- **Pre-commit hook** rejects non-conforming subjects locally before the commit lands.

## Quick reference

```
<type>(<scope>): <≤72-char imperative subject>

<body explaining why; wrap at 72>

BREAKING CHANGE: <only if applicable>
Refs: #<issue>
```

Scope vocabulary (kept in sync with workspace members per [[rust-workspace]]):

- `airsstack-cli`, `airsstack-core`, `provider-claude`, `provider-openrouter`, `airsdsp`
- `workspace` — root `Cargo.toml`, `Cargo.lock`, top-level Rust config
- `repo` — `.claude/`, `.github/`, `docs/`, top-level non-Rust files
