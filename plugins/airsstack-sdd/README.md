# airsstack-sdd

Spec-driven development workflow for Claude Code: `brainstorm` → `write-plan` → `execute-plan`.

`execute-plan` drives the `airsstack` plugin's `orchestrate` skill when that plugin is installed, and
degrades to guided inline execution otherwise.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack-sdd@airsstack
```

Skills are namespaced `airsstack-sdd:<name>`. Specs and plans are written to `docs/specs/` and
`docs/plans/` in your project.

## License

Apache-2.0. See [LICENSE](./LICENSE).
