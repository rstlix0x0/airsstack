# clauders Agent SDK — Feature Parity vs the Official Claude Agent SDKs

> **Parity-first update (2026-07-13):** The 🟣 rows below marked clauders "ahead of the official SDK"
> were a **superset** with no official counterpart. Per [`../vision-and-strategy.md`](../vision-and-strategy.md)
> §5 they have been **removed** from the crate. They remain in this table (re-legended as "Removed")
> so the parity picture is honest: clauders is now a *subset-completing* client, not a superset.
> (This holds for the 🟣 rows themselves; §7 separately records that the removed native
> `SessionStore`'s original "no official counterpart" justification was itself wrong — see §7's
> correction.)

Compares the `clauders` Rust Agent SDK (module `clauders::agent`) against the two **official**
Claude Agent SDKs:

- **Python** — `claude-agent-sdk` (formerly `claude-code-sdk`)
- **TypeScript** — `@anthropic-ai/claude-agent-sdk`

**As of:** 2026-07-20 · Phase 4 (WS A–F) + Phase 5 WS 1–7 landed (`Options` breadth, `thinking`/`effort`,
hook-event edges, MCP result-content kinds, `AgentDefinition` extra fields, streaming input, MCP
elicitation), plus WS 8's session-config slice (`sessionId`/`title`/`persistSession`), on the
parity-first, **CLI-only** tree. The five session filesystem ops
(list/inspect/messages/rename/tag), `resumeSessionAt`, and all of WS 9 (live-control tail) remain open —
see §7. Grounded on the live `claude` Code binary **v2.1.215**, the shipped
`@anthropic-ai/claude-agent-sdk@0.3.215` and `claude-agent-sdk` 0.2.123 (Python) **sources**, and the
live SDK references at `code.claude.com/docs/en/agent-sdk/{python,typescript}`.

> **The tree is CLI-only now.** After the parity-first pivot (vision §5) the native runtimes —
> `ApiRuntime` (in-process `POST /v1/messages` loop), `OpenRouterRuntime`, `RoutingRuntime` — and their
> enforcement machinery (the `permission_engine` `RuleStore`, the model-judge `PermissionJudge`, the
> filesystem `SessionStore`/`list_sessions`, `CachePolicy`, the middleware/evals/orchestration tiers)
> were **all removed**. `CliRuntime` is the single agent-execution path, exactly like the official SDKs.
> Everything WS D/E/F and WS 1 added therefore lands as **CLI passthrough** — data forwarded to the
> binary's control protocol, or flags lowered in `build_argv` — never native in-process enforcement.

