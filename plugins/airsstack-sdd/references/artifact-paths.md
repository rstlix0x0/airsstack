# SDD Artifact Paths

Canonical location and naming for every artifact the airsstack-sdd workflow reads or
writes. This file is the **single source of truth (prose side)** for these paths. The
shell side is mirrored in `../hooks/ensure-layout.sh` (the `SDD_ROOT` constant); the two
MUST agree. Change one, change the other.

## Base

All SDD artifacts live under a per-project, git-ignored root:

```
.airsstack/cc/plugins/sdd/
```

The whole `.airsstack/` tree is git-ignored (one `.gitignore` line: `.airsstack/`).
Artifacts are therefore local to the working tree, not committed to git.

## Directories

| Artifact | Directory | Written by | Naming |
| --- | --- | --- | --- |
| RFC | `.airsstack/cc/plugins/sdd/rfcs/` | human (external) — read-only to the plugin | any filename |
| Spec | `.airsstack/cc/plugins/sdd/specs/` | `brainstorm` | `YYYY-MM-DD-<topic>.md` |
| Plan | `.airsstack/cc/plugins/sdd/plans/` | `write-plan` | `YYYY-MM-DD-<topic>.md` |
| Archived plan | `.airsstack/cc/plugins/sdd/plans/_archive/` | `write-plan` lifecycle | `YYYY-MM-DD-<topic>.md` |

## Provisioning

The tree is created by `../hooks/ensure-layout.sh`, invoked three ways: the SessionStart
hook, the `/airsstack-sdd:setup` command, and lazy-create inside the writing skills
immediately before their first write. All three are idempotent — a skill must still
ensure its target directory exists before writing, never assuming a wrapper ran.
