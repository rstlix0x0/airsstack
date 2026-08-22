# airsstack

The execution engine for a spec-driven, review-gated development methodology, packaged as a Claude
Code plugin. It ships the agents and the orchestration skill that turn a plan into reviewed,
verified changes — plus process guidelines, project memory, and a verbosity mode.

Language-agnostic: the agents obtain their Definition-of-Done and rules from whichever
`*-guidelines` skill you have installed (e.g. `airsstack-guideline-rust`), and degrade gracefully
when none is present.

## Install

```
/plugin marketplace add rstlix0x0/airsstack
/plugin install airsstack@airsstack
```

All components are namespaced `airsstack:<name>`.

## Agents

Spawned by the `orchestrate` skill (or directly via the Agent tool). Each pins its model tier and
effort level in frontmatter.

| Agent | Model / effort | Role |
| --- | --- | --- |
| `coder` | sonnet · high | Executes one scoped task with strict TDD, runs the active guideline DoD, never commits. |
| `reviewer` | opus · high | Re-runs the DoD and reviews the diff for style + correctness + spec/plan intent. Report-only. |
| `explorer` | haiku · low | Read-only locator: returns `file:line` for "where is X". Refuses judgment. |

Agents are leaves — they never spawn other agents. Chaining lives in `orchestrate`.

## Skills

| Skill | Purpose |
| --- | --- |
| `orchestrate` | Drives `explorer → coder → reviewer → user` per task; routes findings through the orchestrator; the user is the only commit gate. |
| `process-guidelines` | Conventional Commits (workspace-aware scope), model-routing, and the agent-orchestration flow. |
| `concise` | Verbosity-reduction mode (lite / full / ultra). Clean professional terseness that persists across the session. See [Attribution](#attribution). |
| `snapshot-load` | Reads the project-local snapshot(s) and reports the rehydrated state. No-arg loads the current branch's latest; an explicit topic does a branch-agnostic topic search. |
| `snapshot-save` | Captures a conversation snapshot (session summary + key snippets) into the project-local snapshot store, with a durability gate so thin sessions write nothing. No-arg captures the whole session; an explicit topic focuses the capture and tags it. |

## Hooks

- `SessionStart` (startup / resume / clear) → nudge to run `/airsstack:snapshot-load`.
- `SessionEnd` → nudge to run `/airsstack:snapshot-save`.
- `UserPromptSubmit` → re-inject the active `concise` level each turn (persistent concise mode; no-op
  when no level is active).

The session hooks **nudge only** — you (the model) keep the selection and durability judgment.

### Concise hook runtime

The `UserPromptSubmit` hook — like every other airsl-backed hook in the suite (`enforce.lua`,
`rearm.lua`, SDD layout provisioning, the journal orientation card) — runs on
[`airsl`](../../crates/airsl), the embedded Lua runtime, and its launcher exits silently when the
`airsl` binary is not installed, so that hook's own effect disappears with no error at the point it
fires. That per-hook silence is no longer the whole story: `hooks/preflight.sh` runs on every
`SessionStart` (startup, resume, and clear) precisely to break it. It re-resolves `airsl` the same
way the other hook wrappers do and, if that resolution fails, prints a `STATUS:` / `Disabled:` /
`FIX:` block plus the install command — so a machine without it still gets one signal per session
start, even though every individual hook stays quiet. The `Disabled:` line names the four hooks
with a user-visible effect (rule enforcement, the concise tracker, SDD layout provisioning, the
journal orientation card); `rearm.lua` and the two `airsstack-plugin-dev` hooks are equally inert
without `airsl` and are left off deliberately, because naming every hook would bury the ones that
change what a session does.

Install `airsl` with `cargo install --git https://github.com/rstlix0x0/airsstack --locked airsl-cli`,
or run `plugins/airsstack/scripts/install-airsl.sh` from the repo root (`scripts/install-airsl.sh`
from inside this directory) for the same thing in one command — it also detects an
already-installed binary and reports its location. If `airsl` is installed but a hook still can't
find it, its directory is likely off the hook's PATH — set `AIRSL_BIN` to the binary's full path.

## Project snapshots

`snapshot-save` writes timestamped conversation snapshots (session summary + key snippets) to a
store **outside the repo**, at `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` (same
user-global root the `concise` hook uses), with a custom `index.md`. `<project-key>` is derived from
`git rev-parse --git-common-dir`, so **all worktrees of one repo share a single store** and snapshots
survive worktree teardown, branch churn, `target/` cleans, and `/clear`. Because it lives outside the
repo, it can never be accidentally committed.

This store is **deliberately separate from Claude's native memory tool** (`~/.claude/projects/.../`
+ `MEMORY.md`), whose store has size limits we are working around — these skills never write there,
and the index is named `index.md`, never `MEMORY.md`.

