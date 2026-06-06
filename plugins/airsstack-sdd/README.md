# airsstack-sdd

Spec-driven development for Claude Code: turn an idea into a spec, the spec into focused plans, and
the plans into reviewed implementation — three skills, one workflow.

No agents, no hooks: this plugin is pure workflow. It soft-couples to the `airsstack` plugin for
execution and degrades to guided inline work when that plugin is absent.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack-sdd@airsstack
```

Skills are namespaced `airsstack-sdd:<name>`.

## Skills

| Skill | Purpose |
| --- | --- |
| `brainstorm` | Idea → spec via structured dialogue, behind a hard gate so design is settled before code. Writes the spec to `docs/specs/`, then hands off to `write-plan`. |
| `write-plan` | Spec → implementation plan, **one objective per plan file**. Writes to `docs/plans/`. Owns the artifact lifecycle: specs are the durable record, plans are disposable scaffolding deletable once their spec is the source of truth. |
| `execute-plan` | Plan → implementation with review checkpoints. Drives the `airsstack` plugin's `orchestrate` skill when installed; degrades to guided inline execution otherwise. The user is the commit gate. |

## Artifacts

Specs and plans are written to `docs/specs/` and `docs/plans/` in your project — plain, committable
Markdown. (Adapt the location to your repo's conventions if you prefer.)

## License

Apache-2.0. See [LICENSE](./LICENSE).
