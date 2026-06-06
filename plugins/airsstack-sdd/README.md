# airsstack-sdd

Spec-driven development for Claude Code: turn an idea into a spec, the spec into focused
plans, and the plans into reviewed implementation — three skills, one workflow, plus a
small layout provisioner.

This plugin ships three workflow skills, one SessionStart hook, and one setup command. It
soft-couples to the `airsstack` plugin for execution and degrades to guided inline work
when that plugin is absent.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack-sdd@airsstack
```

Skills are namespaced `airsstack-sdd:<name>`.

## Skills

| Skill | Purpose |
| --- | --- |
| `brainstorm` | Idea → spec via structured dialogue, behind a hard gate so design is settled before code. Scans `rfcs/` for design input, writes the spec to `specs/`, then hands off to `write-plan`. |
| `write-plan` | Spec → implementation plan, **one objective per plan file**. Writes to `plans/`. Owns the artifact lifecycle: specs are the durable record, plans are disposable scaffolding deletable once their spec is the source of truth. |
| `execute-plan` | Plan → implementation with review checkpoints. Drives the `airsstack` plugin's `orchestrate` skill when installed; degrades to guided inline execution otherwise. The user is the commit gate. |

## Artifact layout

All artifacts live in a per-project, **git-ignored** tree:

```
.airsstack/cc/plugins/sdd/
├── rfcs/    # human-authored RFCs — design input, read-only to the plugin
├── specs/   # brainstorm writes specs
└── plans/   # write-plan writes plans
    └── _archive/
```

The whole `.airsstack/` tree is git-ignored via a single `.gitignore` line: `.airsstack/`.
Artifacts are therefore local to your working tree, not committed.

The tree and the `.gitignore` entry are provisioned three ways, all idempotent:

- automatically on session start (a plugin SessionStart hook),
- on demand with `/airsstack-sdd:setup`,
- lazily by `brainstorm` and `write-plan` immediately before their first write.

Canonical path definitions live in two mirrored authorities: prose in
`references/artifact-paths.md`, shell in `hooks/ensure-layout.sh`. They must agree.

## RFCs as input

Drop an RFC into `rfcs/` (any filename — no convention enforced). `brainstorm` auto-scans
the directory and surfaces what it finds, or you can name an RFC explicitly to load it as
primary design input. When an RFC seeds a spec, the spec header records its provenance
with a `Derived-from-RFC: rfcs/<file>` line. RFCs are read-only to the plugin — it never
creates, edits, moves, or deletes them.

## License

Apache-2.0. See [LICENSE](./LICENSE).