This is deliberately **local persistence, not git-shareable** — snapshots do not travel to
teammates, CI, or a fresh clone. If you need shared project knowledge, commit it as source (docs,
ADRs), not as a snapshot.

### Topic-focused snapshots

Both skills take an **optional topic** that switches their mode. Save and load are symmetric: the
saver tags a focus, the loader matches it across branches.

| Command | Mode | What it does |
| --- | --- | --- |
| `/airsstack:snapshot-save` | whole-session | Captures the session as a whole — the default "where was I" record. `topic:` left empty. |
| `/airsstack:snapshot-save streaming parser` | topic-focused | Biases the summary, snippets, and carryovers toward *streaming parser* and stamps `topic: streaming parser` (also added to the `index.md` line). |
| `/airsstack:snapshot-load` | current-branch | Loads the latest snapshot(s) for the **current branch** — branch orientation. |
| `/airsstack:snapshot-load streaming parser` | topic search | Ranks **all** snapshots (any branch) by the topic — matching the saved `topic:` key first, then the `summary`. Branch only breaks ties. |

**When to use a topic.** Reach for topic-save when one session covered a discrete thread you'll want
to resume on its own later — possibly from a different branch or session. Then topic-load pulls just
that thread back. Example: save `/airsstack:snapshot-save retry backoff` on a spike branch; weeks
later, on `main`, `/airsstack:snapshot-load retry backoff` rehydrates that thread without dragging in
the rest of the spike.

**Defaults stay simple.** No topic = today's behavior on both sides (whole-session save, current-branch
load). The session hooks still nudge the no-arg forms; add a topic only when you want a focused slice.

**Back-compat.** Snapshots saved before this feature have no `topic:` slot; topic-load falls back to
matching their `summary`, so they remain findable.

## Enforcement dispatcher

The `airsstack` plugin is the suite's single rule-enforcement dispatcher. A
`PreToolUse(Read|Edit|Write)` hook (`hooks/enforce.sh` → `enforce.lua`, on
the `airsl` runtime under `--policy confined`) reads `~/.claude/plugins/installed_plugins.json`, keeps only
airsstack-marketplace plugins (keys ending `@airsstack`), and loads each one's
root `enforcement.json`. For the file being read or written it surfaces the
matching guideline skill — once per `stack:phase` per session **per agent
context** — by injecting `additionalContext` alone; it carries no
`permissionDecision` field, so it never blocks or defers a tool call. That
field matters because a hook returning `permissionDecision: defer` was watched
returning with no `tool_result` at all — swallowing the tool call the hook
fired on — when the session is non-interactive, the tool batch is solo, and
the abort signal is not already set. Other cases still produced a result: an
interactive session or a multi-tool batch just warned and let the tool run
normally, and an already-aborted signal still produced a `tool_result`
carrying a `cancelled` denial. That was observed directly against an
installed CLI rather than read out of any build's source or documentation —
no version is claimed, and the exact conditions may shift release to release
(see the `airsl::modules::hook` module doc). Firing on `Read` puts the rule in context before the
design decision, not at the moment of writing.

Three gates must all pass:

1. **Project binding.** The plugin's registry record must resolve to this
   project's key, or be a user-scope record. A plugin installed only for repo A
   contributes nothing in repo B.
