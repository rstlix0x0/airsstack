# clauders Agent SDK — Feature Parity vs the Official Claude Agent SDKs

Compares the `clauders` Rust Agent SDK (module `clauders::agent`) against the two **official**
Claude Agent SDKs:

- **Python** — `claude-agent-sdk` (formerly `claude-code-sdk`)
- **TypeScript** — `@anthropic-ai/claude-agent-sdk`

**As of:** 2026-07-09 · clauders at HEAD `6fa63f5` (Phase 3 ws2 Scope C complete).
Official surfaces captured from `code.claude.com/docs/en/agent-sdk/{python,typescript}`.

> **Read this first — the one difference that reframes everything.**
> The official SDKs are **thin clients that drive the `claude` Code CLI binary as a subprocess**.
> Every "runtime" they have is that one subprocess transport; they do **not** implement a native
> Messages API loop, and they are **Claude-only**. `clauders` ships that same subprocess runtime
> (`CliRuntime`) **and** three additional native runtimes the official SDKs have no equivalent of —
> `ApiRuntime` (an in-process `POST /v1/messages` agentic loop), `OpenRouterRuntime` (native
> non-Claude models), and `RoutingRuntime` (LLM-classified per-turn model routing). So parity is not
> a single axis: on the *CLI-driving surface* clauders trails on session/config breadth; on
> *native multi-provider execution + token efficiency + a typed extension system* clauders is
> deliberately ahead and has no counterpart to compare against.

---

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Full parity — equivalent capability exists |
| 🟡 | Partial — core exists, narrower than official |
| ❌ | Absent in clauders |
| 🟣 | **clauders-only** — no official counterpart |
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

**Verdict:** ✅ core session loop at parity; 🟡 missing streaming-input and the long tail of live-control ops.

---

## 2. Configuration surface (`Options` / `ClaudeAgentOptions`)

clauders `Options` (14 fields) vs the official surface (~40+ fields). Core is covered; the gap is
session management, filesystem-settings integration, and newer model/feature knobs.

| Option (official name) | Python | TS | clauders field | Status |
|---|---|---|---|---|
| System prompt (plain string) | ✅ | ✅ | `system_prompt: Option<String>` | ✅ |
| System prompt **preset** `claude_code` + `append` | ✅ | ✅ | ❌ | 🟡 string only |
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
| `output_format` / structured output (agent layer) | ✅ | ✅ | ❌ on agent (✅ on `messages::OutputConfig`) | 🟡 |
| `thinking` / `effort` / `max_thinking_tokens` | ✅ | ✅ | ❌ | ❌ |
| `max_budget_usd` | ✅ | ✅ | ❌ | ❌ |
| `skills`, `plugins`, `sandbox`, `betas` | ✅ | ✅ | ❌ | ❌ (CLI-feature knobs) |
| `session_store` / `enable_file_checkpointing` | ✅ | ✅ | ❌ | ❌ |
| Per-request `max_tokens` | — (CLI-managed) | — | `max_tokens: MaxTokens` (default 4096) | 🟣 needed by native `ApiRuntime` |
| Prompt-cache policy | ❌ | ❌ | `CachePolicy` (via `ApiRuntime`) | 🟣 (see §13) |
| Min-version gate / shutdown grace | — | — | `require_min_version`, `shutdown_grace` | 🟣 process-hygiene |

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

**Verdict:** ✅ strong parity on in-process tools; only gaps are richer result-content kinds and the
TS Zod-style typed argument inference.

---

## 4. Hooks

clauders models a broad hook-event set and the full control-response payload.

| Aspect | Python | TS | clauders |
|---|---|---|---|
| Registration with matcher | ✅ `HookMatcher` | ✅ `HookCallbackMatcher` | ✅ `Options::hook(event, matcher, Arc<dyn Hook>)` |
| Capability-gated to binary support | — | — | 🟣 `Capabilities::supports_hook` skips unsupported events |
| Return: block / continue / suppressOutput / systemMessage / reason | ✅ | ✅ | ✅ `HookOutput { continue_, suppress_output, decision: Block, system_message, reason }` |

**clauders `HookEvent`s:** `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`,
`Stop`, `SubagentStart`, `SubagentStop`, `PreCompact`, `Notification`, `PermissionRequest`.

