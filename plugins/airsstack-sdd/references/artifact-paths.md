# SDD Artifact Paths

Canonical location and naming for every artifact the airsstack-sdd workflow reads or
writes. This file is the **single source of truth (prose side)** for these paths. The
shell side is mirrored in `../hooks/ensure-layout.sh` (the root constants and the key
resolution); the two MUST agree. Change one, change the other.

## Base — two roots, split by artifact type

SDD artifacts live under **two roots**, chosen by what the artifact is:

- **Worktree-local** (transient input): `.airsstack/cc/plugins/sdd/` — holds `rfcs/`
  only. It sits under the git-ignored `.airsstack/` tree (one `.gitignore` line:
  `.airsstack/`) and is resolved relative to the current working directory, so it is
  local to the working tree and not shared across worktrees.
- **HOME-global** (durable derived output): `${AIRSSTACK_HOME:-~/.airsstack}/cc/plugins/sdd/<key>/`
  — holds `specs/` and `plans/`. It is outside any repo, so it is never committed; it is
  shared across every worktree of one repo and survives worktree teardown. This is the
  same root the snapshot and memory stores use.

`<key>` is a stable per-repo project key. It resolves from `git rev-parse
--git-common-dir` so every linked worktree collapses to the main repo's key (no
per-worktree fragmentation); with no git it falls back to hashing the cwd. The path is
canonicalized with `pwd -P` (resolving symlinks so every worktree maps to one key) and
the human-readable component is sanitized so the key remains safe as a directory name. It
is the same key scheme the snapshot store uses. The shell side computes it as:

```sh
if common_dir=$(git rev-parse --git-common-dir 2>/dev/null); then
  abs=$(cd "$(dirname "$common_dir")" 2>/dev/null && pwd -P)/$(basename "$common_dir")
  base=$(basename "$(dirname "$abs")")
else
  abs=$(pwd -P)
  base=$(basename "$abs")
fi
# Sanitize the human-readable component; hash8 (from the full path) keeps keys unique.
base=$(printf '%s' "$base" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')
hash8=$(printf '%s' "$abs" | shasum | cut -c1-8)
key="${base}-${hash8}"   # e.g. airsstack-3f9a2c1b
```

## Directories

| Artifact | Directory | Written by | Naming |
| --- | --- | --- | --- |
| RFC | `.airsstack/cc/plugins/sdd/rfcs/` (worktree-local) | human (external) — read-only to the plugin | any filename |
| Spec | `${AIRSSTACK_HOME:-~/.airsstack}/cc/plugins/sdd/<key>/specs/` | `brainstorm` | `YYYY-MM-DD-<topic>.md` |
| Plan | `${AIRSSTACK_HOME:-~/.airsstack}/cc/plugins/sdd/<key>/plans/` | `write-plan` | `YYYY-MM-DD-<topic>.md` |
| Archived plan | `${AIRSSTACK_HOME:-~/.airsstack}/cc/plugins/sdd/<key>/plans/_archive/` | `write-plan` lifecycle | `YYYY-MM-DD-<topic>.md` |

## Provisioning

The tree is created by `../hooks/ensure-layout.sh`, invoked three ways: the SessionStart
hook, the `/airsstack-sdd:setup` command, and lazy-create inside the writing skills
immediately before their first write. All three are idempotent — a skill must still
ensure its target directory exists before writing, never assuming a wrapper ran. Only the
worktree-local root gets a `.gitignore` line; the HOME-global root is outside any repo and
needs none.
