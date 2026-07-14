# clauders Agent SDK — Phase 4: Official CLI-Surface Parity (Epic)

> **Parity-first update (2026-07-13):** This epic is now scoped strictly to **Pillar 2 — the Agent
> SDK CLI runtime** ([`../vision-and-strategy.md`](../vision-and-strategy.md)). The "CLI-surface
> parity" workstreams (WS A–F) that survive are the CLI-passthrough halves and remain valid parity
> work. Any workstream half that built on the native `ApiRuntime` (native structured output, native
> permission enforcement, the auto-permission judge, the native subagent loop, the native session
> store) has been **removed** (vision §5) — treat those halves as historical, not backlog.

Umbrella planning doc for closing the *worth-building* parity gaps identified in
[`feature-parity.md`](./feature-parity.md). This is a **planning artifact**, not an SDD design
spec — each workstream below is brainstormed into its own dated spec under the SDD `specs/` store,
then decomposed into an execution plan via `write-plan`, then built via `execute-plan`. Same
structure Phases 1–3 used (one roadmap doc + N per-workstream specs).

**As of:** 2026-07-09 · clauders at HEAD `6518699` · derives from `feature-parity.md` §"Candidate
parity gaps worth closing".

---

## Scope

**In scope** — the parity gaps that serve the airsstack thesis (token efficiency, mixed routing,
long-lived workflows) or keep the permission/prompt surface current:

| WS | Feature | Effort | Depends on |
|----|---------|--------|------------|
| A | System-prompt preset + append | XS | — |
| B | Structured output on the agent layer | S | reuse `messages::structured_outputs::OutputConfig` |
| C | `dontAsk` mode + `updated_permissions` + deny-interrupt | S–M | `permissions.rs` |
| D | ~~REMOVED (vision §5):~~ `auto` permission mode (model-classified) | M | reuse `RoutingRuntime` `Classifier` infra; after C |
| E1 | Subagents — `AgentDefinition` + CLI passthrough | M | permissions, model routing |
| E2 | ~~REMOVED (vision §5):~~ Subagents — native nested loop on `ApiRuntime` | L | E1; **unblocks per-subtask downgrade** |
| F1 | Sessions — `SessionControl` + CLI passthrough | M | frame `SessionId` wiring |
| F2 | ~~REMOVED (vision §5):~~ Sessions — native conversation-history object on `ApiRuntime` | L | F1; **unblocks context pruning** |
| G | Streaming input | L | `Prompt` type + all runtime adapters |
| H | MCP elicitation | M | **after G** (needs interactive-input path) |

**Sequence:** A → B → C → D → E1 → E2 → F1 → F2 → G → H. Trivia first, architectural last. Hard chains:
**E2 after E1**, **F2 after F1**, **D after C**, **H after G**. A/B/C independent, can reorder freely.
~~REMOVED (vision §5):~~ E2 and F2 are **in this epic** (the two native slices that double as the
Scope C token-efficiency unblockers — grouped here so parity and the token north star land together).

**Explicitly out of scope** (CLI-feature passthroughs, no thesis bearing — from `feature-parity.md`):
`skills`, `plugins`, `sandbox`, `betas`, `session_store` mirroring, warm startup (`startup()`),
`reinitialize`/`applyFlagSettings`, `thinking`/`effort` knobs. **Setting sources** (`CLAUDE.md`/
filesystem settings) stays deliberately out — loading disk memory fights token hygiene; revisit only
if a concrete need appears.

---

## Cross-cutting architectural constraints

These bind every workstream and are the reason the sequencing looks the way it does. All are driven
by the Rust guideline (strong types, static dispatch, `mod.rs` export-only, object-safety).

