# clauders Agent SDK — Feature Parity vs the Official Claude Agent SDKs

> **Parity-first update (2026-07-13):** The 🟣 rows below marked clauders "ahead of the official SDK"
> were a **superset** with no official counterpart. Per [`../vision-and-strategy.md`](../vision-and-strategy.md)
> §5 they have been **removed** from the crate. They remain in this table (re-legended as "Removed")
> so the parity picture is honest: clauders is now a *subset-completing* client, not a superset.

Compares the `clauders` Rust Agent SDK (module `clauders::agent`) against the two **official**
Claude Agent SDKs:

- **Python** — `claude-agent-sdk` (formerly `claude-code-sdk`)
- **TypeScript** — `@anthropic-ai/claude-agent-sdk`

**As of:** 2026-07-17 · Phase 4 (WS A–F) + Phase 5 WS 1 (Options breadth) landed on the parity-first,
**CLI-only** tree. Grounded on the live `claude` Code binary **v2.1.209** and the live official SDK
references at `code.claude.com/docs/en/agent-sdk/{python,typescript}`.

> **The tree is CLI-only now.** After the parity-first pivot (vision §5) the native runtimes —
> `ApiRuntime` (in-process `POST /v1/messages` loop), `OpenRouterRuntime`, `RoutingRuntime` — and their
> enforcement machinery (the `permission_engine` `RuleStore`, the model-judge `PermissionJudge`, the
> filesystem `SessionStore`/`list_sessions`, `CachePolicy`, the middleware/evals/orchestration tiers)
> were **all removed**. `CliRuntime` is the single agent-execution path, exactly like the official SDKs.
> Everything WS D/E/F and WS 1 added therefore lands as **CLI passthrough** — data forwarded to the
> binary's control protocol, or flags lowered in `build_argv` — never native in-process enforcement.

> **Read this first — clauders drives the CLI, same as the official SDKs.**
> The official SDKs are **thin clients that drive the `claude` Code CLI binary as a subprocess**.
> Every "runtime" they have is that one subprocess transport; they do **not** implement a native
> Messages API loop, and they are **Claude-only**. `clauders` ships that same subprocess runtime
> (`CliRuntime`) as its Agent SDK surface — and, after the pivot, *only* that. Parity is a single axis:
> on the *CLI-driving surface* clauders is now at parity on the session/config/subagent breadth it used
> to trail on (see the scorecard), with streaming input, live MCP control, warm start, and MCP
> elicitation the remaining gaps. The Pillar-1 bundled Messages API client (`clauders::Client` /
> `messages::`) is a separate, non-Agent-SDK surface — not a parity axis against the official Agent SDKs.

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
| Streaming **input** (prompt as async stream) | ✅ `AsyncIterable` | ✅ `AsyncIterable<SDKUserMessage>` | ❌ | clauders `Prompt` is a single value per turn (WS G, next) |
| Message stream out | ✅ `AsyncIterator[Message]` | ✅ `AsyncGenerator<SDKMessage>` | ✅ `MessageStream` (`Stream<Item=Result<Message>>`) | |
| Interrupt / cancel | ✅ `interrupt()` | ✅ `interrupt()` / `AbortController` | ✅ `interrupt()` | |
| Switch model mid-session | ✅ `set_model()` | ✅ `setModel()` | ✅ `set_model()` | |
| Switch permission mode mid-session | ✅ `set_permission_mode()` | ✅ `setPermissionMode()` | ✅ `set_permission_mode()` | |
| MCP server status | ✅ `get_mcp_status()` | ✅ `mcpServerStatus()` | ✅ `mcp_status()` | |
| Reconnect / toggle / set MCP servers live | ✅ | ✅ | ❌ | |
| `supportedCommands / Models / Agents`, `accountInfo`, `rewindFiles`, `stopTask`, `setMaxThinkingTokens` | ✅/partial | ✅ | ❌ | official CLI-control extras |
| Warm startup (pre-warmed subprocess) | ✅ (`startup()` / `WarmQuery`) | ✅ `startup()` | ❌ | official spawn-latency optimization |
| Live reconfigure (`reinitialize`, `applyFlagSettings`) | partial | ✅ | ❌ | |

**Verdict:** ✅ core session loop at parity; 🟡 missing streaming-input (WS G) and the long tail of
live-control ops (warm start, reinitialize, live MCP set).

