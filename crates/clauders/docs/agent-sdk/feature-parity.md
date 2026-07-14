# clauders Agent SDK — Feature Parity vs the Official Claude Agent SDKs

> **Parity-first update (2026-07-13):** The 🟣 rows below marked clauders "ahead of the official SDK"
> were a **superset** with no official counterpart. Per [`../vision-and-strategy.md`](../vision-and-strategy.md)
> §5 they have been **removed** from the crate. They remain in this table (re-legended as "Removed")
> so the parity picture is honest: clauders is now a *subset-completing* client, not a superset.

Compares the `clauders` Rust Agent SDK (module `clauders::agent`) against the two **official**
Claude Agent SDKs:

- **Python** — `claude-agent-sdk` (formerly `claude-code-sdk`)
- **TypeScript** — `@anthropic-ai/claude-agent-sdk`

**As of:** 2026-07-09 · clauders at HEAD `6518699` (Phase 3 ws2 Scope C complete; parity doc merged in #29).
Official surfaces captured from `code.claude.com/docs/en/agent-sdk/{python,typescript}`.

> **Phase 4 update (2026-07-11, HEAD `6dce97b`):** the **WS A** (system-prompt preset + append, HEAD
> `6f68a10`), **WS B** (structured output) and **WS C** (permission control — `dontAsk` /
> deny-interrupt / `updated_permissions` via the CLI `PermissionPolicy`/`can_use_tool` seam) rows
> below are refreshed to as-landed. Other rows remain as of the 2026-07-09 baseline and may lag.
> The native `ApiRuntime` enforcement path referenced by the original WS C landing was removed in
> the parity-first pivot (vision §5) — see the banner above.

> **Read this first — clauders drives the CLI, same as the official SDKs.**
> The official SDKs are **thin clients that drive the `claude` Code CLI binary as a subprocess**.
> Every "runtime" they have is that one subprocess transport; they do **not** implement a native
> Messages API loop, and they are **Claude-only**. `clauders` ships that same subprocess runtime
> (`CliRuntime`) as its Agent SDK surface. It previously also shipped three native runtimes with no
> official counterpart — `ApiRuntime` (an in-process `POST /v1/messages` agentic loop),
> `OpenRouterRuntime` (native non-Claude models), and `RoutingRuntime` (LLM-classified per-turn model
> routing) — but those were a superset with no parity target, and were **removed** in the
> parity-first pivot (vision §5). So parity is a single axis now: on the *CLI-driving surface*
> clauders trails the official SDKs on session/config breadth (see the scorecard below), and the
> Pillar-1 bundled Messages API client (`clauders::Client`/`messages::`) remains a separate,
> non-Agent-SDK surface — not a parity axis against the official Agent SDKs.

---

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Full parity — equivalent capability exists |
| 🟡 | Partial — core exists, narrower than official |
| ❌ | Absent in clauders |
| 🟣 | **Removed** — clauders-only superset built ahead of the official SDK; deleted in the parity-first pivot (see [`../vision-and-strategy.md`](../vision-and-strategy.md) §5). Listed for history, not as parity gaps. |
| — | Not applicable |

---

## 1. Entry points & session control

| Capability | Python | TS | clauders | Notes |
|---|---|---|---|---|
| One-shot `query()` | ✅ | ✅ | ✅ `agent::query(prompt, Options)` | clauders returns a `MessageStream` that owns the session |
| Stateful client | ✅ `ClaudeSDKClient` | ✅ `query()` returns `Query` | ✅ `agent::Client<R>` | clauders client is generic over the `Runtime` |
| Streaming **input** (prompt as async stream) | ✅ `AsyncIterable` | ✅ `AsyncIterable<SDKUserMessage>` | ❌ | clauders `Prompt` is a single value per turn |
| Message stream out | ✅ `AsyncIterator[Message]` | ✅ `AsyncGenerator<SDKMessage>` | ✅ `MessageStream` (`Stream<Item=Result<Message>>`) | |
| Interrupt / cancel | ✅ `interrupt()` | ✅ `interrupt()` / `AbortController` | ✅ `interrupt()` | |
| Switch model mid-session | ✅ `set_model()` | ✅ `setModel()` | ✅ `set_model()` | |
| Switch permission mode mid-session | ✅ `set_permission_mode()` | ✅ `setPermissionMode()` | ✅ `set_permission_mode()` | |
| MCP server status | ✅ `get_mcp_status()` | ✅ `mcpServerStatus()` | ✅ `mcp_status()` | |
| Reconnect / toggle / set MCP servers live | ✅ | ✅ | ❌ | |
| `supportedCommands / Models / Agents`, `accountInfo`, `rewindFiles`, `stopTask`, `setMaxThinkingTokens` | ✅/partial | ✅ | ❌ | official CLI-control extras |
| Warm startup (pre-warmed subprocess) | ✅ (`startup()` / `WarmQuery`) | ✅ `startup()` | ❌ | official spawn-latency optimization |
| Live reconfigure (`reinitialize`, `applyFlagSettings`) | partial | ✅ | ❌ | |

**Verdict:** ✅ core session loop at parity; 🟡 missing streaming-input and the long tail of live-control ops (warm start, reinitialize, live MCP set).

---

## 2. Configuration surface (`Options` / `ClaudeAgentOptions`)

clauders `Options` (17 fields) vs the official surface (~40+ fields). Core is covered; the gap is
session management, filesystem-settings integration, and newer model/feature knobs.

| Option (official name) | Python | TS | clauders field | Status |
|---|---|---|---|---|
| System prompt (plain string) | ✅ | ✅ | `system_prompt: SystemPromptConfig::Text` | ✅ |
| System prompt **preset** `claude_code` + `append` | ✅ | ✅ | `SystemPromptConfig::Preset { append, exclude_dynamic_sections }` | ✅ CLI (→ `--append-system-prompt`) |
| `model` | ✅ | ✅ | `model: Option<ModelId>` | ✅ |
| `fallback_model` | ✅ | ✅ | ❌ | ❌ |
| `max_turns` | ✅ | ✅ | `max_turns: Option<u32>` | ✅ |
| `allowed_tools` | ✅ | ✅ | `allowed_tools: Vec<String>` | ✅ |
| `disallowed_tools` | ✅ | ✅ | `disallowed_tools: Vec<String>` | ✅ |
| `permission_mode` | ✅ | ✅ | `permission_mode: PermissionMode` | ✅ (see §5) |
| `can_use_tool` | ✅ | ✅ | `permission_policy: Arc<dyn PermissionPolicy>` | ✅ (see §5) |
| `permission_prompt_tool_name` | ✅ | ✅ | ❌ (uses `--permission-prompt-tool stdio`) | 🟡 |
| `mcp_servers` (external) | ✅ | ✅ | `mcp_servers: Vec<McpServerConfig>` | ✅ pass-through |
| `strict_mcp_config` | ✅ | ✅ | ❌ | ❌ |
| In-process MCP servers | ✅ (via `mcp_servers`) | ✅ | `sdk_mcp_servers: SdkMcpRegistry` | ✅ (see §3) |
| `hooks` | ✅ | ✅ | `hooks: HookRegistry` | ✅ (see §4) |
| `agents` (subagents) | ✅ | ✅ | ❌ | ❌ (see §6) |
| `cwd` | ✅ | ✅ | `cwd: Option<PathBuf>` | ✅ |
| `add_dirs` / `additionalDirectories` | ✅ | ✅ | ❌ | ❌ |
| `env` | ✅ | ✅ | `env: Vec<(String,String)>` | ✅ |
| `continue_conversation` / `continue` | ✅ | ✅ | ❌ | ❌ (see §7) |
| `resume` (session id) | ✅ | ✅ | ❌ | ❌ |
| `fork_session` | ✅ | ✅ | ❌ | ❌ |
| `setting_sources` (user/project/local) | ✅ | ✅ | ❌ | ❌ (see §9) |
| `settings` (inline / path) | ✅ | ✅ | ❌ | ❌ |
| `extra_args` | ✅ | ✅ | `executable_args: Vec<String>` (prepend) | 🟡 |
| Executable path override | ✅ `cli_path` | ✅ `pathToClaudeCodeExecutable` | `path_to_executable: Option<PathBuf>` | ✅ |
| `max_buffer_size` | ✅ | — | ❌ | ❌ |
| `stderr` callback | ✅ | ✅ | ❌ | ❌ |
| `include_partial_messages` | ✅ | ✅ | ❌ | ❌ |
| `output_format` / structured output (agent layer) | ✅ | ✅ | ✅ `Options::output_format` + `ResultMessage::structured_output` (native on Api/OpenRouter; CLI best-effort passthrough) | ✅ (WS B) |
| `thinking` / `effort` / `max_thinking_tokens` | ✅ | ✅ | ❌ | ❌ |
| `max_budget_usd` | ✅ | ✅ | ❌ | ❌ |
| `skills`, `plugins`, `sandbox`, `betas` | ✅ | ✅ | ❌ | ❌ (CLI-feature knobs) |
| `session_store` / `enable_file_checkpointing` | ✅ | ✅ | ❌ | ❌ |
| Per-request `max_tokens` | — (CLI-managed) | — | `max_tokens: MaxTokens` (default 4096) | 🟣 needed by native `ApiRuntime` — removed (vision §5) |
| Prompt-cache policy | ❌ | ❌ | `CachePolicy` (via `ApiRuntime`) | 🟣 (see §13) — removed (vision §5) |
| Min-version gate / shutdown grace | — | — | `require_min_version`, `shutdown_grace` | 🟣 process-hygiene — removed (vision §5) |

**Verdict:** ✅ on the tool/permission/mcp/hook/cwd/env core; ❌ on sessions, subagents, setting-sources,
and the newer model-feature knobs (thinking, budget, skills, plugins, sandbox).

---

## 3. In-process MCP tools

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| Define a tool | ✅ `@tool(name, desc, schema)` | ✅ `tool(name, desc, zodShape, handler)` | ✅ `tool(name, desc, json_schema, closure)` + `impl Tool` | ✅ |
| Create in-process server | ✅ `create_sdk_mcp_server(name, version, tools)` | ✅ `createSdkMcpServer({name, version, tools})` | ✅ `SdkMcpServer::builder(name).version().tool().build()` | ✅ |
| Registry of servers | implicit | implicit | ✅ `SdkMcpRegistry` | ✅ |
| Tool result content blocks | ✅ | ✅ | ✅ `ToolResult` / `ToolContent` (`text` today; `image`/`resource` `#[non_exhaustive]`) | 🟡 text-only content |
| Tool annotations (readOnly/destructive/…) | ✅ | ✅ `ToolAnnotations` | ✅ `ToolAnnotations` | ✅ |
| JSON-RPC dispatch (`tools/list`, `tools/call`) | ✅ | ✅ | ✅ `mcp::router` | ✅ |
| Input schema | JSON / type | **Zod shape** (typed inference) | raw `serde_json::Value` schema | 🟡 no compile-time arg typing |
| MCP **elicitation** (server asks user for input mid-call) | ✅ | ✅ `onElicitation` + `mcp_elicitation` hook | ❌ | ❌ no elicitation path |

**Verdict:** ✅ strong parity on in-process tools; gaps are richer result-content kinds, the
TS Zod-style typed argument inference, and MCP elicitation.

---

## 4. Hooks

clauders models a broad hook-event set and the full control-response payload.

| Aspect | Python | TS | clauders |
|---|---|---|---|
| Registration with matcher | ✅ `HookMatcher` | ✅ `HookCallbackMatcher` | ✅ `Options::hook(event, matcher, Arc<dyn Hook>)` |
| Capability-gated to binary support | — | — | 🟣 `Capabilities::supports_hook` skips unsupported events — removed (vision §5) |
| Return: block / continue / suppressOutput / systemMessage / reason | ✅ | ✅ | ✅ `HookOutput { continue_, suppress_output, decision: Block, system_message, reason }` |

**clauders `HookEvent`s:** `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`,
`Stop`, `SubagentStart`, `SubagentStop`, `PreCompact`, `Notification`, `PermissionRequest`.

The official SDKs forward whatever the CLI supports (at least PreToolUse, PostToolUse,
UserPromptSubmit, Stop, SubagentStop, PreCompact, SessionStart/End, Notification, and newer granular
tool hooks). clauders does **not** yet model `SessionStart`/`SessionEnd` or the newer
`mcp_elicitation` event; it adds `PostToolUseFailure` and `PermissionRequest`.

**Verdict:** ✅ parity on the hook mechanism and payload; 🟡 event-name set differs at the edges
(clauders missing `SessionStart`/`SessionEnd`/`mcp_elicitation`).

---

## 5. Permissions

| Aspect | Python | TS | clauders | Status |
|---|---|---|---|---|
| `default` | ✅ | ✅ | ✅ | ✅ |
| `acceptEdits` | ✅ | ✅ | ✅ | ✅ |
| `plan` | ✅ | ✅ | ✅ | ✅ |
| `bypassPermissions` | ✅ | ✅ | ✅ | ✅ |
| `dontAsk` | ✅ | ✅ | ✅ `PermissionMode::DontAsk` (CLI passthrough; native `ApiRuntime` enforcement removed, vision §5) | ✅ (WS C) |
| `auto` (model-classified) | ✅ | ✅ | ❌ | 🟡 newer mode (WS D) |
| `can_use_tool` callback | ✅ | ✅ | ✅ `PermissionPolicy::can_use_tool` | ✅ |
| Allow + rewrite input | ✅ `updated_input` | ✅ | ✅ `Allow { updated_input }` | ✅ |
| Deny + message | ✅ `message` | ✅ | ✅ `Deny { message }` | ✅ |
| Deny + `interrupt` | ✅ | ✅ | ✅ `Deny { interrupt }` + `deny_interrupt()` (aborts the native turn → `stop_reason: "permission_denied"`) | ✅ (WS C) |
| Return **permission updates** (persist allow/deny rules) | ✅ `updated_permissions` | ✅ `updatedPermissions` → settings scopes | ✅ `updated_permissions` (CLI: passthrough to the binary's settings scopes; the native in-memory `RuleStore` enforcement path was removed, vision §5) | 🟡 CLI-only now (WS C) |
| Rich request context | ✅ `ToolPermissionContext` | ✅ (toolUseID, agentID, blockedPath, decisionReason…) | ✅ `PermissionContext` (all of those fields) | ✅ |

**Verdict:** ✅ parity on the allow/deny + input-rewrite core, request context, `dontAsk`,
deny-interrupt, and `updated_permissions` — via the live CLI `PermissionPolicy`/`can_use_tool` seam
(`canUseTool` passthrough to the `claude` Code CLI, WS C). The native `ApiRuntime` enforcement path
(a `permission_engine::{RuleStore, evaluate}` gate on an in-process tool loop) was **removed** in the
parity-first pivot (vision §5) along with `ApiRuntime` itself; `PermissionMode`/`PermissionUpdate` data
types are kept, but there is no native runtime left to enforce them in-process. Remaining deltas:
`auto` (model-classified — WS D); permission-rule persistence is entirely the CLI binary's
responsibility now (settings-scope/disk persistence, spec §9).

---

## 6. Subagents

| Capability | Python | TS | clauders |
|---|---|---|---|
| Programmatic `agents` / `AgentDefinition` (description, prompt, tools, model, …) | ✅ | ✅ | ❌ |
| Awareness of subagent lifecycle | via hooks | via hooks | 🟡 `HookEvent::SubagentStart/Stop` only |

**Verdict:** ❌ clauders has no programmatic subagent definitions. (It has a *separate* multi-process
**orchestration pool** — see §12 — which is a different concept: bounded-concurrency fan-out, not
CLI subagents.)

---

## 7. Sessions

| Capability | Python | TS | clauders |
|---|---|---|---|
| `continue` most recent | ✅ | ✅ | ❌ |
| `resume` by session id | ✅ | ✅ | ❌ |
| `fork_session` | ✅ | ✅ | ❌ |
| List / inspect / rename / tag sessions | ✅ | ✅ | ❌ |
| Session id type | ✅ | ✅ | 🟡 `SessionId` exists on frames, but no resume/fork wiring |

**Verdict:** ❌ session management is the single largest missing subsystem.

---

## 8. System prompt

| Capability | Python | TS | clauders |
|---|---|---|---|
| Plain string | ✅ | ✅ | ✅ (`SystemPromptConfig::Text`) |
| Preset `claude_code` + `append` | ✅ | ✅ | ✅ CLI / 🟡 native |
| `excludeDynamicSections` / `exclude_dynamic_sections` | ✅ | ✅ | ✅ CLI (`--exclude-dynamic-system-prompt-sections`) |

**Verdict:** ✅ on the CLI runtime (HEAD `6f68a10`, WS A). `CliRuntime` lowers `Preset` to
`--append-system-prompt` (keeping the CLI's built-in `claude_code` base) plus
`--exclude-dynamic-system-prompt-sections`. The native runtimes (`ApiRuntime`/`OpenRouterRuntime`) have
no CLI base to append to, so `Preset` degrades to its `append` text only (logged warning) — the base
preset is a CLI-only capability by construction.

---

## 9. Setting sources (filesystem config)

| Capability | Python | TS | clauders |
|---|---|---|---|
| `setting_sources: [user, project, local]` | ✅ | ✅ | ❌ |
| Load `CLAUDE.md`, project agents, slash commands from disk | ✅ | ✅ | ❌ |
| Inline / path `settings` | ✅ | ✅ | ❌ |

**Verdict:** ❌ clauders does not load filesystem settings/memory/commands. Everything is programmatic
via `Options`. (Arguably a deliberate token-hygiene choice, but it is a parity gap.)

---

## 10. Message types

| Type | Python | TS | clauders | Status |
|---|---|---|---|---|
| Assistant | ✅ `AssistantMessage` | ✅ `SDKAssistantMessage` | ✅ `AssistantMessage` | ✅ |
| User | ✅ | ✅ | ✅ `UserMessage` | ✅ |
| System | ✅ | ✅ | ✅ `SystemMessage` | ✅ |
| Result | ✅ `ResultMessage` | ✅ `SDKResultMessage` | ✅ `ResultMessage` | ✅ |
| Stream/partial event | ✅ `StreamEvent` | ✅ `SDKPartialMessage` | ✅ `StreamEvent` | ✅ |
| Content blocks (text / thinking / tool_use / tool_result / server_tool_use) | ✅ | ✅ | ✅ `ContentBlock` (exhaustive, forward-compatible) | ✅ |
| `total_cost_usd` on result | ✅ | ✅ | ✅ `ResultMessage.total_cost_usd` | ✅ |
| Usage incl. **cache** counters | ✅ | ✅ | ✅ `Usage { input, output, cache_creation, cache_read }` | ✅ |
| `structured_output`, `model_usage`, `permission_denials`, rate-limit events | ✅ (rich) | ✅ | 🟡 `ResultMessage::structured_output` ✅ (WS B); `model_usage` / `permission_denials` / rate-limit ❌ | 🟡 leaner result frame |

**Verdict:** ✅ parity on the core frame taxonomy incl. cache-aware usage and cost; 🟡 official result
frame carries more diagnostic fields.

---

## 11. Runtimes / transport 🟣 — partially removed (vision §5)

This was where clauders diverged from — and exceeded — the official SDKs. The *native, non-Claude*
runtimes below have been removed; the swappable-seam abstraction and the bundled Messages API client
were not superset claims in the same sense and remain.

| Runtime | Python | TS | clauders |
|---|---|---|---|
| `claude` CLI subprocess | ✅ (the *only* runtime) | ✅ (the *only* runtime) | ✅ `CliRuntime` |
| Native `POST /v1/messages` agentic loop (in-process tool loop) | ❌ | ❌ | 🟣 `ApiRuntime<T: HttpTransport>` — removed (vision §5) |
| Native non-Claude models (DeepSeek/Kimi/Qwen via OpenRouter) | ❌ | ❌ | 🟣 `OpenRouterRuntime<T>` — removed (vision §5) |
| LLM-classified per-turn model routing across backends | ❌ | ❌ | 🟣 `RoutingRuntime` (+ `Classifier`, `RuntimeClassifier`, `ModelCard`, `RoutingSummary`) — removed (vision §5) |
| Swappable runtime seam / test double | ❌ | ❌ | 🟣 `Runtime` trait + `MockRuntime` — kept, not removed (vision §5): the object-safe seam and its test double are ordinary architecture, not a superset claim |
| Raw Messages API client bundled in the same crate | ❌ | ❌ | 🟣 `clauders::Client` / `messages::` — kept, not removed (vision §5): the Messages API is Pillar 1, core to the parity target, not a superset |

**Verdict:** 🟣 partially removed (vision §5) — clauders *was* a strict superset on execution
backends. The official SDKs cannot run a model without the `claude` binary and cannot run a
non-Claude model at all; clauders no longer runs a native or non-Claude model either — `CliRuntime`
is the only agent-execution path. The `Runtime` trait/`MockRuntime` seam and the bundled Messages API
client remain, now framed as ordinary architecture and Pillar 1 respectively, not superset claims.

---

## 12. Extension system 🟣 — removed (vision §5)

No official counterpart — the official SDKs offer no in-SDK middleware, evals, or concurrency engine.
This entire subsystem has been removed from clauders too.

| Subsystem | clauders surface |
|---|---|
| Middleware (Tower-style) | `Layer`, `Stack`, `Trace`/`TraceRuntime`, `Retry`/`RetryRuntime`, `TokenMeter`/`MeterRuntime`/`MeterHandle`/`UsageTotals`, `Tap` |
| Evals harness (runtime-agnostic) | `Case`, `EvalSuite`, `Scorer`, `Grader`, `Judge`, `Score`, `Outcome`, `Report`, `CaseReport` |
| Multi-process orchestration | `Pool`, `Limiter`, `SemaphoreLimiter` (bounded-concurrency, backpressure) |

**Verdict:** 🟣 removed (vision §5) — was entirely clauders-only. These were the "framework" ambitions
(LangChain/DSPy/DeepEval inspirations) the official SDKs leave to userland; none of it exists in
clauders now.

---

## 13. Token efficiency 🟣 — removed (vision §5)

| Capability | Python | TS | clauders |
|---|---|---|---|
| Programmable prompt-cache breakpoint policy | ❌ (CLI-managed) | ❌ (CLI-managed) | 🟣 `CachePolicy { Off, Prefix, PrefixAndConversation }` on `ApiRuntime` — removed (vision §5) |
| Cache-aware usage accounting across a tool loop | partial (surfaced in usage) | partial | ✅ summed across turns onto the terminal `Result` |
| Cost-aware routing / context pruning / per-subtask downgrade | ❌ | ❌ | 🚧 planned (later Scope C slices) |

**Verdict:** 🟣 removed (vision §5) — clauders *used to* expose prompt caching as a first-class,
programmable SDK surface on its native runtime; that native runtime is gone. The official SDKs
delegate all caching to the CLI and never surface a policy knob, and clauders now matches that.

---

## Overall scorecard

| Area | clauders vs official |
|---|---|
| One-shot + stateful entry points | ✅ parity |
| In-process MCP tools | ✅ parity (minus Zod typing / rich content) |
| Hooks | ✅ parity (event set differs at edges) |
| Permissions core (allow/deny/rewrite/context) | ✅ parity |
| `dontAsk` + deny-interrupt + `updated_permissions` (CLI `can_use_tool` seam) | ✅ parity (WS C; native `ApiRuntime` enforcement removed, vision §5; `auto` still behind → WS D) |
| Structured output (`output_format` + typed result) | ✅ parity (WS B; CLI passthrough best-effort; native-on-`ApiRuntime` path removed, vision §5) |
| Message taxonomy incl. cache usage + cost | ✅ parity |
| Config breadth | 🟡 core covered, ~25 newer knobs missing |
| **Subagents** (`agents`/`AgentDefinition`) | ❌ behind |
| **Sessions** (continue/resume/fork/list) | ❌ behind |
| **Setting sources** (filesystem config/CLAUDE.md) | ❌ behind |
| System-prompt preset + append | ✅ parity (WS A; ✅ CLI) |
| Streaming input, live MCP control, partial messages, warm start, MCP elicitation | ❌ behind |
| **Native multi-provider runtimes** (Api/OpenRouter/Routing) | 🟣 ahead — removed (vision §5) |
| **Prompt-cache policy + token efficiency** | 🟣 ahead — removed (vision §5) |
| **Middleware / evals / orchestration pool** | 🟣 ahead — removed (vision §5) |
| **Bundled raw Messages API client** | 🟣 kept, not removed (vision §5) — reclassified as Pillar 1, not a superset |

**One-line summary:** clauders is at parity on the *CLI-driving agent core* (query/client, tools,
hooks, permissions, system prompt, messages), and trails on *session management, subagents, and
filesystem-settings integration*. The native multi-provider runtimes, prompt-cache policy, and
middleware/evals/orchestration rows that used to read "ahead" were a superset with no official
counterpart and were **removed** in the parity-first pivot (vision §5); clauders is now a
subset-completing parity client, not a superset.

---

## Candidate parity gaps worth closing (not commitments)

Ranked by leverage for the airsstack mission, not by official-checklist completeness:

1. **Subagents (`AgentDefinition`)** — highest-value missing primitive; also unblocks the blocked
   Scope C "per-subtask downgrade" slice (route a subagent to a cheaper model).
2. **Sessions (continue / resume / fork)** — the largest missing subsystem; needed for any long-lived
   or resumable workflow, and a prerequisite for context-pruning experiments.
3. ~~**System-prompt preset + append**~~ — **landed (WS A, HEAD `6f68a10`)**: `SystemPromptConfig::Preset`
   lowers to `--append-system-prompt` on the CLI (keeping the built-in `claude_code` base); native
   runtimes degrade to append-only text (base is CLI-only by construction).
4. ~~**`dontAsk` permission mode + `updated_permissions` + deny-interrupt**~~ — **landed (WS C, HEAD
   `6dce97b`)** via the CLI `PermissionPolicy`/`can_use_tool` seam (`canUseTool` passthrough to the
   `claude` Code CLI). The original landing also referenced a native `ApiRuntime` enforcement path
   (a `permission_engine` gate); that path is **removed** along with `ApiRuntime` itself (vision §5).
   Only **`auto`** (model-classified — WS D) remains; permission-rule persistence is entirely the
   CLI binary's responsibility (settings-scope/disk persistence, spec §9).
5. **Streaming input** — enables interactive multi-turn feeds into a live session.
6. **Setting sources** — evaluate deliberately: loading `CLAUDE.md`/settings fights token hygiene;
   may stay intentionally out of scope.

Explicitly *not* gaps to chase: `skills`, `plugins`, `sandbox`, `betas`, session-store mirroring —
CLI-feature passthroughs with little bearing on the Rust SDK's thesis.

---

## Methodology & caveats

- **clauders side** — read directly from source at HEAD `6518699` (`crates/clauders/src/agent/`:
  `options.rs`, `runtime/port.rs`, `client.rs`, `permissions.rs`, `hooks.rs`, `capabilities.rs`,
  `message.rs`, `content.rs`, `mcp/`, `runtime/api/cache.rs`, and the `agent/mod.rs` re-export set).
  Authoritative.
- **Official side** — fetched 2026-07-09 from `code.claude.com/docs/en/agent-sdk/{python,typescript}`
  (redirected from `docs.claude.com`). The official SDKs iterate quickly; exact option keys,
  permission modes, and hook-event names drift between releases. Re-verify against the live reference
  before treating any single ❌ as a hard commitment.
- Parity marks judge *capability*, not wire/name identity. clauders is idiomatic Rust (builders,
  trait objects, exhaustive enums), so equivalent features carry Rust-shaped names.

## Sources

- TypeScript SDK reference — <https://code.claude.com/docs/en/agent-sdk/typescript>
- Python SDK reference — <https://code.claude.com/docs/en/agent-sdk/python>
- clauders roadmap — [`../agent-sdk-roadmap.md`](../agent-sdk-roadmap.md)