1. ~~REMOVED (vision §5):~~ **`Runtime` must stay object-safe.** `RoutingRuntime` holds `HashMap<ModelId, Arc<dyn Runtime>>`
   and `runtime/port.rs` carries a `runtime_is_object_safe()` compile assertion. No workstream may
   add a generic method (`fn foo<T>()`) to the `Runtime` trait. Streaming input (G) therefore cannot
   introduce `run_stream<S: Stream>` on the trait — the stream must be **boxed inside the input
   type**, not a generic trait method.

2. ~~REMOVED (vision §5):~~ **Native runtimes reimplement the loop; the CLI runtime delegates.** For most workstreams the
   `CliRuntime` change is *passthrough* (serialize a config into a CLI flag / control message) while
   the native `ApiRuntime`/`OpenRouterRuntime` change is a *real implementation* (subagent spawning,
   history persistence, elicitation plumbing). Where those diverge in cost, the WS is split into a
   CLI-passthrough slice (small) and a native slice (large). The native slices are where the
   airsstack levers live (per-subtask downgrade, context pruning).

3. ~~REMOVED (vision §5):~~ **Capability-gate anything the CLI supports but a native runtime doesn't (and vice versa).** The
   existing `Capabilities` type already gates hook events. Preset system prompts, subagents, and
   sessions are CLI-native concepts; on `ApiRuntime` they either get a native implementation or are
   reported unsupported via `Capabilities` and error cleanly — never silently ignored.

4. **Strong types over stringly config.** New config surfaces are modeled as exhaustive enums /
   newtypes (e.g. `SystemPrompt`, `SessionControl`), not `Option<String>` + magic values. No `bool`
   parameters for semantic flags in public constructors; type-state builders where a lifecycle is
   ordered.

---

## Workstreams

