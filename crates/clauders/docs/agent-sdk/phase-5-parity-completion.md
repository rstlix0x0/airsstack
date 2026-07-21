# clauders Agent SDK — Phase 5: Pillar-2 Parity Completion (Epic)

> **Active backlog.** This epic supersedes [`phase-4-cli-parity.md`](./phase-4-cli-parity.md) as the
> live Pillar-2 workstream ledger. Phase 4 (A–H) is **closed**: WS A/B/C/E1/F1 landed (#30), WS
> D/E2/F2 were **removed** in the parity-first pivot ([`../vision-and-strategy.md`](../vision-and-strategy.md) §5),
> and only **WS G + WS H** were never built — they carry into this epic (as WS 6 + WS 7 below).
> Everything else here is **new**: either vision §7 Pillar-2 backlog never promoted to an epic, or
> **drift** in the official SDK caught by a live re-verification.

Umbrella planning doc for driving **Pillar 2 — the Agent SDK CLI runtime** (`clauders::agent`, i.e.
`CliRuntime` + `MockRuntime`) to **100% feature parity with the live official Claude Agent SDKs**
(Python `claude-agent-sdk`, TypeScript `@anthropic-ai/claude-agent-sdk`). This is a **planning
artifact**, not an SDD design spec — each workstream is brainstormed into its own dated spec under the
SDD `specs/` store, decomposed via `write-plan`, then built via `execute-plan` (the Phase 1–4 rhythm).

**As of:** 2026-07-14 · clauders at HEAD `afd1ab8` (post native-superset removal). Official surface
re-verified live 2026-07-14 against `code.claude.com/docs/en/agent-sdk/{python,typescript}` — this
epic supersedes the 2026-07-09 gap snapshot in `feature-parity.md`, which is **stale** (still shows
subagents/sessions ❌; WS E1/F1 landed) and should be refreshed to as-landed truth alongside WS 1.

---

## Scope

**In scope** — the nine workstreams that close the gap between `clauders::agent` and the live official
Agent SDK surface, on the CLI-driving runtime only (there is no native runtime anymore; the whole loop
is the `claude` subprocess, exactly like the official SDKs).

| # | Workstream | Missing surface | Origin | Effort | Depends on |
|---|---|---|---|---|---|
| 1 | **`Options` breadth** | `fallback_model`, `permission_prompt_tool_name`, `strict_mcp_config`, `add_dirs`, `settings`, `max_buffer_size`, `stderr`, `include_partial_messages`, `include_hook_events`, `user`, `max_budget_usd` | vision §7 backlog | S (×~11 knobs) | — |
| 2 | **`thinking` / `effort`** | `ThinkingConfig` + `EffortLevel` (`max_thinking_tokens` deprecated) | **drift** | S | — |
| 3 | **Hook-event edges** | `SessionStart`, `SessionEnd` | vision §7 gap #5 | S | — |
| 4 | **MCP result content kinds** | `image` / `document` / `resource` tool-result blocks | feature-parity §3 | S | — |
| 5 | **`AgentDefinition` extra fields** | `skills`, `memory`, `mcp_servers`, `initial_prompt`, `background`, `effort` | feature-parity §6 + drift | S | — |
| 6 | **Streaming input** (was WS G) | prompt as `AsyncIterable<SDKUserMessage>` | phase-4 carryover | M | — |
| 7 | **MCP elicitation** (was WS H) | `onElicitation` callback + `Elicitation` hook event | phase-4 carryover | M | **WS 6** |
| 8 | **Session management ops** (F3) | `listSessions`/`getSessionMessages`/`getSessionInfo`, `renameSession`/`tagSession`, `resumeSessionAt`, `sessionId`/`title`/`persistSession` | feature-parity §7 | M | **verify CLI support** |
| 9 | **Live-control tail** | `toggleMcpServer`, `reconnectMcpServer`, `setMcpServers`, `supportedCommands/Models/Agents`, `accountInfo`, `reinitialize`, `stopTask`, `rewindFiles` | §7 gap #3 + drift | M–L | — |

**Sequence:** 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9. **Trivia first, architectural last** (same principle
Phase 4 used). WS 1–5 are all **S**, independent, no ordering between them — bank the cheap parity
checklist wins first. Hard chains: **WS 7 after WS 6** (elicitation needs the bidirectional input
path); **WS 8 gated on a CLI-support probe** (its native backing store was removed — §WS 8). WS 9 is
last: it is the largest and lowest per-effort parity value (many control-request round-trips).

**Explicitly out of scope** (official gaps deliberately shelved per vision §7 — *not* forgotten, just
deferred until the core loop is complete):

- **`setting_sources` / `settings`-from-disk** (`CLAUDE.md`, filesystem agents/commands/settings) —
  loading disk memory fights token hygiene; revisit only on a concrete need. (Note: `settings` as an
  *inline/path passthrough* is WS 1; **`setting_sources` filesystem loading** is what stays out.)
- **CLI-feature passthroughs:** `skills`, `plugins`, `sandbox`, `betas`,
  `session_store` / `enable_file_checkpointing` — thin flag passthroughs with little bearing on the
  parity thesis.

---

## Current baseline (what is already at parity — do not rebuild)

Grounded in source at HEAD `afd1ab8`:

- **Entry points:** `agent::query(prompt, Options)` and stateful `Client<R: Runtime>` (`client.rs`).
- **In-process MCP tools:** `SdkMcpServer` / `tool()` / `SdkMcpRegistry` / JSON-RPC router (`mcp/`).
- **Hooks:** 10-variant `HookEvent` + full control-response payload (`capabilities.rs`, `hooks.rs`).
- **Permissions:** all six `PermissionMode` values — `Default`/`AcceptEdits`/`Plan`/`BypassPermissions`/`DontAsk`/`Auto`
  (`Auto`/`DontAsk` as live CLI passthrough) — plus `PermissionDecision::{Allow,Deny}` with
  `updated_input`, `interrupt`, `updated_permissions`, and the rich `PermissionContext`
  (`permissions/`).
- **System prompt:** `SystemPromptConfig::{None,Text,Preset{append,exclude_dynamic_sections}}`
  (`system_prompt.rs`).
- **Subagents:** `Options::agents: HashMap<String, AgentDefinition>` → `--agents` JSON passthrough
  (`subagents/`).
- **Sessions (core):** `SessionControl::{New, Continue{fork}, Resume{id,fork}}` + `session_dir` →
  `--continue`/`--resume <id>`/`--fork-session` (`types/session_control.rs`).
- **Structured output:** `Options::output_format: Option<OutputConfig>` (CLI passthrough).
- **Message taxonomy:** assistant/user/system/result/stream frames, cache-aware `Usage`,
  `total_cost_usd` (`message.rs`, `content.rs`).
- **Live control (partial):** `interrupt`, `set_model`, `set_permission_mode`, `mcp_status`
  (`client.rs`, `runtime/port.rs`) — the tail is WS 9.

---

## Cross-cutting constraints

Carried from Phase 4, updated for the post-removal tree:

1. **`Runtime` stays object-safe.** `runtime/port.rs:65` still asserts `runtime_is_object_safe()`.
   `RoutingRuntime` (the old `HashMap<ModelId, Arc<dyn Runtime>>` holder that forced this) is **gone**,
   so the constraint is now *soft* — but keep it: it costs nothing and keeps the trait a clean mock
   seam. **Consequence for WS 6:** no generic `run_stream<S: Stream>` on the trait; the stream is
   **boxed inside the `Prompt` type**, and `Runtime::run(&self, Prompt)` keeps its signature.
2. **Only two adapters now.** Every WS touches at most `CliRuntime` (real passthrough/plumbing) and
   `MockRuntime` (records for tests). The old "5-adapter problem" is gone — this is why WS 6 collapsed
   from **L** to **M**.
3. **Strong types over stringly config.** New surfaces are exhaustive enums / newtypes
   (`ThinkingConfig`, `EffortLevel`, richer `ToolContent`), never `Option<String>` + magic values. No
   `bool` parameters for semantic flags in public constructors.
4. ~~**Capability-gate CLI-only concepts.** The existing `Capabilities` type gates hook events against
   binary support; extend it for new hook events (WS 3) and elicitation (WS 7) rather than silently
   emitting unsupported frames.~~ — **withdrawn (Phase 1)**: the registration-time gate was a
   clauders-only superset the official SDKs do not have, and it never fired (it was conditional on a
   non-empty manifest that nothing populated). It is deleted; every registered hook is now declared
   unconditionally and the binary ignores events it does not support.
5. **Featureless, zero-warning, test-first.** The Rust Definition-of-Done binds every change
   (`cargo test --workspace --all-features`, clippy `-D warnings`, `RUSTDOCFLAGS="-D warnings"`).

---

## Workstreams

Each entry: **official surface**, **clauders today** (source-grounded), a **design sketch** (refined
during that WS's brainstorm), **acceptance**, and **open questions** the brainstorm must resolve.
Exact CLI flag/control-message names are marked *verify against `--help`/live*, not asserted from docs.

### WS 1 — `Options` breadth batch · S (×~11)

- **Official:** `fallback_model`, `permission_prompt_tool_name`, `strict_mcp_config`,
  `add_dirs`/`additionalDirectories`, `settings` (inline/path), `max_buffer_size`, `stderr` (callback),
  `include_partial_messages`, `include_hook_events`, `user`, `max_budget_usd`.
- **clauders today:** `agent::Options` has 21 fields (`options.rs:32-77`); none of the above.
- **Design sketch:** add fields + builder methods, each lowering to a CLI flag or spawn behavior in
  `CliRuntime`. `stderr` is a callback (`Arc<dyn Fn(&str) + Send + Sync>` or an `mpsc` sink into the
  existing process-io plumbing). `max_buffer_size` tunes the stdout NDJSON reader.
  `include_partial_messages` enables partial-message frames on the stream. Cluster into 1–2 specs by
  cohesion (flag-passthroughs vs. the two behavioral ones: `stderr`, `max_buffer_size`).
- **Acceptance:** each field set → correct argv/spawn effect on `CliRuntime`; colocated unit tests;
  DoD green.
- **Open questions:** does `stderr` belong as an `Options` callback or a `Client` builder hook? Is
  `include_partial_messages` a stream-shape change that ripples into `MessageStream` consumers?

### WS 2 — `thinking` / `effort` · S

- **Official:** `thinking: ThinkingConfig` (adaptive form; `max_thinking_tokens` **deprecated**) and
  `effort: EffortLevel` (`low`/`medium`/`high`/`xhigh`/`max`). New since the 2026-07-09 check.
- **clauders today:** absent on `agent::Options`. (Distinct from Pillar 1: the Messages API needs
  `thinking` as a real request field — vision §7 Pillar-1 gap #1; here it is a **CLI passthrough
  flag**, so *not* correctness-critical on the agent path — the binary owns the model constraints.)
- **Design sketch:** `Options::thinking: Option<ThinkingConfig>` + `effort: Option<EffortLevel>` as
  exhaustive enums, mapped to the CLI's thinking/effort flags (*verify names*). Reuse or share the
  Pillar-1 `ThinkingConfig`/`EffortLevel` types if they exist by the time this lands, to avoid a
  duplicate type (a common `types` home).
- **Acceptance:** set → correct flags; enums exhaustive; DoD green.
- **Open questions:** share the type with the Messages API pillar or keep an agent-local copy? Exact
  flag surface.

### WS 3 — Hook-event edges · S

- **Official:** hook events include `SessionStart` and `SessionEnd` (plus `Elicitation` → WS 7). The
  live docs also list a broader set (`Setup`, `PostTurn`, `Pre/PostAgentMessage`, …) — **verify which
  the binary actually forwards** before modeling; do not add events the CLI never emits.
- **clauders today:** `HookEvent` (`capabilities.rs:11-33`) has `PreToolUse`, `PostToolUse`,
  `PostToolUseFailure`, `UserPromptSubmit`, `Stop`, `SubagentStart`, `SubagentStop`, `PreCompact`,
  `Notification`, `PermissionRequest` — missing `SessionStart`/`SessionEnd`.
- **Design sketch:** add `SessionStart`/`SessionEnd` variants + ~~`Capabilities::supports_hook` gating~~
  (**withdrawn (Phase 1)** — that gate is deleted, see the guideline above) + demux routing in
  `runtime/cli/demux.rs`. Keep clauders' extra variants (`PostToolUseFailure`, `PermissionRequest`).
- **Acceptance:** `SessionStart`/`SessionEnd` dispatch to registered hooks; ~~capability-gated;~~ DoD
  green.
- **Open questions:** which of the newer official events (`Setup`/`PostTurn`/agent-message hooks) does
  the current binary actually forward? Scope to the verified set.

### WS 4 — MCP result content kinds · S

- **Official:** in-process tool results carry `text` / `image` / `document` / `resource` content
  blocks (`json` deprecated but accepted).
- **clauders today:** `ToolContent` (`mcp/tool.rs:17-25`) has **only** `Text`; the enum is
  `#[non_exhaustive]` but no other variant is implemented.
- **Design sketch:** add `Image { data, mime_type }`, `Document { data, mime_type }`,
  `Resource { uri, mime_type }` variants; serialize to the MCP `CallToolResult` content shape. Typed
  arg inference (TS Zod) stays out — Rust tools take `serde_json::Value` schemas (documented delta).
- **Acceptance:** a tool returning image/document/resource round-trips into the tool-result frame;
  DoD green.
- **Open questions:** validate `mime_type` against the official allowed set, or pass through?

### WS 5 — `AgentDefinition` extra fields · S

- **Official:** `AgentDefinition` = `description`, `prompt`, `tools`, `disallowedTools`, `model`,
  `maxTurns`, `permissionMode`, **plus** `skills`, `memory` (`user`/`project`/`local`), `mcpServers`,
  `initialPrompt`, `background`, `effort`.
- **clauders today:** `AgentDefinition` (`subagents/definition.rs:36-49`) models the first seven;
  missing the latter six.
- **Design sketch:** add the six fields + builder methods; they serialize into the existing `--agents`
  JSON passthrough (camelCase wire-pinned, as WS E1 established). `memory` is an exhaustive enum;
  `effort` reuses WS 2's `EffortLevel`.
- **Acceptance:** the six fields serialize into the `--agents` payload with the official key names;
  wire-pin tests; DoD green.
- **Open questions:** are `skills`/`memory`/`mcpServers` inside `AgentDefinition` in the epic's "CLI-
  feature passthrough" shelf, or in-scope here because they are cheap JSON keys on an existing
  passthrough? (Recommend: **in** — they cost only serialization and complete the type.)

### WS 6 — Streaming input · M  *(was WS G)*

- **Official:** prompt as `AsyncIterable<SDKUserMessage>` (Python) / async-iterable `prompt` +
  `streamInput()` (TS) — feed user messages into a live turn as they arrive. `SDKUserMessage` =
  `{ type:"user", content: string | ContentBlock[], uuid? }`.
- **clauders today:** `Prompt(String)` — a single value (`types/prompt.rs:8`), `From<&str>`/`From<String>`.
- **Design sketch (collapsed from the paused 5-adapter brainstorm — its retry-clone / native-dual-use
  / routing-classify tensions are void; those runtimes are gone):**
  ```rust
  pub enum Prompt {
      Single(String),
      Stream(Pin<Box<dyn Stream<Item = String> + Send>>),
  }
  ```
  `Runtime::run(&self, Prompt)` unchanged (object-safe, constraint #1). `CliRuntime` feeds each item as
  an NDJSON user message to the binary's stdin as it arrives. `MockRuntime` drains into a recorded
  `Vec<String>` (replaces the lost `PartialEq`). `From<String>`/`From<&str>` map to `Single` for
  back-compat. `interrupt()` checks between drained items.
- **Acceptance:** `Prompt::Stream` drives a multi-message turn on `CliRuntime`; `Prompt::Single`
  unchanged; `MockRuntime` records streamed inputs; deterministic test; DoD green.
- **Open questions:** stream item type — `String` vs. a structured `UserMessage` (recommend `String`,
  reconsider if elicitation/WS 7 needs richer inbound); back-compat blast radius of the `struct`→`enum`
  change (now small: only `CliRuntime` + `MockRuntime` + `query`/`Client::query` consume `Prompt`).

### WS 7 — MCP elicitation · M · after WS 6  *(was WS H)*

- **What it is:** mid-tool-call, an MCP server pauses and requests structured input from the client
  (prompt + options: text/password/select); the client collects it and returns it; the server resumes.
  Human-in-the-loop *inside* a tool call.
- **Official:** `onElicitation` callback (TS shape: `{clientName, serverName, requestId, prompt,
  options?}` → `{type:"declined"}` | `{type:"responded", value}`) + an `Elicitation` hook event.
- **clauders today:** none anywhere in `agent/`.
- **Design sketch:** an `ElicitationHandler` port (`Arc<dyn>`, object-safe like `PermissionPolicy`) on
  `Options`, invoked when an in-process MCP server emits an elicitation request; plus the `Elicitation`
  `HookEvent` variant (the WS 3 companion). Interactive collection needs WS 6's bidirectional input
  path — hence the ordering. No handler + elicitation → clean error, never a hang.

  > ⚠️ **Superseded** by `specs/2026-07-18-clauders-elicitation-and-session-ops.md` §A, which is
  > grounded on the live `claude` v2.1.209 binary rather than SDK-facing docs. This sketch is wrong in
  > four ways, kept only as the pre-grounding record: the port is **`ElicitationPolicy`**
  > (`M-CONCISE-NAMES`); the response is **`{action:"accept"|"decline"|"cancel", content?}`**, not
  > `{type:"declined"}`/`{type:"responded"}`; there are **two** hook events (`Elicitation` +
  > `ElicitationResult`); and no-policy is **`{action:"decline"}`** — the binary's own fallback — not a
  > "clean error". The port is also purely reactive (it answers elicitations from any MCP server the
  > subprocess manages), so it does **not** depend on WS 6.
- **Acceptance:** an in-process MCP tool that elicits routes to the handler; the returned value resumes
  the call; no-handler → clean error; DoD green.
- **Open questions:** in-process (`sdk_mcp_servers`) tools only at first, or external-server
  elicitation too (opaque passthrough)? Schema/type validation of the elicited value.

### WS 8 — Session management ops (F3) · M · **verify CLI support first**

- **Official:** `listSessions` / `getSessionMessages` / `getSessionInfo`, `renameSession` /
  `tagSession`, `resumeSessionAt` (resume by message UUID), `sessionId` override, `title`,
  `persistSession`.
- **clauders today:** only the `SessionControl` core (continue/resume/fork). The native
  `SessionStore::{list,most_recent}` that *could* have backed list/inspect was **removed** with
  `ApiRuntime` (vision §5) — so **there is no native store left**; F3 can only be CLI-passthrough.
- **Design sketch:** **gated on a probe** — confirm the `claude` binary exposes list/inspect/rename/tag
  as flags or control-requests. If yes: model them as `Client`/`Runtime` control ops + extend
  `SessionControl::Resume` with an optional message anchor (`resumeSessionAt`) and add
  `sessionId`/`title`/`persistSession` to `Options`. If the CLI has no such surface: F3 is *"no parity
  path on a CLI-only runtime"* (like the base preset being CLI-only by construction) — document the
  omission rather than build a dead native store.
- **Acceptance:** whichever ops the CLI supports are wired + tested; unsupported ops documented as
  CLI-limited; DoD green.
- **Open questions:** the probe result (the whole WS shape depends on it); does `resumeSessionAt` fold
  into `SessionControl::Resume { id, at: Option<MessageId>, fork }`?

### WS 9 — Live-control tail · M–L

- **Official (client / TS `Query`):** `toggleMcpServer`, `reconnectMcpServer`, `setMcpServers`
  (live MCP control); `supportedCommands` / `supportedModels` / `supportedAgents`; `accountInfo` /
  `get_server_info` / `initializationResult`; `reinitialize` / `applyFlagSettings`; `stopTask`;
  `rewindFiles` (needs file checkpointing — likely out with `enable_file_checkpointing`).
- **clauders today:** `interrupt`, `set_model`, `set_permission_mode`, `mcp_status` on `Runtime`
  (`port.rs`) + `Client` (`client.rs`).
- **Design sketch:** add methods to the `Runtime` trait (each object-safe, `&self` + control request)
  + `Client` forwarders + `CliRuntime` control-request/response frames in `runtime/cli/dispatch.rs` /
  `protocol/`. Group by cohesion: (a) live-MCP trio, (b) `supported*` introspection, (c)
  `reinitialize`/`accountInfo`, (d) `stopTask`. `rewindFiles` deferred with checkpointing unless a need
  appears.
- **Acceptance:** each op issues the correct control request and decodes the response; `MockRuntime`
  stubs; DoD green.
- **Open questions:** protocol frame shapes for each control op (*verify live*); which are Python-only
  vs TS-only and whether clauders exposes the union; is `rewindFiles` in or out (checkpointing is a
  shelved CLI feature).

---

## Deliverables & doc hygiene

- **WS 1 companion:** refresh `feature-parity.md` to as-landed truth — flip subagents/sessions to ✅,
  keep the removed-superset rows re-legended, fold in the live-2026-07-14 official surface. It is the
  gap table this epic derives from and is currently stale.
- Each WS: one dated spec under the SDD `specs/` store → `write-plan` → `execute-plan` → snapshot +
  journal capture, same as Phases 1–4.

---

## Sources

- Vision & strategy — [`../vision-and-strategy.md`](../vision-and-strategy.md) (§3 scope, §7 backlog).
- Prior epic (closed) — [`phase-4-cli-parity.md`](./phase-4-cli-parity.md).
- Gap table (to refresh) — [`feature-parity.md`](./feature-parity.md).
- Official Python SDK — <https://code.claude.com/docs/en/agent-sdk/python> (live 2026-07-14).
- Official TypeScript SDK — <https://code.claude.com/docs/en/agent-sdk/typescript> (live 2026-07-14).