> **Read this first — clauders drives the CLI, same as the official SDKs.**
> The official SDKs are **thin clients that drive the `claude` Code CLI binary as a subprocess**.
> Their *agent execution* is that one subprocess transport; they do **not** implement a native
> Messages API loop, and they are **Claude-only**. `clauders` ships that same subprocess runtime
> (`CliRuntime`) as its Agent SDK surface — and, after the pivot, *only* that.
>
> **But "drives the CLI" is not the whole surface.** The session ops in §7 bypass the subprocess
> entirely and read/append the CLI's own `.jsonl` transcripts directly from disk. Treating the
> subprocess as the *only* official mechanism is precisely the assumption that made WS 8 conclude these
> ops had "no parity path" — a control-protocol grep cannot disprove a filesystem API. When assessing a
> new op, establish which mechanism it uses before searching for it.
>
> Parity is therefore not a single axis:
> on the *CLI-driving surface* clauders is now at parity on the session/config/subagent breadth it used
> to trail on (see the scorecard), plus streaming input and MCP elicitation (both landed); live MCP
> control, warm start, and the five session filesystem ops remain the gaps (§1, §7). The Pillar-1
> bundled Messages API client (`clauders::Client` /
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
| Streaming **input** (prompt as async stream) | ✅ `AsyncIterable` | ✅ `AsyncIterable<SDKUserMessage>` | ✅ `Prompt::Stream` | landed (WS 6); `Prompt` is `Single(String)` or `Stream(Pin<Box<dyn Stream<Item=String>>>)`, fed to the binary's stdin as items arrive |
| Message stream out | ✅ `AsyncIterator[Message]` | ✅ `AsyncGenerator<SDKMessage>` | ✅ `MessageStream` (`Stream<Item=Result<Message>>`) | |
| Interrupt / cancel | ✅ `interrupt()` | ✅ `interrupt()` / `AbortController` | ✅ `interrupt()` | |
| Switch model mid-session | ✅ `set_model()` | ✅ `setModel()` | ✅ `set_model()` | |
| Switch permission mode mid-session | ✅ `set_permission_mode()` | ✅ `setPermissionMode()` | ✅ `set_permission_mode()` | |
| MCP server status | ✅ `get_mcp_status()` | ✅ `mcpServerStatus()` | ✅ `mcp_status()` | |
| Reconnect / toggle / set MCP servers live | ✅ | ✅ | ❌ | |
| `supportedCommands / Models / Agents`, `accountInfo`, `rewindFiles`, `stopTask`, `setMaxThinkingTokens` | ✅/partial | ✅ | ❌ | official CLI-control extras (`setMaxThinkingTokens`: see the reachability note on the `thinking`/`max_thinking_tokens` row, §2) |
| Warm startup (pre-warmed subprocess) | ✅ (`startup()` / `WarmQuery`) | ✅ `startup()` | ❌ | official spawn-latency optimization |
| Live reconfigure (`reinitialize`, `applyFlagSettings`) | partial | ✅ | ❌ | |

**Verdict:** ✅ core session loop at parity, now including streaming input (`Prompt::Stream`, WS 6); 🟡
the long tail of live-control ops (warm start, reinitialize, live MCP set) remains ❌.

---

## 2. Configuration surface (`Options` / `ClaudeAgentOptions`)

clauders `Options` (37 fields) vs the official surface (~40+ fields). The core plus the WS 1 breadth
knobs, `effort` (WS 2), and the WS 8 session-config slice (`session_id`/`title`/`session_persistence`)
are covered; the residual gap is `setting_sources`, the CLI-feature knobs (skills, plugins, sandbox),
`thinking` (CLI-limited — no binary flag), the live-control long tail, and the WS 8 session
list/inspect/rename/tag ops (see §7).

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
| `sessionId` (force a new session's id) | ✅ | ✅ | `session_id: Option<SessionId>` (→ `--session-id <id>`, new sessions only) | ✅ (WS 8 slice) |
| `title` (session display title) | ✅ | ✅ | `title: Option<String>` (carried in the `initialize` handshake payload) | ✅ (WS 8 slice) |
| `persistSession` | ✅ | ✅ | `session_persistence: SessionPersistence` (→ `--no-session-persistence` when `Disabled`) | ✅ (WS 8 slice) |
| MCP elicitation policy (`onElicitation`) | ✅ | ✅ | `elicitation_policy: Option<Arc<dyn ElicitationPolicy>>` | ✅ (WS 7; see §3) |
| Control-request timeout | — | — | `control_request_timeout: Duration` (bounds `interrupt`/`set_model`/elicitation waits; default 60s, chosen to mirror the official Python SDK's control-request timeout default — grounded in a prior session's reading of the Python SDK source, not re-verified against source in this pass; the TypeScript SDK's equivalent default was never read from source and is not claimed here) | 🟣 clauders-only reliability knob — **kept** (active), no official `Options` field |
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
| `effort` | ✅ | ✅ | `effort: Option<EffortLevel>` (→ `--effort <level>`) | ✅ (WS 2) |
| `thinking` / `max_thinking_tokens` | ✅ | ✅ | ❌ | ❌ CLI-limited — binary `v2.1.215` exposes no thinking flag; adaptive via `--effort`; raw config via `settings`. `max_thinking_tokens` deprecated upstream. Separately — `set_max_thinking_tokens` sits on the same reachability footing as `rename_session` (§7): it is a fully-wired `control_request` subtype, live-dispatched off the same stdin `stream-json` if/else chain `CliRuntime` already drives (real validation logic, not a stub), and it is *also* dispatched on the SSE-bridge path (`function NWu`, wired to Anthropic's cloud-relay "Remote Control" feature, not the subprocess pipe) — the same both-dispatchers shape as `rename_session`. This does not change the mark below — clauders does not implement it either way, so it stays ❌ (§1 cross-reference: `:70`) |
| `max_budget_usd` | ✅ | ✅ | `max_budget_usd: Option<BudgetUsd>` (→ `--max-budget-usd`) | ✅ (WS 1) |
| `skills`, `plugins`, `sandbox`, `betas` | ✅ | ✅ | ❌ | ❌ (CLI-feature knobs; `skills` now exists as an `AgentDefinition`-scoped field, §6, but not on top-level `Options`) |
| `session_store` / `enable_file_checkpointing` | ✅ | ✅ | ❌ | ❌ (native `SessionStore` removed, vision §5) |
| Per-request `max_tokens` | — (CLI-managed) | — | `max_tokens: MaxTokens` (default 4096) | 🟡 field present but **inert** — its native `ApiRuntime` consumer was removed (vision §5) |
| Min-version gate / shutdown grace | — | — | `require_min_version`, `shutdown_grace` | 🟣 clauders-only process-hygiene on `CliRuntime` — **kept** (active), no official counterpart |
| Prompt-cache policy | ❌ | ❌ | `CachePolicy` (via `ApiRuntime`) | 🟣 removed (vision §5) — see §13 |

**Verdict:** ✅ on the tool/permission/mcp/hook/cwd/env core **and** the WS 1 breadth (fallback,
strict-mcp, add-dirs, settings, budget, partial-messages, hook-events, prompt-tool override, stderr,
max-buffer) **and** `effort` (WS 2) **and** sessions + subagents **and** the WS 8 session-config slice
(`session_id`/`title`/`session_persistence`) **and** the MCP elicitation policy seam (WS 7); ❌ on
`setting_sources`, the CLI-feature knobs (skills, plugins, sandbox — `skills` now exists per-subagent on
`AgentDefinition`, §6, but not on top-level `Options`), and the WS 8 session list/inspect/rename/tag ops
(see §7); `thinking` is CLI-limited (no binary flag — see §2 table).

---

## 3. In-process MCP tools

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| Define a tool | ✅ `@tool(name, desc, schema)` | ✅ `tool(name, desc, zodShape, handler)` | ✅ `tool(name, desc, json_schema, closure)` + `impl Tool` | ✅ |
| Create in-process server | ✅ `create_sdk_mcp_server(name, version, tools)` | ✅ `createSdkMcpServer({name, version, tools})` | ✅ `SdkMcpServer::builder(name).version().tool().build()` | ✅ |
| Registry of servers | implicit | implicit | ✅ `SdkMcpRegistry` | ✅ |
| Tool result content blocks | ✅ | ✅ | ✅ `ToolResult` / `ToolContent` (`Text`, `Image`, `Audio`, `ResourceLink`, `Resource`; `#[non_exhaustive]` for forward-compat) | ✅ (WS 4) |
| Tool annotations (readOnly/destructive/…) | ✅ | ✅ `ToolAnnotations` | ✅ `ToolAnnotations` | ✅ |
| JSON-RPC dispatch (`tools/list`, `tools/call`) | ✅ | ✅ | ✅ `mcp::router` | ✅ |
| Input schema | JSON / type | **Zod shape** (typed inference) | raw `serde_json::Value` schema | 🟡 no compile-time arg typing |
| MCP **elicitation** (server asks user for input mid-call) | ✅ | ✅ `onElicitation` + `mcp_elicitation` hook | ✅ `ElicitationPolicy::elicit` + `Elicitation`/`ElicitationResult` `HookEvent`s | ✅ (WS 7) |

**Verdict:** ✅ strong parity on in-process tools, including the richer result-content kinds (WS 4) and
MCP elicitation (WS 7); the remaining gap is the TS Zod-style typed argument inference — clauders tools
take a raw JSON-schema `Value`, with no compile-time arg typing.

---

## 4. Hooks

clauders models a broad hook-event set and the full control-response payload.

| Aspect | Python | TS | clauders |
|---|---|---|---|
| Registration with matcher | ✅ `HookMatcher` | ✅ `HookCallbackMatcher` | ✅ `Options::hook(event, matcher, Arc<dyn Hook>)` |
| Capability-gated to binary support | — | — | ✅ `Capabilities::supports_hook` warns on unsupported events |
| Return: block / continue / suppressOutput / systemMessage / reason | ✅ | ✅ | ✅ `HookOutput { continue_, suppress_output, decision: Block, system_message, reason }` |
| Hook-lifecycle observability frames | ✅ (`includeHookEvents`) | ✅ | ✅ `Options::include_hook_events` → `--include-hook-events`; frames surface as `Message::Other` (WS 1) |

**clauders `HookEvent`s:** `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`,
`Stop`, `SubagentStart`, `SubagentStop`, `PreCompact`, `Notification`, `PermissionRequest`,
`SessionStart`, `SessionEnd`, `Setup`, `Elicitation`, `ElicitationResult` (15 variants total; the last
five landed in WS 3 and WS 7).

The official SDKs forward whatever the CLI supports (at least PreToolUse, PostToolUse,
UserPromptSubmit, Stop, SubagentStop, PreCompact, SessionStart/End, Notification, and newer granular
tool hooks). clauders now models `SessionStart`/`SessionEnd`/`Setup` and the elicitation pair
(`Elicitation`/`ElicitationResult`, WS 3 + WS 7); it still adds `PostToolUseFailure` and
`PermissionRequest`, which remain clauders-only extras with no listed official counterpart.

**Verdict:** ✅ parity on the hook mechanism and payload **and** the event-name set, now including
`SessionStart`/`SessionEnd`/`Setup`/`Elicitation`/`ElicitationResult` (WS 3, WS 7); clauders' own
`PostToolUseFailure`/`PermissionRequest` extras remain the only edge divergence.

> **Deferred:** the `Elicitation`/`ElicitationResult` hook events are registered and capability-gated,
> but their answer-substituting semantics — a hook returning `{action, content}` to pre-empt or override
> an elicitation before the registered `ElicitationPolicy` runs — require `HookOutput` fields that do
> not exist yet (`HookOutput` today carries `continue_`/`suppress_output`/`decision`/`system_message`/
> `reason` only). Tracked as a follow-up, not a gap in the hook *event* surface itself.

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
| Programmatic `agents` / `AgentDefinition` (description, prompt, tools, disallowedTools, model, maxTurns, permissionMode, skills, memory, mcpServers, initialPrompt, background, effort) | ✅ | ✅ | ✅ `Options::agents: HashMap<String, AgentDefinition>` → `--agents` JSON (WS E; six extra fields landed WS 5) | ✅ |
| Awareness of subagent lifecycle | via hooks | via hooks | ✅ `HookEvent::SubagentStart/Stop` | ✅ |

**Verdict:** ✅ clauders has programmatic subagent definitions, lowered to the binary's `--agents` JSON
passthrough (WS E), now covering all 13 official `AgentDefinition` fields — `skills`, `memory`,
`mcpServers`, `initialPrompt`, `background`, and `effort` landed in WS 5. The native nested-subagent loop
that WS E also landed on `ApiRuntime` was removed with that runtime (vision §5), so subagents are a
CLI-passthrough capability now — which is exactly what the official SDKs are. One caveat: the
`mcpServers` wire shape is unconfirmed (`subagents/definition.rs`) — the official `AgentMcpServerSpec`
element is undocumented, so clauders assumes `{ "name": …, "config": … }`; re-verify against a live
`--agents` round-trip before treating it as at parity.

---

## 7. Sessions

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| `continue` most recent | ✅ | ✅ | ✅ `SessionControl::Continue` → `--continue` (WS F) | ✅ |
| `resume` by session id | ✅ | ✅ | ✅ `SessionControl::Resume { id }` → `--resume <id>` (WS F) | ✅ |
| `fork_session` | ✅ | ✅ | ✅ `SessionControl::Resume { fork: true }` → `--fork-session` (WS F) | ✅ |
| Session id type | ✅ | ✅ | ✅ `SessionId` on frames + `SessionControl` resume/fork wiring | ✅ |
| `sessionId` (force a new session's id) | ✅ | ✅ | ✅ `Options::session_id` → `--session-id <id>` (new sessions only) | ✅ (WS 8 slice) |
| `title` (session display title) | ✅ | ✅ | ✅ `Options::title` → `initialize` handshake payload | ✅ (WS 8 slice) |
| `persistSession` | ✅ | ✅ | ✅ `Options::session_persistence` → `--no-session-persistence` when disabled | ✅ (WS 8 slice) |
| `listSessions`/`list_sessions`, `getSessionMessages`/`get_session_messages`, `getSessionInfo`/`get_session_info`, `tagSession`/`tag_session` | ✅ | ✅ | ❌ | ❌ real parity gap — filesystem-only, see below |
| `renameSession`/`rename_session` (rename an **arbitrary** session by id) | ✅ | ✅ | ❌ | ❌ real parity gap — filesystem-only, same as the four above |
| Rename the **currently running** session | ❌ | ❌ | ❌ | ❌ CLI-only capability with no official-SDK counterpart — a live `rename_session` control subtype, see below |
| `resumeSessionAt` (resume at a specific message UUID) | ❌ no Python equivalent | ✅ `Options.resumeSessionAt` | ❌ | ❌ TS-only, undocumented CLI flag — see below |

**List / inspect / messages / tag sessions — a real parity gap, filesystem-only.** Verified directly against the
shipped `@anthropic-ai/claude-agent-sdk@0.3.215` bundle (`package/sdk.mjs`), and independently
corroborated from a second direction by mining the live `claude` v2.1.215 binary itself: the compiled
CLI embeds this same JS/TS Agent SDK module (a bundler export table exposing `query`,
`listSessions`, `getSessionMessages`, `getSessionInfo`, `renameSession`, `tagSession`). Both official
SDKs implement `listSessions`/`getSessionMessages`/`getSessionInfo`/`tagSession` (and their Python
snake_case equivalents) as **plain local-filesystem CRUD** over the same
`~/.claude/projects/<encoded-cwd>/*.jsonl` transcripts the CLI itself writes — `readdir`/`stat`/`open`
via Node's `fs/promises`, appending a `tag` JSONL entry for tag. Each function's default path takes an
optional caller-supplied `sessionStore` override (the official pluggable-backend extension point); absent
that override — the shipped default — there is no HTTP or control-plane call, only local file I/O.
(Confirmed by a discriminating search: zero `fetch`/`http`/`socket` occurrences across the entire TS
session-implementation region or either Python module; the only out-of-process call on the local path is
`git worktree list --porcelain`.) None of these four go through the subprocess stream-json control
protocol; a Rust implementation therefore needs no new subprocess/control-plane plumbing — only
JSONL-compatible file I/O against the documented on-disk format. Their absence in clauders is a genuine
parity gap — see "On cost" below for why it is not a *cheap* one.

**The two SDKs are not at parity with each other; TS is a strict superset.** Reimplementing "the
official behavior" therefore requires choosing a target. TS-only: `includeProgrammatic`,
`includeSystemMessages` (and a `"system"` message type), `parent_agent_id`, a `relocatedCwd` preference
for `cwd`, a `sessionId` sort tiebreak, batched pagination with early exit, compact-boundary parent
re-chaining, sibling-assistant re-insertion, and a >5 MiB pre-compact fast path. Python-only behaviors:
unconditional NFC normalization (TS normalizes only on darwin) and silent rather than throwing overflow
in the tag unicode sanitizer. Python also lacks `resumeSessionAt` entirely.

**`renameSession`/`rename_session` — a filesystem op, plus a *different* CLI-only capability.** The
official `renameSession(sessionId, title)` is the fifth filesystem op: it appends a
`{"type":"custom-title","customTitle":…,"sessionId":…}` line to the target session's `.jsonl`, and it
targets an **arbitrary** session by id. Closing it needs the same local file I/O as the four above.

A `rename_session` `control_request` subtype does also exist on the binary's stdin `stream-json` chain
that `CliRuntime` already drives — but **it is not a path to the official op, and it does not make that
op cheaper to close.** The handler reads exactly one field, `title`, and renames the *currently running*
session implicitly; it accepts no session id in any spelling. Verbatim from the live v2.1.215 binary at
byte `236717377`:

```js
else if(qe.request.subtype==="rename_session")try{let lr=qe.request.title.trim();if(!lr)rn(qe,"title must be non-empty");else{if(eM())await toe(Tt(),lr,void 0,"remote");else ZLt(lr);Gt=!0,Qs(qe)}}catch(lr){rn(qe,ue(lr))}
```

`Tt()` is `getSessionId()` — the running session. So the two are **distinct capabilities**: an
arbitrary-session rename (official SDK surface, filesystem) and a running-session rename (CLI surface,
control protocol, *no* official-SDK counterpart in either language). clauders implements neither, so both
stay ❌, but they are separate line items with separate mechanisms and separate costs.

> **Correction.** A previous revision of this section claimed the control subtype made the official
> `renameSession` gap "closable without touching the filesystem at all… the cheapest of the five ops to
> close." **That was false** — it conflated the two capabilities above. The error came from confirming
> that the subtype exists without reading which fields its handler consumes. Grounding for the current
> text is a verbatim read of the live v2.1.215 handler; no earlier-version artifact was available, so no
> version-change claim is made here.

**`resumeSessionAt` — asymmetric across the official SDKs, not a sixth list/inspect op.** It is a
TS-only `Options` field on `query()` (not a standalone function, and absent from the official Python
SDK), lowered to a `--resume-session-at=<uuid>` CLI argument at subprocess spawn. **The flag is real
but hidden, not absent.** It is registered on the live v2.1.215 binary at byte `236775633` with
`.hideHelp()`, which is why it does not appear in `claude --help` (verified both ways: the string is
present in the binary, absent from the 230-line help output). It carries two preconditions the binary
enforces itself — it requires `--resume` (`Error: --resume-session-at requires --resume`, byte
`236634651`) and applies in print mode — and the SDK emits it in `=`-joined form. Rust
parity-with-Python does not require this field; parity-with-TS would, against a deliberately
undocumented CLI surface.

> **Correction to the vision §5 removal rationale.** This doc previously stated that clauders' removed
> native `SessionStore`/`list_sessions` was justified because it had "no official counterpart," and
> that "the official SDKs don't expose it either." **That premise is false** — both official SDKs ship
> `list_sessions`/`listSessions` and the four sibling ops above as public, documented, local-filesystem
> functions (verified against shipped SDK source, not just docs). This does **not** reverse the removal
> itself (vision §5) — the native store was `ApiRuntime`-only plumbing removed for reasons unrelated to
> this claim — but the *justification* that no official counterpart exists was wrong, and the
> official-parity gap it leaves behind is real. `vision-and-strategy.md` itself is not edited by this
> correction; it is recorded here, where the parity claim lives.

**Verdict:** ✅ parity on continue/resume/fork via `SessionControl` → CLI flags (WS F), plus the WS 8
session-config slice (`session_id`/`title`/`session_persistence`). ❌ remaining: all five filesystem ops
— `listSessions`/`getSessionMessages`/`getSessionInfo`/`tagSession`/`renameSession` — plus
`resumeSessionAt` (TS-only, hidden CLI flag) and, separately, the CLI-only running-session rename.

**On cost.** An earlier revision called the filesystem ops "comparatively cheap to close." Reading the
shipped implementations does not support that. They are not thin CRUD wrappers over
`serde_json::from_str`; the shared substrate is a fault-tolerant text-scanning engine over
possibly-truncated JSONL: an encoded-cwd path rule (realpath → NFC → non-alphanumeric→`-` → 200-char
truncation with a base36 int32-djb2 suffix → prefix-fallback sibling scan, because the CLI hashes with
`Bun.hash` and the SDKs cannot reproduce it); a 64 KiB head/tail "lite" read that never parses the whole
file; hand-rolled first/last `"key":"` scanners honoring backslash escapes; and, for
`getSessionMessages`, a conversation-DAG reconstruction (index by uuid, find terminals, walk `parentUuid`
with a cycle guard, prefer non-sidechain/non-meta/non-team leaves, pick highest file index, reverse).
The mutations are append-only with deliberate `O_WRONLY|O_APPEND` and no `O_CREAT`. **The five ops are a
workstream, not a task.** Sizing them as cheap is what the WS 8 spec did, and it is how they came to be
skipped.

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
| In-process MCP tools | ✅ parity (minus Zod typing) |
| Hooks | ✅ parity (event set differs at edges) |
| Permissions — full mode set (`default`/`acceptEdits`/`plan`/`bypass`/`dontAsk`/`auto`) + allow/deny/rewrite/context + deny-interrupt + `updated_permissions` | ✅ parity (WS C/D via CLI `can_use_tool` seam; native enforcement removed, vision §5) |
| Structured output (`output_format` + typed result) | ✅ parity (WS B; CLI passthrough best-effort) |
| Message taxonomy incl. cache usage + cost + `Message::Other` catch-all | ✅ parity |
| **Subagents** (`agents`/`AgentDefinition`) | ✅ parity (WS E; `--agents` passthrough) |
| **Sessions** (continue/resume/fork + session-config slice) | ✅ parity (WS F + WS 8 session-config slice); ❌ the five filesystem ops (list/inspect/messages/rename/tag) + `resumeSessionAt`, see §7 |
| System-prompt preset + append | ✅ parity (WS A) |
| Config breadth (WS 1: fallback, strict-mcp, add-dirs, settings, budget, partial-messages, hook-events, prompt-tool override, stderr, max-buffer; WS 8 session-config slice: `session_id`/`title`/`session_persistence`) | ✅ parity on the WS 1 + WS 8-slice knobs (`Options` now 37 fields); 🟡 residual `setting_sources` |
| **Setting sources** (filesystem config/CLAUDE.md) | ❌ behind (`settings` path/inline landed; `setting_sources` not) |
| Streaming input (WS 6) + MCP elicitation (WS 7) | ✅ parity — both landed |
| Live MCP control, warm start (WS 9 live-control tail) | ❌ behind |
| **Native multi-provider runtimes** (Api/OpenRouter/Routing) | 🟣 removed (vision §5) |
| **Prompt-cache policy + token efficiency** | 🟣 removed (vision §5) |
| **Middleware / evals / orchestration pool** | 🟣 removed (vision §5) |
| **Bundled raw Messages API client** | 🟣 kept, not removed (vision §5) — reclassified as Pillar 1, not a superset |

**One-line summary:** on the *CLI-driving agent core* clauders is now at parity across query/client,
tools (incl. richer result content, WS 4), hooks (incl. `SessionStart`/`SessionEnd`/`Setup`, WS 3), the
full permission-mode set, system prompt, messages, structured output, **subagents (incl. the WS 5 extra
fields), sessions (incl. the WS 8 session-config slice), streaming input (WS 6), MCP elicitation (WS 7),
and the WS 1 config breadth**. The remaining gaps are `setting_sources` + filesystem config, the
live-control long tail (warm start, live MCP set), and the five session filesystem ops —
`listSessions`/`getSessionMessages`/`getSessionInfo`/`renameSession`/`tagSession` (§7 — a real gap, and
a substantial one: a JSONL-scanning substrate plus conversation-DAG reconstruction, not thin CRUD;
`resumeSessionAt` is TS-only and lower priority). The native
multi-provider runtimes, prompt-cache policy, and middleware/evals/orchestration that used to read
"ahead" were a superset with no official counterpart and were **removed** in the parity-first pivot
(vision §5); clauders is a subset-completing parity client, not a superset.

---

## Candidate parity gaps worth closing (not commitments)

Ranked by leverage for the airsstack mission, not by official-checklist completeness:

1. ~~**Streaming input** (`Prompt::Stream`)~~ — **landed (WS 6)**: fed to the binary's stdin as NDJSON
   user messages as they arrive.
2. ~~**Subagents (`AgentDefinition`)**~~ — **landed (WS E; six extra fields WS 5)**: `Options::agents` →
   `--agents` JSON passthrough, now the full 13-field official shape.
3. ~~**Sessions (continue / resume / fork)**~~ — **landed (WS F)**: `SessionControl` → `--continue` /
   `--resume <id>` / `--fork-session`, plus the WS 8 session-config slice
   (`session_id`/`title`/`session_persistence`) — **also landed**. **Not landed:** session
   **list / inspect / messages / rename / tag**
   (`listSessions`/`getSessionMessages`/`getSessionInfo`/`renameSession`/`tagSession`) — a real parity
   gap, not CLI-unreachable (see §7's correction); both official SDKs implement all five as local
   `.jsonl` file I/O, needing no new subprocess/control-plane plumbing — but *not* cheap: they share a
   fault-tolerant JSONL-scanning substrate and `getSessionMessages` reconstructs the conversation DAG
   (§7, "On cost"). Size as a workstream. Also not landed: `resumeSessionAt` — TS-only (absent from the
   official Python SDK), mapping to the real-but-`.hideHelp()`-hidden `--resume-session-at` flag, which
   the binary requires be paired with `--resume`; lower priority. Separately not landed, and *not* an
   official-SDK op: renaming the **running** session via the `rename_session` control subtype.
4. ~~**System-prompt preset + append**~~ — **landed (WS A)**.
5. ~~**`dontAsk` + `auto` permission modes + `updated_permissions` + deny-interrupt**~~ — **landed
   (WS C/D)** via the CLI `can_use_tool` seam. Native enforcement (`permission_engine`, model-judge)
   removed with `ApiRuntime` (vision §5); modes are forwarded verbatim to the binary.
6. ~~**MCP result content kinds**~~ — **landed (WS 4)**: `ToolContent` now has `Text`, `Image`, `Audio`,
   `ResourceLink`, and `Resource`.
7. ~~**Hook-event edges** (`SessionStart`/`SessionEnd`)~~ — **landed (WS 3)**, plus `Setup`.
8. ~~**MCP elicitation**~~ — **landed (WS 7)**: `ElicitationPolicy` + `Elicitation`/`ElicitationResult`
   hook events. The answer-substituting hook semantics (a hook pre-empting or overriding an elicitation
   before the policy runs) remain deferred — `HookOutput` has no fields for it yet.
9. **Setting sources** — evaluate deliberately: loading `CLAUDE.md`/settings fights token hygiene; may
   stay intentionally out of scope. (Inline/path `settings` already landed in WS 1.)
10. **Live-control long tail** — warm start, `reinitialize`, live MCP set/toggle (WS 9).

Deferred CLI-only flags with no official Agent-SDK option counterpart: `--from-pr` (resume a session
linked to a PR) and `-n`/`--name` (a display-name convenience superseded by the `title` handshake
field).

Explicitly *not* gaps to chase: `skills`, `plugins`, `sandbox`, `betas`, and re-introducing a *native*
in-crate `SessionStore` that mirrors/duplicates the CLI's own on-disk session store (the superset
removed per vision §5) — distinct from the read-only session list/inspect/tag ops in item 3 above,
which *are* tracked as a real gap. These are CLI-feature passthroughs and superset re-additions with
little bearing on the Rust SDK's thesis.

---

## Methodology & caveats

- **clauders side** — read directly from source on the parity-first, CLI-only tree
  (`crates/clauders/src/agent/`: `options.rs`, `permissions/`, `subagents/`, `elicitation/`,
  `types/{session_control,session_persistence,prompt}.rs`,
  `runtime/cli/{argv,runtime,handshake,dispatch}.rs`, `process/{pipes,spawn,handle}.rs`, `message.rs`,
  `content.rs`, `hooks.rs`, `capabilities.rs`, `mcp/`, and the `agent/mod.rs` re-export set).
  Authoritative.
- **Official side** — the two official SDK references at
  `code.claude.com/docs/en/agent-sdk/{python,typescript}`, cross-checked against the live `claude` Code
  binary **v2.1.215** for the flag surface. Sessions (§7) are additionally cross-checked against the
  shipped `@anthropic-ai/claude-agent-sdk@0.3.215` npm bundle (`package/sdk.mjs`) and, independently, by
  mining the live v2.1.215 binary's embedded JS (the compiled CLI bundles that same SDK module plus the
  binary's own `control_request` schema union) — the actual shipped source on both counts, not a docs
  summary — to resolve the local-filesystem-vs-control-plane mechanism question per op:
  all five of `listSessions`/`getSessionMessages`/`getSessionInfo`/`renameSession`/`tagSession` are
  filesystem-only. The binary's stdin control chain was enumerated exhaustively (50 subtypes between
  bytes `236686835` and `236723607`, terminated by an `Unsupported control request subtype` default arm);
  four of the five op names appear **nowhere in the 247 MB executable** as byte strings, which forecloses
  a dispatcher of any shape, and the `rename_session` subtype that does exist renames the *running*
  session by `title` alone. Grounding for the binary-mining findings is
  v2.1.215 only; no earlier-version binary was available to confirm whether this is new or long-standing.
  The official SDKs iterate quickly; exact option keys, permission modes, and hook-event names drift
  between releases. Re-verify against the live reference before treating any single ❌ as a hard
  commitment.
- Parity marks judge *capability*, not wire/name identity. clauders is idiomatic Rust (builders,
  trait objects, exhaustive enums), so equivalent features carry Rust-shaped names.

## Sources

- TypeScript SDK reference — <https://code.claude.com/docs/en/agent-sdk/typescript>
- Python SDK reference — <https://code.claude.com/docs/en/agent-sdk/python>
- Session-storage reference — <https://code.claude.com/docs/en/agent-sdk/session-storage>
- Official session-ops shipped source — `@anthropic-ai/claude-agent-sdk@0.3.215` npm bundle,
  `package/sdk.d.ts` (unminified declarations) + `package/sdk.mjs` (used to resolve §7's
  local-filesystem-vs-control-plane question and to read the ops' actual implementations)
- Live CLI binary — `claude` v2.1.215 (Mach-O arm64, embeds the same JS/TS SDK module plus the binary's
  `control_request` schema union; used to independently corroborate §7's mechanism question, to
  enumerate the stdin control chain exhaustively, and to read the `rename_session` handler verbatim)
- Official Python SDK source — `claude-agent-sdk` 0.2.123 sdist, `_internal/sessions.py` +
  `_internal/session_mutations.py` + `types.py` (read directly for §7's TS-vs-Python divergences).
  Separately, the control-request timeout default cited at §2's "Control-request timeout" row comes from
  a *prior* session's reading and was not re-verified in this pass
- clauders roadmap — [`../agent-sdk-roadmap.md`](../agent-sdk-roadmap.md)