Each entry gives the **official surface**, **clauders today**, a **design sketch** (refined during
that WS's brainstorm), **acceptance criteria**, and **open questions** the brainstorm must resolve.

### WS A — System-prompt preset + append  ·  XS

- **Official:** `system_prompt` accepts a plain string **or** a preset object
  `{type:"preset", preset:"claude_code", append?, exclude_dynamic_sections?}` (Python + TS).
- **clauders today:** `Options::system_prompt: Option<String>` (plain string only).
- **Design sketch:** replace the field with an exhaustive enum. Only one preset value exists
  (`claude_code`), so no preset-name field (YAGNI):
  ```rust
  pub enum SystemPrompt {
      None,
      Text(String),
      Preset { append: Option<String>, exclude_dynamic_sections: bool },
  }
  ```
  `CliRuntime` maps `Preset` to the binary's preset flags; `Text` to `--system-prompt`. The
  ~~REMOVED (vision §5):~~ `claude_code` preset base is a *CLI-only* concept (the native loop has no
  claude binary prompt), so on `ApiRuntime` `Preset` is capability-unsupported and errors —
  `Text`/`None` work everywhere.
- **Acceptance:**
  - `Options` carries `SystemPrompt`; existing `Option<String>` callers migrate with a `From<String>`
    / builder helper.
  - ~~REMOVED (vision §5):~~ `CliRuntime` emits the correct flags for all three variants; `ApiRuntime`
    handles `None`/`Text`, reports `Preset` unsupported via `Capabilities`.
  - Colocated unit tests per the unit-test mandate; DoD green.
- **Open questions:** exact CLI flag names for preset + `exclude_dynamic_sections`
  (verify against `--help` / e2e, not docs).

### WS B — Structured output on the agent layer  ·  S

- **Official:** `output_format` / structured outputs (JSON-schema-constrained result).
- **clauders today:** absent on `agent::Options`; **present** at the messages layer
  (`messages::structured_outputs::OutputConfig`, wired through `request.rs:178`).
- **Design sketch:** add `Options::output_format: Option<OutputConfig>` reusing the existing
  `OutputConfig` (no new type). ~~REMOVED (vision §5):~~ `ApiRuntime` threads it into the `messages`
  request it already builds. `CliRuntime` maps to the CLI `--output-format` surface.
  ~~REMOVED (vision §5):~~ `OpenRouterRuntime` gates on OpenRouter structured-output support
  (capability-report if the target model lacks it).
- **Acceptance:**
  - ~~REMOVED (vision §5):~~ `Options::output_format` set → `ApiRuntime` constrains the terminal
    result; round-trip test asserts schema-conforming output on a mock transport.
  - `CliRuntime` passthrough wired; ~~REMOVED (vision §5):~~ `OpenRouterRuntime` gates unsupported
    models cleanly.
  - DoD green.
- **Open questions:** does structured output belong on `ResultMessage` as a typed `structured_output`
  field (official has one) or stay in the free-form result string? Brainstorm decides.

### WS C — `dontAsk` mode + `updated_permissions` + deny-interrupt  ·  S–M

- **Official:** `PermissionMode` gained `dontAsk` (deny anything not pre-approved). `can_use_tool`
  may return an `interrupt` on deny and `updated_permissions` (persist allow/deny rules into settings
  scopes).
- **clauders today:** `PermissionMode` = `Default|AcceptEdits|Plan|BypassPermissions`;
  `PermissionDecision` = `Allow{updated_input}` / `Deny{message}`.
- **Design sketch:**
  - Add `PermissionMode::DontAsk`.
  - Extend deny: `Deny { message: String, interrupt: bool }`.
  - Add rule persistence: a `PermissionUpdate` value (scope + rule) returnable from a decision, plus a
    `PermissionRuleStore` port the runtime consults. **This is the meaty part** — persisting rules
    needs a store abstraction; the CLI has settings-scope files. ~~REMOVED (vision §5):~~ the native
    runtime needs an in-memory/pluggable store.
- **Acceptance:**
  - `DontAsk` denies un-preapproved tools without prompting on `CliRuntime` (live — this is the
    `canUseTool` control-request seam). ~~REMOVED (vision §5):~~ and `ApiRuntime`.
  - Deny-interrupt aborts the turn.
  - A returned `PermissionUpdate` is passed through to the CLI on `CliRuntime`.
    ~~REMOVED (vision §5):~~ honored on subsequent tool calls within the session (native).
  - DoD green.
- **Open questions:** ~~REMOVED (vision §5):~~ is `updated_permissions` persistence in scope for the
  *native* runtime now, or CLI-passthrough only (native store deferred)? If it inflates past M, split
  the store into its own WS.

### WS D — ~~REMOVED (vision §5):~~ `auto` permission mode (model-classified)  ·  M  ·  after C

> This whole workstream is the auto-permission judge, removed with the native superset (vision §5).
> The `RoutingRuntime`/`Classifier` infrastructure it depended on is also gone. Kept below for history.

- **Official:** `PermissionMode::auto` — a model classifier approves/denies each tool call.
- **clauders today:** none. **But** the classifier machinery exists: `RoutingRuntime` already ships
  `Classifier` / `RuntimeClassifier` / `ModelCard` for per-turn model routing.
- **Design sketch:** add `PermissionMode::Auto` backed by a built-in `PermissionPolicy` impl
  (`AutoPermissionPolicy`) that calls a `Classifier`-style LLM judge to decide allow/deny per tool.
  Reuse the existing classifier port rather than inventing a second one. The judge model is itself a
  cheap model (thesis-aligned: classify with a small model).
- **Acceptance:**
  - `PermissionMode::Auto` routes each `can_use_tool` through the classifier policy; allow/deny
    reflects the classifier verdict.
  - Classifier is injectable (test double); deterministic test asserts allow and deny paths.
  - DoD green.
- **Open questions:** does `Auto` compose with a user-supplied `permission_policy` (classifier as
  fallback) or replace it? Which model classifies by default?

### WS E — Subagents (`AgentDefinition`)  ·  L

- **Official:** `agents: dict[str, AgentDefinition]`. `AgentDefinition` = description, prompt, tools,
  disallowedTools, model, maxTurns, permissionMode (+ skills, memory, mcpServers, background, effort —
  the last set is CLI-feature territory).
- **clauders today:** none (only `HookEvent::SubagentStart/Stop` awareness).
- **Design sketch:** a `AgentDefinition` value + `Options::agents: HashMap<String, AgentDefinition>`.
  Split by runtime:
  - **E1 (CLI passthrough, M):** serialize `AgentDefinition` to the CLI's `agents` config; subagents
    run inside the binary. Model the core fields (description, prompt, tools, disallowed_tools, model,
    max_turns, permission_mode); defer skills/memory/mcpServers/background/effort as CLI-only extras.
  - ~~REMOVED (vision §5):~~ **E2 (native, L):** `ApiRuntime` spawns a nested agentic loop per
    subagent invocation with the sub-definition's model/prompt/tools. **This unblocks the Scope C
    "per-subtask downgrade" lever** — route a subagent to a cheaper model. Exposed as a built-in
    Task-style tool the parent loop can call.
- **Acceptance:**
  - E1: `Options::agents` set → `CliRuntime` passes definitions; subagent runs observable via
    `SubagentStart/Stop` hooks.
  - ~~REMOVED (vision §5):~~ E2: `ApiRuntime` invokes a subagent as a nested loop on the
    sub-definition's model; a downgrade test asserts the subagent ran on the cheaper model while the
    parent stayed on the advanced one.
  - DoD green.
- **Open questions:** ~~REMOVED (vision §5):~~ nested-loop re-entrancy on `ApiRuntime`; tool-name for
  the subagent-invocation tool; whether E2 shares any history plumbing with F2. (E2 is **in this
  epic** — decided.)

### WS F — Sessions (continue / resume / fork)  ·  L

- **Official:** `continue_conversation`, `resume` (id), `fork_session`, plus `listSessions` /
  `getSessionMessages` / `getSessionInfo` / `renameSession` / `tagSession` and external `sessionStore`.
- **clauders today:** `SessionId` exists on frames; no resume/fork/continue wiring.
- **Design sketch:** model session intent as an exhaustive enum on `Options`:
  ```rust
  pub enum SessionControl {
      New,
      ContinueLatest,
      Resume { id: SessionId, fork: bool },
  }
  ```
  Split by runtime:
  - **F1 (CLI passthrough, M):** map `SessionControl` to `--continue` / `--resume <id>` /
    `--fork-session`. Sessions live in the binary's store; clauders just addresses them.
  - ~~REMOVED (vision §5):~~ **F2 (native, L):** `ApiRuntime` is stateless per query today.
    Resume/continue on native requires a **conversation/history object** that persists the message
    turns and reloads them — this is the exact "multi-turn history primitive"
    `feature-parity.md`/roadmap flagged as **blocked** for context pruning. Building it here
    **unblocks context pruning** (a later Scope C slice).
  - Session **list/inspect/rename/tag** = read-only free functions over whatever store F2 defines;
    lower priority, likely a follow-on (F3) or evaluate-out for native.
- **Acceptance:**
  - F1: `ContinueLatest`/`Resume{fork}` produce the correct CLI flags; a resumed CLI session continues
    prior context (e2e-gated).
  - ~~REMOVED (vision §5):~~ F2: `ApiRuntime` persists turns to a history store and a `Resume`
    reloads them; a two-query test asserts the second query sees the first's context.
  - DoD green.
- **Open questions:** ~~REMOVED (vision §5):~~ history store shape and persistence backend (in-memory
  vs pluggable trait); read-only session list/inspect/rename/tag — F3 or evaluate-out? (F2 is **in
  this epic** — decided, it's the pruning prerequisite and belongs with the parity work.)

### WS G — Streaming input  ·  L

- **Official:** prompt as `AsyncIterable<SDKUserMessage>` (Python) / `streamInput()` (TS) — feed user
  messages into a live turn as they arrive.
- **clauders today:** `Prompt(String)` — one value per turn; `Runtime::run(&self, Prompt)`.
- **Design sketch:** **object-safety forbids a generic trait method** (constraint #1). So the stream
  is boxed inside the input type, not a new generic `Runtime` method:
  ```rust
  pub enum Prompt {
      Single(String),
      Stream(Pin<Box<dyn Stream<Item = UserMessage> + Send>>),
  }
  ```
  `Runtime::run(&self, Prompt)` keeps its signature (object-safe). Each adapter handles `Stream`:
  `CliRuntime` feeds NDJSON user messages to the binary's stdin as they arrive.
  ~~REMOVED (vision §5):~~ `ApiRuntime` accumulates the stream into request turns. Touches **all
  five adapters** (Cli, Api, OpenRouter, Routing, Mock) — of which only `CliRuntime` and
  `MockRuntime` remain.
- **Acceptance:**
  - `Prompt::Stream` drives a multi-message turn on `CliRuntime`. ~~REMOVED (vision §5):~~ and
    `ApiRuntime`.
  - `Prompt::Single` unchanged (back-compat via `From<String>`).
  - `MockRuntime` records streamed inputs; deterministic test.
  - DoD green.
- **Open questions:** `UserMessage` shape as a streamed input item (reuse the existing message type?);
  interaction with `interrupt()`; ~~REMOVED (vision §5):~~ whether `RoutingRuntime` classifies on the
  first streamed chunk.

### WS H — MCP elicitation  ·  M  ·  after G

- **What it is:** an MCP protocol feature — mid-tool-call, an MCP server pauses and requests
  additional structured input from the client (a prompt + JSON schema: a confirmation, a missing
  param, a choice). The client collects it (from the user) and returns it; the server resumes.
  Human-in-the-loop *inside* a tool call.
- **Official:** `onElicitation` callback + `mcp_elicitation` hook event.
- **clauders today:** none.
- **Design sketch:** an `ElicitationHandler` port (`Arc<dyn>`, object-safe like `PermissionPolicy`)
  on `Options`, invoked when an in-process MCP server emits an elicitation request. Interactive
  collection needs the bidirectional input path from **G** — hence the ordering. For one-shot
  `query()` with no handler, elicitation auto-fails with a clear error rather than hanging.
- **Acceptance:**
  - An in-process MCP tool that elicits gets its request routed to the `ElicitationHandler`; the
    returned value resumes the tool call.
  - No handler + elicitation → clean error, no hang.
  - DoD green.
- **Open questions:** elicitation only matters for in-process (`sdk_mcp_servers`) tools initially —
  external MCP servers are opaque passthrough; is external-server elicitation in scope? Schema
  validation of the elicited value.

---

## What this epic unblocks

~~REMOVED (vision §5):~~ this whole section describes the shelved token-efficiency north star and the
native E2/F2 slices that were meant to unblock it; both the native slices and the mixed-routing thesis
they served are gone. Kept below for history.

Beyond parity, three workstreams unblock the **blocked Scope C token-efficiency slices** the roadmap
flagged as waiting on missing primitives:

- **E2 (native subagents)** → unblocks **per-subtask downgrade** (route a subagent to a cheaper model).
- **F2 (native history object)** → unblocks **context pruning** (needs a multi-turn history to prune).

So the parity work and the airsstack token-efficiency north star converge on E2 + F2 — the two native
slices are the highest-leverage items in the epic, not merely checklist parity.

---

## Sources

- Gap analysis — [`feature-parity.md`](./feature-parity.md)
- Roadmap (Phases 1–3) — [`../agent-sdk-roadmap.md`](../agent-sdk-roadmap.md)
- Official Python SDK — <https://code.claude.com/docs/en/agent-sdk/python>
- Official TypeScript SDK — <https://code.claude.com/docs/en/agent-sdk/typescript>