---

## 2. Configuration surface (`Options` / `ClaudeAgentOptions`)

clauders `Options` (≈31 fields) vs the official surface (~40+ fields). The core plus the WS 1 breadth
knobs are covered; the residual gap is `setting_sources`, the newer model/feature knobs (thinking,
skills, plugins, sandbox), and the live-control long tail.

| Option (official name) | Python | TS | clauders field | Status |
|---|---|---|---|---|
| System prompt (plain string) | ✅ | ✅ | `system_prompt: SystemPromptConfig::Text` | ✅ |
| System prompt **preset** `claude_code` + `append` | ✅ | ✅ | `SystemPromptConfig::Preset { append, exclude_dynamic_sections }` | ✅ CLI (→ `--append-system-prompt`) |
| `model` | ✅ | ✅ | `model: Option<ModelId>` | ✅ |
| `fallback_model` | ✅ | ✅ | `fallback_model: Option<ModelId>` (→ `--fallback-model`) | ✅ (WS 1) |
| `max_turns` | ✅ | ✅ | `max_turns: Option<u32>` | ✅ |
| `allowed_tools` | ✅ | ✅ | `allowed_tools: Vec<String>` | ✅ |
| `disallowed_tools` | ✅ | ✅ | `disallowed_tools: Vec<String>` | ✅ |
| `permission_mode` | ✅ | ✅ | `permission_mode: PermissionMode` | ✅ (see §5) |
| `can_use_tool` | ✅ | ✅ | `permission_policy: Arc<dyn PermissionPolicy>` | ✅ (see §5) |
| `permission_prompt_tool_name` | ✅ | ✅ | `permission_prompt_tool_name: Option<String>` (→ `--permission-prompt-tool <name>`, overrides the `stdio` sentinel) | ✅ (WS 1) |
| `mcp_servers` (external) | ✅ | ✅ | `mcp_servers: Vec<McpServerConfig>` | ✅ pass-through |
| `strict_mcp_config` | ✅ | ✅ | `strict_mcp_config: bool` (→ presence `--strict-mcp-config`) | ✅ (WS 1) |
| In-process MCP servers | ✅ (via `mcp_servers`) | ✅ | `sdk_mcp_servers: SdkMcpRegistry` | ✅ (see §3) |
| `hooks` | ✅ | ✅ | `hooks: HookRegistry` | ✅ (see §4) |
| `agents` (subagents) | ✅ | ✅ | `agents: HashMap<String, AgentDefinition>` (→ `--agents` JSON) | ✅ (WS E; see §6) |
| `cwd` | ✅ | ✅ | `cwd: Option<PathBuf>` | ✅ |
| `add_dirs` / `additionalDirectories` | ✅ | ✅ | `add_dirs: Vec<PathBuf>` (→ variadic `--add-dir`) | ✅ (WS 1) |
| `env` | ✅ | ✅ | `env: Vec<(String,String)>` | ✅ |
| `continue_conversation` / `continue` | ✅ | ✅ | `session: SessionControl::Continue` (→ `--continue`) | ✅ (WS F; see §7) |
| `resume` (session id) | ✅ | ✅ | `session: SessionControl::Resume { id }` (→ `--resume <id>`) | ✅ (WS F) |
| `fork_session` | ✅ | ✅ | `session: SessionControl::Resume { fork: true }` (→ `--fork-session`) | ✅ (WS F) |
| `setting_sources` (user/project/local) | ✅ | ✅ | ❌ | ❌ (see §9) |
| `settings` (inline / path) | ✅ | ✅ | `settings: Option<SettingsSource>` (→ `--settings <path\|json>`) | ✅ (WS 1) |
| `extra_args` | ✅ | ✅ | `executable_args: Vec<String>` (prepend) | 🟡 |
| Executable path override | ✅ `cli_path` | ✅ `pathToClaudeCodeExecutable` | `path_to_executable: Option<PathBuf>` | ✅ |
| `max_buffer_size` | ✅ | — | `max_buffer_size: Option<NonZeroUsize>` (SDK-side stdout line cap → `Protocol` error; no CLI flag) | ✅ (WS 1) |
| `stderr` callback | ✅ | ✅ | `stderr: Option<Arc<dyn Fn(&str)>>` (SDK-side per-chunk callback; augments capture; no CLI flag) | ✅ (WS 1) |
| `include_partial_messages` | ✅ | ✅ | `include_partial_messages: bool` (→ presence `--include-partial-messages`) | ✅ (WS 1) |
| `include_hook_events` (hook-lifecycle frames) | ✅ | ✅ | `include_hook_events: bool` (→ presence `--include-hook-events`; unknown frames caught as `Message::Other`) | ✅ (WS 1) |
| `user` | ✅ | ✅ | `user: Option<String>` (inert — API-shape parity; no CLI flag lowered) | 🟡 |
| `output_format` / structured output (agent layer) | ✅ | ✅ | `output_format` + `ResultMessage::structured_output` (CLI best-effort passthrough) | ✅ (WS B) |
| `thinking` / `effort` / `max_thinking_tokens` | ✅ | ✅ | ❌ | ❌ |
| `max_budget_usd` | ✅ | ✅ | `max_budget_usd: Option<BudgetUsd>` (→ `--max-budget-usd`) | ✅ (WS 1) |
| `skills`, `plugins`, `sandbox`, `betas` | ✅ | ✅ | ❌ | ❌ (CLI-feature knobs) |
| `session_store` / `enable_file_checkpointing` | ✅ | ✅ | ❌ | ❌ (native `SessionStore` removed, vision §5) |
| Per-request `max_tokens` | — (CLI-managed) | — | `max_tokens: MaxTokens` (default 4096) | 🟡 field present but **inert** — its native `ApiRuntime` consumer was removed (vision §5) |
| Min-version gate / shutdown grace | — | — | `require_min_version`, `shutdown_grace` | 🟣 clauders-only process-hygiene on `CliRuntime` — **kept** (active), no official counterpart |
| Prompt-cache policy | ❌ | ❌ | `CachePolicy` (via `ApiRuntime`) | 🟣 removed (vision §5) — see §13 |