The official SDKs forward whatever the CLI supports (at least PreToolUse, PostToolUse,
UserPromptSubmit, Stop, SubagentStop, PreCompact, SessionStart/End, Notification, and newer granular
tool hooks). clauders does **not** yet model `SessionStart`/`SessionEnd`; it adds `PostToolUseFailure`
and `PermissionRequest`.

**Verdict:** ✅ parity on the hook mechanism and payload; 🟡 event-name set differs at the edges.

---

## 5. Permissions

| Aspect | Python | TS | clauders | Status |
|---|---|---|---|---|
| `default` | ✅ | ✅ | ✅ | ✅ |
| `acceptEdits` | ✅ | ✅ | ✅ | ✅ |
| `plan` | ✅ | ✅ | ✅ | ✅ |
| `bypassPermissions` | ✅ | ✅ | ✅ | ✅ |
| `dontAsk` | ✅ | ✅ | ❌ | 🟡 newer mode |
| `auto` (model-classified) | ✅ | ✅ | ❌ | 🟡 newer mode |
| `can_use_tool` callback | ✅ | ✅ | ✅ `PermissionPolicy::can_use_tool` | ✅ |
| Allow + rewrite input | ✅ `updated_input` | ✅ | ✅ `Allow { updated_input }` | ✅ |
| Deny + message | ✅ `message` | ✅ | ✅ `Deny { message }` | ✅ |
| Deny + `interrupt` | ✅ | ✅ | ❌ | 🟡 |
| Return **permission updates** (persist allow/deny rules) | ✅ `updated_permissions` | ✅ `updatedPermissions` → settings scopes | ❌ | ❌ |
| Rich request context | ✅ `ToolPermissionContext` | ✅ (toolUseID, agentID, blockedPath, decisionReason…) | ✅ `PermissionContext` (all of those fields) | ✅ |

**Verdict:** ✅ full parity on the allow/deny + input-rewrite core and request context; ❌ on persisting
permission-rule updates; 🟡 missing the `dontAsk`/`auto` modes and deny-interrupt.

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
| Plain string | ✅ | ✅ | ✅ |
| Preset `claude_code` + `append` | ✅ | ✅ | ❌ |
| `excludeDynamicSections` | — | ✅ | ❌ |

**Verdict:** 🟡 plain string only.

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
| `structured_output`, `model_usage`, `permission_denials`, rate-limit events | ✅ (rich) | ✅ | ❌ | 🟡 leaner result frame |

**Verdict:** ✅ parity on the core frame taxonomy incl. cache-aware usage and cost; 🟡 official result
frame carries more diagnostic fields.

---

## 11. Runtimes / transport 🟣

This is where clauders diverges from — and exceeds — the official SDKs.

| Runtime | Python | TS | clauders |
|---|---|---|---|
| `claude` CLI subprocess | ✅ (the *only* runtime) | ✅ (the *only* runtime) | ✅ `CliRuntime` |
| Native `POST /v1/messages` agentic loop (in-process tool loop) | ❌ | ❌ | 🟣 `ApiRuntime<T: HttpTransport>` |
| Native non-Claude models (DeepSeek/Kimi/Qwen via OpenRouter) | ❌ | ❌ | 🟣 `OpenRouterRuntime<T>` |
| LLM-classified per-turn model routing across backends | ❌ | ❌ | 🟣 `RoutingRuntime` (+ `Classifier`, `RuntimeClassifier`, `ModelCard`, `RoutingSummary`) |
| Swappable runtime seam / test double | ❌ | ❌ | 🟣 `Runtime` trait + `MockRuntime` |
| Raw Messages API client bundled in the same crate | ❌ | ❌ | 🟣 `clauders::Client` / `messages::` |

**Verdict:** 🟣 clauders is a strict superset on execution backends. The official SDKs cannot run a
model without the `claude` binary and cannot run a non-Claude model at all. This directly serves the
airsstack north star (mixed routing to cheaper models).

---

## 12. Extension system 🟣

No official counterpart — the official SDKs offer no in-SDK middleware, evals, or concurrency engine.

