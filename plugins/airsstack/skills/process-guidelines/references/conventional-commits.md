# Conventional Commits

All commits follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) with a
workspace-aware scope.

## Format

```
<type>(<scope>): <short summary>

<optional body>

<optional footer(s)>
```

- **Subject ≤ 72 chars** (target ≤ 50). Imperative mood ("add", not "added"/"adds"). No trailing period.
- **Blank line** between subject, body, and footers.
- **Body** explains the *why*, not the *what* — the diff shows the what. Wrap at 72 chars.
- **Footers**: `BREAKING CHANGE: ...`, `Refs: #123`, `Co-Authored-By: ...`, etc.

## Allowed types

| Type | When to use |
|------|-------------|
| `feat` | New user-visible feature or capability |
| `fix` | Bug fix |
| `perf` | Performance improvement, no behavior change |
| `refactor` | Internal restructure, no behavior change, no new feature |
| `docs` | Documentation only (API docs, README, rules, config docs) |
| `test` | Adding or fixing tests; no production code change |
| `build` | Build system, manifests, workspace config, toolchain pin |
| `ci` | CI config and scripts run by CI |
| `chore` | Maintenance not covered above (cleanup, non-build dep bumps) |
| `style` | Formatting, whitespace, formatter — never logic |
| `revert` | Revert of a prior commit; body MUST cite the reverted commit hash |

If a change spans multiple types, split it. If genuinely inseparable, pick the dominant type.

## Scope — workspace-aware

Scope is **required** for any change that touches a specific package. Format:

```
<member-name>[/<area>]
```

- `<member-name>` is the workspace member's name verbatim — the directory name under your packages root.
  In a multi-crate Cargo workspace these are your crate names; in another ecosystem, your package names.
- `<area>` is optional, kebab-case: the sub-module / feature / file group inside the member. Pick
  something a reader recognizes without grep.

Examples (generic):

```
fix(core/auth): map expired tokens to a retryable error
feat(api/streaming): emit an error event on mid-stream failure
perf(core/parser): reuse the buffer allocation across calls
refactor(cli/config): extract the flag-parsing builder
docs(core/users): document the Repository associated types
test(api/handlers): add round-trip tests for the upload path
build(workspace): bump the async runtime in workspace.dependencies
ci(github): run the test matrix on PRs touching packages
chore(deps): update — non-breaking patch bumps
```

### Choosing scope across multiple packages

In priority order:

1. **Single package touched** → that package's scope: `fix(core/...)`.
2. **Workspace-level files only** (root manifest, lockfile, toolchain pin, root configs) → `workspace`.
3. **Two or three packages for one logical change** → join with `+` (no spaces): `refactor(core+api): ...`.
   Cap at three; beyond that, split or use the broader scope below.
4. **Sweeping change across all/most packages** → `workspace`, explain breadth in the body.
5. **Tooling / repo meta** (CI, ignore files, top-level docs, agent/skill config) → `repo`.

### When to omit scope

Only for changes with no meaningful scope: initial commit, license file, top-level README typo. Prefer
`chore(repo): ...` over an unscoped commit.

## Breaking changes

Two ways, either accepted; use both if both apply:

1. `!` after type/scope: `feat(api/users)!: rename create to insert`
2. Footer: `BREAKING CHANGE: <description + migration note>`

The footer MUST explain how a downstream consumer migrates. Flag breaking changes even pre-1.0 — semver
tooling and consumers still need the signal.

## Body content rules

- Explain motivation, constraints, alternatives considered. Skip narration of the diff.
- Reference issues/PRs in footers, not the subject: `Refs: #42`, `Closes: #42`.
- If a specific guideline motivates the change, cite it in the body.
- No emojis. No marketing language. Plain technical prose.
- No "generated with" trailers unless explicitly requested. `Co-Authored-By:` is fine when accurate.

## Anti-patterns (rejected in review)

- `update code`, `fix stuff`, `wip` — not Conventional, not informative.
- `feat: lots of changes` — missing scope, vague subject.
- A scope that is not a real package name — use the member name verbatim.
- `Feat(...)` — type is lowercase.
- Subject lines over 72 chars.
- Mixing unrelated changes in one commit ("feat + drive-by refactor + rename").
- A commit that fails the stack's Definition of Done. Every commit must be green on its own — no "fix
  lint" follow-up commits.