2. **Activation.** A `detect` marker must sit at or above the *edited file's*
   directory — so a `.rs` file with no `Cargo.toml` above it does not fire,
   because `cargo test` could not run there anyway. Design-phase docs anchor on
   the working directory instead: an SDD spec lives outside the repo (under
   `~/.airsstack`), so `cwd` is the only signal of which project it describes.
3. **Selection.** `match` globs are tested against the file's **repo-relative**
   path (basename, when the file is outside any repository). `**/` matches zero
   or more leading segments, so `**/Cargo.toml` covers a workspace-root
   manifest.

A `SessionStart(compact)` hook (`hooks/rearm.sh` → `rearm.lua`) clears that
session's dedup sentinels, so the pointer re-enters context after compaction
drops it. The session id survives compaction; the injected context does not.

### The `enforcement.json` convention

Any airsstack sub-plugin that enforces rules declares them in an
`enforcement.json` at its plugin root. This is the **only** sanctioned
enforcement channel — a plugin never ships its own enforcement hook.

```json
{
  "stack": "rust",
  "detect": ["Cargo.toml"],
  "match": ["**/*.rs", "**/Cargo.toml"],
  "skill": "airsstack-guideline-rust:rust-guidelines",
  "phase": ["code", "design"]
}
```

- `stack` — identifier for the rule domain (and the dedup key component).
- `detect` — repo-root marker files; the activation gate for **both** phases
  (the stack is "active" when a marker sits at the anchor directory or any
  ancestor — the edited file's directory in the code phase, `cwd` in the design
  phase).
- `match` — path globs; the code-phase trigger (matched against the edited
  file's path relative to the git toplevel, or its basename when the file is
  outside a repository).
- `skill` — the skill id the dispatcher tells the model to load.
- `phase` — which surfaces fire: `code` (editing source) and/or `design`
  (editing an SDD spec/plan while a `detect` marker is present).

Enforcement is two-tier: this hook is the **proactive** surface (it makes the
rule visible at the moment it applies); the `reviewer` agent re-running the
Definition of Done is the **retroactive** gate. The dispatcher is fail-open —
a missing registry, an absent or malformed manifest, or a missing runtime all
resolve to "do nothing," never to a blocked edit.

### Diagnosing it — `/airsstack:enforce-doctor`

The dispatcher is fail-open, which means several distinct failures all look
identical from outside: an empty registry, a plugin whose manifest never reached
the install cache, a project-binding miss, a missing `detect` marker, a glob that
did not hit, and an already-fired pointer are all just *silence*. That
indistinguishability is precisely how the framework stayed dead for weeks without
anyone noticing.

`/airsstack:enforce-doctor <path>` runs `enforce.lua --explain <path>` — the same
`resolve()` the hook drives, with a trace attached — and names the stage that
ended the run. It also reports the runtime, the resolved project key, the
repo-relative path the globs were tested against, which registry record (i.e.
`installPath`) each matching plugin resolved to, and which dedup sentinels this
context already holds. A second `--explain` of the same path in the same session
will itself show up as "already claimed" — that is the doctor observing the
sentinel its own first run set, not a real hook's.

Inside the plugin source repo it additionally diffs each plugin's source tree
against its install cache. That check is not optional decoration: the doctor
ships inside the plugin and therefore runs *from the cache*, so without it, faced
with the exact bug it exists to diagnose, it would report "zero manifests loaded"
and be unable to say why. Every source file reported `MISSING from cache` under
one plugin means that plugin's cache directory does not exist, or exists but is
empty — the two produce identical output; a mix of `MISSING`/`DIFFERS` alongside
files that are not reported means the cache dir has some content but delivery is
partial or stale. If the source directory itself cannot be read, the parity check
reports that directly (`source tree unreadable, parity unknown`) rather than
falling through to a false "repo and cache agree".

## Attribution

The `concise` skill is **inspired by the [caveman](https://github.com/juliusbrussee/caveman)
plugin** — airsstack's professional-terseness take on the same idea. The adjustment is
deliberate: where caveman compresses to caveman-speak, `concise` keeps readable prose and never
touches code, shell, error text, or careful safety-critical instructions. The persistent
level-based hook (lite / full / ultra) is airsstack's own.

## License

Apache-2.0. See [LICENSE](./LICENSE).