| Subsystem | clauders surface |
|---|---|
| Middleware (Tower-style) | `Layer`, `Stack`, `Trace`/`TraceRuntime`, `Retry`/`RetryRuntime`, `TokenMeter`/`MeterRuntime`/`MeterHandle`/`UsageTotals`, `Tap` |
| Evals harness (runtime-agnostic) | `Case`, `EvalSuite`, `Scorer`, `Grader`, `Judge`, `Score`, `Outcome`, `Report`, `CaseReport` |
| Multi-process orchestration | `Pool`, `Limiter`, `SemaphoreLimiter` (bounded-concurrency, backpressure) |

**Verdict:** 🟣 entirely clauders-only. These are the "framework" ambitions (LangChain/DSPy/DeepEval
inspirations) the official SDKs leave to userland.

---

## 13. Token efficiency 🟣

| Capability | Python | TS | clauders |
|---|---|---|---|
| Programmable prompt-cache breakpoint policy | ❌ (CLI-managed) | ❌ (CLI-managed) | 🟣 `CachePolicy { Off, Prefix, PrefixAndConversation }` on `ApiRuntime` |
| Cache-aware usage accounting across a tool loop | partial (surfaced in usage) | partial | ✅ summed across turns onto the terminal `Result` |
| Cost-aware routing / context pruning / per-subtask downgrade | ❌ | ❌ | 🚧 planned (later Scope C slices) |

**Verdict:** 🟣 clauders exposes prompt caching as a first-class, programmable SDK surface on its
native runtime — the official SDKs delegate all caching to the CLI and never surface a policy knob.

---

## Overall scorecard

| Area | clauders vs official |
|---|---|
| One-shot + stateful entry points | ✅ parity |
| In-process MCP tools | ✅ parity (minus Zod typing / rich content) |
| Hooks | ✅ parity (event set differs at edges) |
| Permissions core (allow/deny/rewrite/context) | ✅ parity |
| Message taxonomy incl. cache usage + cost | ✅ parity |
| Config breadth | 🟡 core covered, ~25 newer knobs missing |
| Permission-rule updates, `dontAsk`/`auto`, deny-interrupt | ❌ behind |
| **Subagents** (`agents`/`AgentDefinition`) | ❌ behind |
| **Sessions** (continue/resume/fork/list) | ❌ behind |
| **Setting sources** (filesystem config/CLAUDE.md) | ❌ behind |
| System-prompt preset + append | 🟡 behind |
| Streaming input, live MCP control, partial messages | ❌ behind |
| **Native multi-provider runtimes** (Api/OpenRouter/Routing) | 🟣 ahead — no official equivalent |
| **Prompt-cache policy + token efficiency** | 🟣 ahead |
| **Middleware / evals / orchestration pool** | 🟣 ahead |
| **Bundled raw Messages API client** | 🟣 ahead |

**One-line summary:** clauders is at parity on the *CLI-driving agent core* (query/client, tools,
hooks, permissions, messages), trails on *session management, subagents, and filesystem-settings
integration*, and is deliberately ahead on *native multi-provider execution, token efficiency, and a
typed extension framework* — the axes that serve the airsstack "cheaper tokens, mixed routing" thesis.

---

## Candidate parity gaps worth closing (not commitments)

Ranked by leverage for the airsstack mission, not by official-checklist completeness:

1. **Subagents (`AgentDefinition`)** — highest-value missing primitive; also unblocks the blocked
   Scope C "per-subtask downgrade" slice (route a subagent to a cheaper model).
2. **Sessions (continue / resume / fork)** — the largest missing subsystem; needed for any long-lived
   or resumable workflow, and a prerequisite for context-pruning experiments.
3. **System-prompt preset + append** — cheap, unblocks reusing Claude Code's built-in prompt.
4. **`dontAsk` / `auto` permission modes + `updated_permissions`** — small deltas, keeps the
   permission surface current.
5. **Streaming input** — enables interactive multi-turn feeds into a live session.
6. **Setting sources** — evaluate deliberately: loading `CLAUDE.md`/settings fights token hygiene;
   may stay intentionally out of scope.

Explicitly *not* gaps to chase: `skills`, `plugins`, `sandbox`, `betas`, session-store mirroring —
CLI-feature passthroughs with little bearing on the Rust SDK's thesis.

---

## Methodology & caveats

- **clauders side** — read directly from source at HEAD `6fa63f5` (`crates/clauders/src/agent/`:
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