**Verdict:** ✅ on the tool/permission/mcp/hook/cwd/env core **and** the WS 1 breadth (fallback,
strict-mcp, add-dirs, settings, budget, partial-messages, hook-events, prompt-tool override, stderr,
max-buffer) **and** sessions + subagents; ❌ on `setting_sources` and the newer model-feature knobs
(thinking, skills, plugins, sandbox).

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
| Hook-lifecycle observability frames | ✅ (`includeHookEvents`) | ✅ | ✅ `Options::include_hook_events` → `--include-hook-events`; frames surface as `Message::Other` (WS 1) |

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
| `dontAsk` | ✅ | ✅ | ✅ `PermissionMode::DontAsk` (forwarded to the binary's `set_permission_mode`) | ✅ (WS C) |
| `auto` (model-classified) | ✅ | ✅ | ✅ `PermissionMode::Auto` (forwarded to the binary; native model-judge removed, vision §5) | ✅ (WS D) |
| `can_use_tool` callback | ✅ | ✅ | ✅ `PermissionPolicy::can_use_tool` | ✅ |
| Allow + rewrite input | ✅ `updated_input` | ✅ | ✅ `Allow { updated_input }` | ✅ |
| Deny + message | ✅ `message` | ✅ | ✅ `Deny { message }` | ✅ |
| Deny + `interrupt` | ✅ | ✅ | ✅ `Deny { interrupt }` + `deny_interrupt()` | ✅ (WS C) |
| Return **permission updates** (persist allow/deny rules) | ✅ `updated_permissions` | ✅ `updatedPermissions` → settings scopes | ✅ `updated_permissions` (passthrough to the binary's settings scopes) | 🟡 CLI-only (WS C) |
| Rich request context | ✅ `ToolPermissionContext` | ✅ (toolUseID, agentID, blockedPath, decisionReason…) | ✅ `PermissionContext` (all of those fields) | ✅ |

**Verdict:** ✅ parity on the full mode set (incl. `dontAsk` and `auto`), the allow/deny + input-rewrite
core, deny-interrupt, request context, and `updated_permissions` — all via the live CLI
`PermissionPolicy`/`can_use_tool` seam (`canUseTool` passthrough to the `claude` Code CLI). The native
enforcement surfaces built during WS C/D — the `permission_engine::{RuleStore, evaluate}` gate and the
model-judge (`PermissionJudge`/`RuntimeJudge`/`JudgeRubric`) — were **removed** with `ApiRuntime`
(vision §5); the `PermissionMode`/`PermissionUpdate` data types are kept and forwarded verbatim, but
there is no native runtime left to enforce them in-process. Permission-rule persistence is entirely the
CLI binary's responsibility (settings-scope/disk persistence).

---

## 6. Subagents

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| Programmatic `agents` / `AgentDefinition` (description, prompt, tools, model) | ✅ | ✅ | ✅ `Options::agents: HashMap<String, AgentDefinition>` → `--agents` JSON (WS E) | ✅ |
| Awareness of subagent lifecycle | via hooks | via hooks | ✅ `HookEvent::SubagentStart/Stop` | ✅ |

**Verdict:** ✅ clauders has programmatic subagent definitions, lowered to the binary's `--agents` JSON
passthrough (WS E). The native nested-subagent loop that WS E also landed on `ApiRuntime` was removed
with that runtime (vision §5), so subagents are a CLI-passthrough capability now — which is exactly what
the official SDKs are.

---

## 7. Sessions

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| `continue` most recent | ✅ | ✅ | ✅ `SessionControl::Continue` → `--continue` (WS F) | ✅ |
| `resume` by session id | ✅ | ✅ | ✅ `SessionControl::Resume { id }` → `--resume <id>` (WS F) | ✅ |
| `fork_session` | ✅ | ✅ | ✅ `SessionControl::Resume { fork: true }` → `--fork-session` (WS F) | ✅ |
| List / inspect / rename / tag sessions | ✅ | ✅ | ❌ (native filesystem `SessionStore`/`list_sessions` removed, vision §5) | ❌ |
| Session id type | ✅ | ✅ | ✅ `SessionId` on frames + `SessionControl` resume/fork wiring | ✅ |

**Verdict:** ✅ parity on continue/resume/fork via `SessionControl` → CLI flags (WS F). The native
filesystem history store (`SessionStore`, `list_sessions`) that WS F also landed on `ApiRuntime` was
removed with that runtime (vision §5); session listing/inspection is now the CLI binary's job, and the
official SDKs don't expose it either. Remaining ❌: programmatic list/rename/tag.

---

## 8. System prompt

| Capability | Python | TS | clauders |
|---|---|---|---|
| Plain string | ✅ | ✅ | ✅ (`SystemPromptConfig::Text`) |
| Preset `claude_code` + `append` | ✅ | ✅ | ✅ (`SystemPromptConfig::Preset`) |
| `excludeDynamicSections` / `exclude_dynamic_sections` | ✅ | ✅ | ✅ CLI (`--exclude-dynamic-system-prompt-sections`) |

**Verdict:** ✅ parity on the CLI runtime (WS A). `CliRuntime` lowers `Preset` to `--append-system-prompt`
(keeping the CLI's built-in `claude_code` base) plus `--exclude-dynamic-system-prompt-sections`. (The
native-runtime append-only degradation noted in earlier revisions is moot — those runtimes were removed,
vision §5; the CLI base preset is the only path.)

---

## 9. Setting sources (filesystem config)

| Capability | Python | TS | clauders |
|---|---|---|---|
| `setting_sources: [user, project, local]` | ✅ | ✅ | ❌ |
| Load `CLAUDE.md`, project agents, slash commands from disk | ✅ | ✅ | ❌ |
| Inline / path `settings` | ✅ | ✅ | ✅ `settings: SettingsSource` → `--settings <path\|json>` (WS 1) |

**Verdict:** 🟡 the inline/path `settings` knob landed (WS 1, `--settings`), but clauders still does not
enumerate `setting_sources` or load `CLAUDE.md`/project agents/slash commands from disk. Everything else
is programmatic via `Options`. (Arguably a deliberate token-hygiene choice, but `setting_sources` remains
a parity gap.)

---

## 10. Message types

| Type | Python | TS | clauders | Status |
|---|---|---|---|---|
| Assistant | ✅ `AssistantMessage` | ✅ `SDKAssistantMessage` | ✅ `AssistantMessage` | ✅ |
| User | ✅ | ✅ | ✅ `UserMessage` | ✅ |
| System | ✅ | ✅ | ✅ `SystemMessage` | ✅ |
| Result | ✅ `ResultMessage` | ✅ `SDKResultMessage` | ✅ `ResultMessage` | ✅ |
| Stream/partial event | ✅ `StreamEvent` | ✅ `SDKPartialMessage` | ✅ `StreamEvent` | ✅ |
| Unknown / forward-compat frame | — | — | ✅ `Message::Other(Value)` (catch-all; WS 1) | ✅ |
| Content blocks (text / thinking / tool_use / tool_result / server_tool_use) | ✅ | ✅ | ✅ `ContentBlock` (exhaustive, forward-compatible) | ✅ |
| `total_cost_usd` on result | ✅ | ✅ | ✅ `ResultMessage.total_cost_usd` | ✅ |
| Usage incl. **cache** counters | ✅ | ✅ | ✅ `Usage { input, output, cache_creation, cache_read }` | ✅ |
| `structured_output`, `model_usage`, `permission_denials`, rate-limit events | ✅ (rich) | ✅ | 🟡 `ResultMessage::structured_output` ✅ (WS B); `model_usage` / `permission_denials` / rate-limit ❌ | 🟡 leaner result frame |

**Verdict:** ✅ parity on the core frame taxonomy incl. cache-aware usage, cost, and a forward-compatible
`Message::Other` catch-all; 🟡 official result frame carries more diagnostic fields.

---

## 11. Runtimes / transport 🟣 — native runtimes removed (vision §5)

This was where clauders diverged from — and exceeded — the official SDKs. The *native, non-Claude*
runtimes below have been removed; the swappable-seam abstraction and the bundled Messages API client
were not superset claims in the same sense and remain.

| Runtime | Python | TS | clauders |
|---|---|---|---|
| `claude` CLI subprocess | ✅ (the *only* runtime) | ✅ (the *only* runtime) | ✅ `CliRuntime` (the *only* runtime) |
| Native `POST /v1/messages` agentic loop (in-process tool loop) | ❌ | ❌ | 🟣 `ApiRuntime<T: HttpTransport>` — removed (vision §5) |
| Native non-Claude models (DeepSeek/Kimi/Qwen via OpenRouter) | ❌ | ❌ | 🟣 `OpenRouterRuntime<T>` — removed (vision §5) |
| LLM-classified per-turn model routing across backends | ❌ | ❌ | 🟣 `RoutingRuntime` (+ `Classifier`, `RuntimeClassifier`, `ModelCard`, `RoutingSummary`) — removed (vision §5) |
| Swappable runtime seam / test double | ❌ | ❌ | 🟣 `Runtime` trait + `MockRuntime` — kept, not removed (vision §5): the object-safe seam and its test double are ordinary architecture, not a superset claim |
| Raw Messages API client bundled in the same crate | ❌ | ❌ | 🟣 `clauders::Client` / `messages::` — kept, not removed (vision §5): the Messages API is Pillar 1, core to the parity target, not a superset |

**Verdict:** 🟣 native runtimes removed (vision §5) — clauders *was* a strict superset on execution
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
| Cache-aware usage accounting across a tool loop | partial (surfaced in usage) | partial | 🟣 summed across turns onto the terminal `Result` (native loop) — removed with `ApiRuntime` (vision §5) |
| Cost-aware routing / context pruning / per-subtask downgrade | ❌ | ❌ | 🚧 shelved (vision §8 re-introduction criteria) |

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
| Permissions — full mode set (`default`/`acceptEdits`/`plan`/`bypass`/`dontAsk`/`auto`) + allow/deny/rewrite/context + deny-interrupt + `updated_permissions` | ✅ parity (WS C/D via CLI `can_use_tool` seam; native enforcement removed, vision §5) |
| Structured output (`output_format` + typed result) | ✅ parity (WS B; CLI passthrough best-effort) |
| Message taxonomy incl. cache usage + cost + `Message::Other` catch-all | ✅ parity |
| **Subagents** (`agents`/`AgentDefinition`) | ✅ parity (WS E; `--agents` passthrough) |
| **Sessions** (continue/resume/fork) | ✅ parity (WS F; list/inspect still ❌) |
| System-prompt preset + append | ✅ parity (WS A) |
| Config breadth (WS 1: fallback, strict-mcp, add-dirs, settings, budget, partial-messages, hook-events, prompt-tool override, stderr, max-buffer) | ✅ parity on the WS 1 knobs; 🟡 residual `setting_sources` + newer model knobs |
| **Setting sources** (filesystem config/CLAUDE.md) | ❌ behind (`settings` path/inline landed; `setting_sources` not) |
| Streaming input, live MCP control, warm start, MCP elicitation | ❌ behind (streaming input = WS G, next) |
| **Native multi-provider runtimes** (Api/OpenRouter/Routing) | 🟣 removed (vision §5) |
| **Prompt-cache policy + token efficiency** | 🟣 removed (vision §5) |
| **Middleware / evals / orchestration pool** | 🟣 removed (vision §5) |
| **Bundled raw Messages API client** | 🟣 kept, not removed (vision §5) — reclassified as Pillar 1, not a superset |

**One-line summary:** on the *CLI-driving agent core* clauders is now at parity across query/client,
tools, hooks, the full permission-mode set, system prompt, messages, structured output, **subagents,
sessions, and the WS 1 config breadth**. The remaining gaps are streaming input (WS G), `setting_sources`
+ filesystem config, and the live-control long tail (warm start, live MCP set, MCP elicitation). The
native multi-provider runtimes, prompt-cache policy, and middleware/evals/orchestration that used to read
"ahead" were a superset with no official counterpart and were **removed** in the parity-first pivot
(vision §5); clauders is a subset-completing parity client, not a superset.

---

## Candidate parity gaps worth closing (not commitments)

Ranked by leverage for the airsstack mission, not by official-checklist completeness:

1. **Streaming input** (`Prompt::Stream`) — the highest-value remaining primitive; enables interactive
   multi-turn feeds into a live session (WS G, in design).
2. ~~**Subagents (`AgentDefinition`)**~~ — **landed (WS E)**: `Options::agents` → `--agents` JSON
   passthrough.
3. ~~**Sessions (continue / resume / fork)**~~ — **landed (WS F)**: `SessionControl` → `--continue` /
   `--resume <id>` / `--fork-session`. Programmatic list/inspect remains ❌ (native store removed).
4. ~~**System-prompt preset + append**~~ — **landed (WS A)**.
5. ~~**`dontAsk` + `auto` permission modes + `updated_permissions` + deny-interrupt**~~ — **landed
   (WS C/D)** via the CLI `can_use_tool` seam. Native enforcement (`permission_engine`, model-judge)
   removed with `ApiRuntime` (vision §5); modes are forwarded verbatim to the binary.
6. **Setting sources** — evaluate deliberately: loading `CLAUDE.md`/settings fights token hygiene; may
   stay intentionally out of scope. (Inline/path `settings` already landed in WS 1.)
7. **Live-control long tail** — warm start, `reinitialize`, live MCP set/toggle, MCP elicitation.

Explicitly *not* gaps to chase: `skills`, `plugins`, `sandbox`, `betas`, session-store mirroring —
CLI-feature passthroughs with little bearing on the Rust SDK's thesis.

---

## Methodology & caveats

- **clauders side** — read directly from source on the parity-first, CLI-only tree
  (`crates/clauders/src/agent/`: `options.rs`, `permissions/`, `subagents/`, `types/session_control.rs`,
  `runtime/cli/{argv,runtime}.rs`, `process/{pipes,spawn,handle}.rs`, `message.rs`, `content.rs`,
  `hooks.rs`, `mcp/`, and the `agent/mod.rs` re-export set). Authoritative.
- **Official side** — the two official SDK references at
  `code.claude.com/docs/en/agent-sdk/{python,typescript}`, cross-checked against the live `claude` Code
  binary **v2.1.209** for the flag surface. The official SDKs iterate quickly; exact option keys,
  permission modes, and hook-event names drift between releases. Re-verify against the live reference
  before treating any single ❌ as a hard commitment.
- Parity marks judge *capability*, not wire/name identity. clauders is idiomatic Rust (builders,
  trait objects, exhaustive enums), so equivalent features carry Rust-shaped names.

## Sources

- TypeScript SDK reference — <https://code.claude.com/docs/en/agent-sdk/typescript>
- Python SDK reference — <https://code.claude.com/docs/en/agent-sdk/python>
- clauders roadmap — [`../agent-sdk-roadmap.md`](../agent-sdk-roadmap.md)
