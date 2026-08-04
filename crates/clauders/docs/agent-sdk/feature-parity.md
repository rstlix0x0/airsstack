# Agent SDK — parity with the official Claude Agent SDKs

What `clauders::agent` does and does not match in Anthropic's official Agent SDKs, judged against the
shipped artifacts rather than documentation.

**Pinned versions**

| Side | Artifact |
|---|---|
| TypeScript | `@anthropic-ai/claude-agent-sdk@0.3.221` — `sdk.d.ts` (unminified declarations) and `sdk.mjs` (runtime) |
| Python | `claude-agent-sdk 0.2.129` sdist — `types.py`, `client.py`, `query.py`, `_internal/sessions.py` |
| Binary | `claude` Code CLI v2.1.221 |
| clauders | `crates/clauders/src/agent/`; every `file:line` citation resolves against that tree |

Two cautions about the TypeScript artifact. `sdk.d.ts` is processed by api-extractor and has stripped
regions — the `Query` interface has gaps at `sdk.d.ts:2500-2506` and `:2522-2527` where non-public
members were removed. And `sdk.mjs` exports `parseDirectConnectUrl`, `DirectConnectTransport` and
`DirectConnectError` with no declarations anywhere. "Absent from `sdk.d.ts`" is therefore not the same
as "absent from the SDK"; where the two disagree this document grades against `sdk.mjs` and says so.

The two official SDKs are also not at parity with each other. TypeScript is close to a strict superset
— it has 63 `Options` properties (`sdk.d.ts:1322-2054`) against Python's 45 (`types.py:1763-2134`).
Rows below distinguish "missing from both" from "missing from one".

## Legend

| Mark | Meaning |
|---|---|
| ✅ | equivalent capability, equivalent behaviour |
| 🟡 | present but narrower than the official SDK |
| ❌ | absent |
| 🔷 | clauders has it; the official SDKs do not |
| 🔶 | deliberately different — see [divergences.md](../divergences.md) |

Marks judge capability, not name identity. clauders is idiomatic Rust — builders, exhaustive enums,
newtypes — so equivalent features carry Rust-shaped names.

---

## 1. Entry points and session control

| Capability | Python | TS | clauders | |
|---|---|---|---|---|
| One-shot query | ✅ | ✅ | `agent::query(prompt, Options)` → `MessageStream` (`client.rs:353`) | ✅ |
| Stateful client | ✅ `ClaudeSDKClient` | ✅ `query()` → `Query` | `Client::connect(options)` (`client.rs:303`), `Client::builder()` (`client.rs:294`) | ✅ |
| Message stream out | ✅ | ✅ | `MessageStream` (`stream.rs`) | ✅ |
| Streaming input as a prompt | ✅ `AsyncIterable` | ✅ `AsyncIterable<SDKUserMessage>` | `Prompt::Stream(Pin<Box<dyn Stream<Item = String>>>)` (`types/prompt.rs:15-20`) | 🟡 items are `String`, not structured user messages |
| Push input into a live turn | — | ✅ `Query.streamInput()` (`sdk.d.ts:2279-2585`) | ❌ | ❌ TS-only |
| Explicit close | — | ✅ `Query.close()` | drop the `Client`; teardown bounded by `shutdown_grace` | 🟡 no explicit method |
| Warm start | ✅ | ✅ `startup()` (`sdk.d.ts:6795`) | `Client::startup(options)` → `WarmQuery` (`client.rs:316`, `warm.rs:13,46`) | ✅ |

**Verdict:** ✅ on the core loop. The one real gap is `streamInput` — clauders takes its stream at
`query()` time and cannot add turns to an already-running one.

---

## 2. Configuration — `Options`

clauders `Options` carries 45 fields (`options.rs:48-151`). Official TypeScript carries 63, official
Python 45.

### At parity

| Official name | clauders field | Lowered as |
|---|---|---|
| `model` | `model: Option<ModelId>` | `--model` (`argv.rs:100`) |
| `fallbackModel` | `fallback_model` | `--fallback-model` (`argv.rs:176`) |
| `maxTurns` | `max_turns` | `--max-turns` (`argv.rs:134`) |
| `allowedTools` | `allowed_tools` | `--allowed-tools` (`argv.rs:126`) |
| `disallowedTools` | `disallowed_tools` | `--disallowed-tools` (`argv.rs:130`) |
| `permissionMode` | `permission_mode: PermissionMode` | `--permission-mode` (`argv.rs:86`) |
| `canUseTool` | `permission_policy: Option<Arc<dyn PermissionPolicy>>` | `--permission-prompt-tool stdio` (`argv.rs:94-97`) |
| `permissionPromptToolName` | `permission_prompt_tool_name` | `--permission-prompt-tool <name>` (`argv.rs:91`) |
| `mcpServers` | `mcp_servers: Vec<McpServerConfig>` | `--mcp-config` per server (`argv.rs:157-161`) |
| `strictMcpConfig` | `strict_mcp_config` | `--strict-mcp-config` (`argv.rs:180`) |
| in-process MCP servers | `sdk_mcp_servers: SdkMcpRegistry` | `--mcp-config` (`argv.rs:162-165`) |
| `hooks` | `hooks: HookRegistry` | `initialize` payload (`handshake.rs:19-24`) |
| `onElicitation` | `elicitation_policy` | dispatcher route (`dispatch.rs:156`) |
| `agents` | `agents: HashMap<String, AgentDefinition>` | `--agents` JSON (`argv.rs:171`) |
| `agent` | `agent: Option<String>` | `--agent` (`argv.rs:282`) |
| `cwd` | `cwd` | spawn config, not argv |
| `additionalDirectories` | `add_dirs` | variadic `--add-dir` (`argv.rs:184`) |
| `env` | `env: Vec<(String, String)>` | spawn config |
| `continue` / `resume` / `forkSession` / `resumeSessionAt` | `session: SessionControl` | `--continue`, `--resume <id>`, `--fork-session`, `--resume-session-at=<uuid>` (`argv.rs:320-341`) |
| `sessionId` | `session_id` | `--session-id` on new sessions only (`argv.rs:309-315`) |
| `title` | `title` | `initialize` payload (`handshake.rs:28-35`) |
| `persistSession` | `session_persistence` | `--no-session-persistence` (`argv.rs:318`) |
| `settings` | `settings: Option<SettingsSource>` | `--settings <path\|json>` (`argv.rs:192`) |
| `settingSources` | `setting_sources: Vec<SettingSource>` | `--setting-sources=<csv>` (`argv.rs:227`) |
| `skills` | `skills: Option<Skills>` | folded into `--allowed-tools` as `Skill` / `Skill(name)` (`argv.rs:15-29`) |
| `plugins` | `plugins: Vec<PluginSpec>` | `--plugin-dir` / `--plugin-dir-no-mcp` (`argv.rs:233-243`) |
| `sandbox` | `sandbox: Option<SandboxConfig>` | merged into `--settings` (`argv.rs:48-70`) |
| `betas` | `betas: Vec<String>` | `--betas` (`argv.rs:229`) |
| `thinking` | `thinking: Option<ThinkingConfig>` | `--thinking` / `--max-thinking-tokens` / `--thinking-display` (`argv.rs:246-271`) |
| `maxThinkingTokens` | `max_thinking_tokens` | fallback when `thinking` is unset (`argv.rs:272-280`) |
| `effort` | `effort: Option<EffortLevel>` | `--effort` (`argv.rs:216`) |
| `maxBudgetUsd` | `max_budget_usd: Option<BudgetUsd>` | `--max-budget-usd` (`argv.rs:206`) |
| `includePartialMessages` | `include_partial_messages` | `--include-partial-messages` (`argv.rs:211`) |
| `includeHookEvents` | `include_hook_events` | `--include-hook-events` (`argv.rs:214`) |
| `outputFormat` | `output_format: Option<OutputConfig>` | `--json-schema` (`argv.rs:151`) |
| `pathToClaudeCodeExecutable` / `cli_path` | `path_to_executable` | discovery override |
| `maxBufferSize` | `max_buffer_size: Option<NonZeroUsize>` | SDK-side stdout line cap; no flag |
| `stderr` | `stderr: Option<Arc<dyn Fn(&str)>>` | SDK-side per-chunk callback; no flag |

The sandbox merge is worth noting: setting `sandbox` alongside a settings *file path* is rejected with
`AgentError::SandboxWithSettingsPath` (`argv.rs:56`), because the two cannot be combined into one
`--settings` argument. Inline settings merge fine.

### Missing from clauders, present in both official SDKs

| Official name | What it does |
|---|---|
| `tools` | the allowlist in preset form — `string[]` or `{type:'preset', preset:'claude_code'}` |
| `extraArgs` / `extra_args` | arbitrary `Record<string, string \| null>` of extra CLI flags |
| `enableFileCheckpointing` | file checkpointing, the prerequisite for a meaningful `rewindFiles` |
| `sessionStore` | pluggable session-storage backend replacing the on-disk default |
| `sessionStoreFlush` | flush policy for that backend |
| `loadTimeoutMs` | bound on subprocess startup |
| `taskBudget` | API-side task budget in tokens |

`executable_args: Vec<String>` (`options.rs:72`) is adjacent to `extraArgs` but not equivalent — it
prepends raw arguments ahead of the SDK-managed argv (`argv.rs:78`) rather than mapping named flags to
values.

### Missing from clauders, TypeScript only

`abortController`, `toolAliases`, `executable` (`'bun' | 'deno' | 'node'`), `toolConfig`,
`onUserDialog`, `supportedDialogKinds`, `forwardSubagentText`, `planModeInstructions`,
`allowDangerouslySkipPermissions`, `promptSuggestions`, `agentProgressSummaries`, `managedSettings`,
`debug`, `debugFile`, `spawnClaudeCodeProcess`.

Python-only and missing: `debug_stderr`.

### clauders-only

| Field | Purpose | |
|---|---|---|
| `require_min_version` | promote a too-old binary from a warning to a hard error | 🔷 |
| `shutdown_grace` | graceful-exit window before a forced kill | 🔷 |
| `control_request_timeout` | bound on waiting for a correlated control response | 🔷 |

### Present but inert

`max_tokens` (`options.rs:54`) and `user` (`options.rs:120-122`) can be set and are carried on the
struct, but nothing reads them on the CLI runtime. `user` says so in its own doc comment. Neither is
lowered to argv nor sent in the handshake. 🟡 — shape parity, no behaviour.

---

## 3. In-process MCP tools

| Capability | Python | TS | clauders | |
|---|---|---|---|---|
| Define a tool | ✅ `@tool` | ✅ `tool(name, desc, zodShape, handler)` | `tool(name, desc, schema, closure)` and `impl Tool` (`mcp/tool.rs`) | ✅ |
| Create an in-process server | ✅ `create_sdk_mcp_server` | ✅ `createSdkMcpServer` | `SdkMcpServer::builder(name)…build()` | ✅ |
| Registry of servers | implicit | implicit | `SdkMcpRegistry` | ✅ |
| Tool-result content kinds | ✅ | ✅ | `ToolContent::{Text, Image, Audio, ResourceLink, Resource}` (`mcp/tool.rs:21,26,33,40,51`), `#[non_exhaustive]` | ✅ |
| Tool annotations | ✅ | ✅ | `ToolAnnotations` | ✅ |
| JSON-RPC dispatch | ✅ | ✅ | `mcp::router` | ✅ |
| Compile-time argument typing | JSON schema | ✅ Zod shape inference | raw `serde_json::Value` schema | ❌ TS-only |
| Elicitation | ❌ | ✅ `onElicitation` | `ElicitationPolicy::elicit` + two hook events | ✅ matches TS |

**Verdict:** ✅ except Zod-style typed argument inference, which has no direct Rust analogue without a
proc macro. Handlers receive `serde_json::Value` and destructure it themselves.

---

## 4. Hooks

| Aspect | Python | TS | clauders | |
|---|---|---|---|---|
| Registration with a matcher | ✅ | ✅ | `Options::hook(event, matcher, Arc<dyn Hook>)` (`options.rs:425`) | ✅ |
| Full response payload | ✅ | ✅ | `HookOutput { continue_, suppress_output, decision, system_message, reason }` | ✅ |
| Lifecycle observability frames | ✅ | ✅ | `include_hook_events` → `--include-hook-events` | ✅ |
| Event coverage | 31 events | 31 events | **15** | 🟡 |

Official `HOOK_EVENTS` is a 31-member list at `sdk.d.ts:816`. clauders models 15
(`capabilities.rs:22-50`): `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`,
`Stop`, `SubagentStart`, `SubagentStop`, `PreCompact`, `Notification`, `PermissionRequest`,
`SessionStart`, `SessionEnd`, `Setup`, `Elicitation`, `ElicitationResult`.

All 15 are members of the official 31 — including `PostToolUseFailure` and `PermissionRequest`, which
some third-party summaries describe as unofficial. Both appear in `HOOK_EVENTS`; the shipped list is
what settles it.

The 16 not modelled: `PostToolBatch`, `UserPromptExpansion`, `StopFailure`, `PostCompact`,
`PermissionDenied`, `TeammateIdle`, `TaskCreated`, `TaskCompleted`, `ConfigChange`, `WorktreeCreate`,
`WorktreeRemove`, `InstructionsLoaded`, `CwdChanged`, `FileChanged`, `DirectoryAdded`,
`MessageDisplay`.

**Verdict:** 🟡 — the mechanism and payload are at parity, the event vocabulary is at 15 of 31.

---

## 5. Permissions

| Aspect | Python | TS | clauders | |
|---|---|---|---|---|
| `default` / `acceptEdits` / `plan` / `bypassPermissions` / `dontAsk` / `auto` | ✅ | ✅ | all six (`argv.rs:290-297`) | ✅ |
| `canUseTool` callback | ✅ | ✅ | `PermissionPolicy::can_use_tool` | ✅ |
| Allow, rewriting the input | ✅ | ✅ | `Allow { updated_input, updated_permissions }` (`permissions/decision.rs:76-81`) | ✅ |
| Deny with a message | ✅ | ✅ | `Deny { message, interrupt, updated_permissions }` (`decision.rs:83-90`) | ✅ |
| Deny interrupts the turn | ✅ | ✅ | `Deny { interrupt: true }` | ✅ |
| Persist rule updates | ✅ | ✅ | `updated_permissions: Vec<PermissionUpdate>`, forwarded to the binary | ✅ |
| Rich request context | ✅ | ✅ | `PermissionContext` incl. suggestions, `matched_ask_rule`, `request_id` | ✅ |

Rule persistence is the binary's job in every implementation, clauders included — the SDK forwards
the updates and the binary writes them to its settings scopes.

**Verdict:** ✅ across the board.

---

## 6. Subagents

`AgentDefinition` (`subagents/definition.rs:39-69`) models all 13 official fields: `description`,
`prompt`, `tools`, `disallowedTools`, `model`, `maxTurns`, `permissionMode`, `skills`, `memory`,
`mcpServers`, `initialPrompt`, `background`, `effort`. Serialized `camelCase` into the `--agents` JSON
payload, with unset optionals skipped.

One caveat carried in the source itself (`subagents/definition.rs:57-61`): the `mcpServers` element
shape is **unconfirmed**. The official `AgentMcpServerSpec` is undocumented, so each server serializes
as an assumed `{"name": …, "config": …}`; the official element likely inlines transport fields
instead. Re-verify against a live `--agents` round-trip before treating that one field as at parity.

**Verdict:** ✅ on the field set, with `mcpServers`' wire shape unverified.

---

## 7. Sessions

Two mechanisms, and conflating them is easy. Session *control* goes through CLI flags at spawn.
Session *inspection* never touches the subprocess at all —
both official SDKs implement it as plain local file I/O over the `~/.claude/projects/<encoded-cwd>/*.jsonl`
transcripts the CLI itself writes.

### Control — at spawn

| Capability | Python | TS | clauders | |
|---|---|---|---|---|
| Continue the most recent | ✅ | ✅ | `SessionControl::Continue { fork }` → `--continue` | ✅ |
| Resume by id | ✅ | ✅ | `SessionControl::Resume { id, .. }` → `--resume <id>` | ✅ |
| Fork | ✅ | ✅ | `fork: true` → `--fork-session` | ✅ |
| Resume at a message uuid | ❌ | ✅ `resumeSessionAt` | `resume_at` → `--resume-session-at=<uuid>` (`argv.rs:339`) | ✅ matches TS |
| Force a new session's id | ✅ | ✅ | `Options::session_id` → `--session-id` | ✅ |
| Session title | ✅ | ✅ | `Options::title` → `initialize` payload | ✅ |
| Disable persistence | ✅ | ✅ | `session_persistence` → `--no-session-persistence` | ✅ |

### Inspection — local filesystem

| Official function | clauders | |
|---|---|---|
| `listSessions` / `list_sessions` | `SessionArchive::list(ListOptions)` (`sessions/archive.rs:82`) | ✅ |
| `getSessionInfo` / `get_session_info` | `SessionArchive::info` (`archive.rs:59`) | ✅ |
| `getSessionMessages` / `get_session_messages` | `SessionArchive::messages` (`archive.rs:130`) | 🔶 |
| `renameSession` / `rename_session` | `SessionArchive::rename` (`archive.rs:159`) | ✅ |
| `tagSession` / `tag_session` | `SessionArchive::tag` (`archive.rs:185`) | ✅ |
| `deleteSession` (`sdk.d.ts:530`) | ❌ | ❌ |
| `forkSession` (`sdk.d.ts:700`) | ❌ | ❌ |
| `listSubagents` (`sdk.d.ts:1009`) | ❌ | ❌ |
| `getSubagentMessages` (`sdk.d.ts:796`) | ❌ | ❌ |
| `importSessionToStore` (`sdk.d.ts:857`) | ❌ | ❌ |
| `SessionStore` / `InMemorySessionStore` (`sdk.d.ts:894`) | ❌ | ❌ |

`messages()` is marked 🔶 rather than ✅ deliberately: it returns a flat list carrying `parent_uuid`
rather than reconstructing the conversation DAG the way the official implementations do. That was an
explicit decision, not an oversight.

The pluggable `SessionStore` is the largest of these gaps — it is the extension point that lets a
caller replace on-disk transcripts with Redis, S3, or Postgres, and the official Python SDK ships
example backends for all three.

**Verdict:** ✅ on control, ✅ on four of five inspection functions, ❌ on the remaining official
session functions and on the pluggable backend.

---

## 8. System prompt

| Form | Python | TS | clauders | |
|---|---|---|---|---|
| Plain string | ✅ | ✅ | `SystemPromptConfig::Text` → `--system-prompt` (`argv.rs:106`) | ✅ |
| Preset `claude_code` + `append` | ✅ | ✅ | `SystemPromptConfig::Preset` → `--append-system-prompt` (`argv.rs:116`) | ✅ |
| `excludeDynamicSections` | ✅ | ✅ | → `--exclude-dynamic-system-prompt-sections` (`argv.rs:120`) | ✅ |
| Array of strings | — | ✅ `string[]` (`sdk.d.ts:2017`) | ❌ | ❌ TS-only |
| From a file | ✅ `SystemPromptFile` (`types.py:60-64`) | — | ❌ | ❌ Python-only |

`Preset` lowers to `--append-system-prompt` rather than `--system-prompt` because the CLI's built-in
base prompt *is* the `claude_code` preset — replacing it would discard the thing the preset names.

---

## 9. Message frames

| Frame | clauders | |
|---|---|---|
| Assistant | `AssistantMessage` — content, parent_tool_use_id, id, model, role, stop_reason, stop_sequence, usage, uuid, session_id, request_id, timestamp, is_meta, extra (`message.rs:52-82`) | ✅ |
| User | `UserMessage` (`message.rs:154-160`) | ✅ |
| System | `SystemMessage` (`message.rs:169-175`) | ✅ |
| Result | `ResultMessage` — subtype, errors, result, structured_output, is_error, total_cost_usd, stop_reason, usage, session_id, num_turns, model_usage, permission_denials, duration_ms, duration_api_ms, ttft_ms, terminal_reason, uuid, extra (`message.rs:207-265`) | ✅ |
| Partial / stream event | `StreamEvent` (`message.rs:274-277`) | ✅ |
| Unmodelled frame | `Message::Other(Value)` | 🔷 |

`Usage` (`message.rs:290-317`) carries `input_tokens`, `output_tokens`,
`cache_creation_input_tokens`, `cache_read_input_tokens`, `cache_creation`, `server_tool_use`,
`service_tier`, `inference_geo`, and an `extra` catch-all.

Every struct in this list carries an `extra` field, so a field added by a newer binary is retained
rather than dropped.

**Verdict:** ✅ on the taxonomy and the field sets, with a forward-compatibility property neither
official SDK has.

---

## 10. Live control

The official `Query` interface (`sdk.d.ts:2279-2585`) exposes 28 methods. The `Runtime` trait
(`runtime/port.rs:30-200`) carries 23 — `run` plus 22 control and introspection methods — and `Client`
(`client.rs:35-345`) forwards them all, plus four readers over the retained handshake response.

| Official | clauders | |
|---|---|---|
| `interrupt` | `interrupt()` → `Option<InterruptReceipt>` | ✅ |
| `setModel` | `set_model(ModelId)` | ✅ |
| `setPermissionMode` | `set_permission_mode(PermissionMode)` | ✅ |
| `setMcpPermissionModeOverride` | `set_mcp_permission_mode_override(name, mode)` | ✅ |
| `setMaxThinkingTokens` | `set_max_thinking_tokens(tokens, display)` | ✅ |
| `applyFlagSettings` | `apply_flag_settings(Value)` | ✅ |
| `initializationResult` | `initialize_result()` | ✅ |
| `reinitialize` | `reinitialize()` | ✅ |
| `mcpServerStatus` | `mcp_status()` → `McpStatus` | ✅ |
| `reconnectMcpServer` | `reconnect_mcp_server(name)` | ✅ |
| `toggleMcpServer` | `toggle_mcp_server(name, enabled)` | ✅ |
| `setMcpServers` | `set_mcp_servers(Value)` → `SetMcpServersResult` | ✅ |
| `getContextUsage` | `get_context_usage()` → `ContextUsage` | ✅ |
| `usage_EXPERIMENTAL_…` | `get_usage()` → `UsageReport` | ✅ |
| `readFile` | `read_file(path, max_bytes, encoding)` | ✅ |
| `seedReadState` | `seed_read_state(path, mtime)` | ✅ |
| `reloadPlugins` | `reload_plugins()` → `ReloadPluginsResult` | ✅ |
| `reloadSkills` | `reload_skills()` → `ReloadSkillsResult` | ✅ |
| `rewindFiles` | `rewind_files(user_message_id, dry_run)` | ✅ |
| `stopTask` | `stop_task(task_id)` | ✅ |
| `backgroundTasks` | `background_tasks(tool_use_id)` | ✅ |
| `supportedCommands` | `supported_commands()` → `Vec<serde_json::Value>` | 🟡 untyped |
| `supportedModels` | `supported_models()` → `Vec<serde_json::Value>` | 🟡 untyped |
| `supportedAgents` | `supported_agents()` → `Vec<serde_json::Value>` | 🟡 untyped |
| `accountInfo` | `account_info()` → `serde_json::Value` | 🟡 untyped |
| `streamInput` | ❌ | ❌ |
| `close` | drop the `Client` | 🟡 |

The four untyped readers are the honest gap in this table. Official returns `SlashCommand[]`,
`ModelInfo[]`, `AgentInfo[]` and `AccountInfo`; clauders hands back raw JSON, so a caller writes
`command.get("name").and_then(Value::as_str)` instead of `command.name`. The data is all there — the
types are not.

`rewind_files` is wired but its usefulness depends on `enableFileCheckpointing`, which clauders does
not expose (§2).

**Verdict:** ✅ on 21 of the 28 official operations, 🟡 on 5 (four untyped returns, plus `close`), ❌ on
`streamInput`.

---

## Where clauders goes beyond both official SDKs

Not parity claims. These exist because a Rust client embedded in a long-lived process has needs the
scripting SDKs do not.

**Unmodelled frames cannot fail a turn.** `Message::Other(Value)` catches any frame kind this release
does not model. Given that the `claude` binary ships continuously and carries no protocol version
negotiation, this is the difference between a log line and a broken deployment when a new frame type
appears. Every frame struct additionally carries an `extra` field for unmodelled *fields*.

**Bounded process lifecycle.** `shutdown_grace` bounds teardown, `control_request_timeout` bounds a
control round-trip, and `require_min_version` turns a too-old binary into an immediate error instead
of a confusing failure later. `agent::process` is tested against a purpose-built child
(`src/bin/clauders-agent-testchild.rs`) whose flags provoke EOF-ignoring, stderr floods, and forked
grandchildren, so the no-zombie and no-orphan properties are asserted rather than assumed.

**A test seam that needs no binary.** `MockRuntime` implements the same `Runtime` trait as
`CliRuntime`, so session and client logic is exercised end to end with no `claude` binary, no
credentials, and no network. Neither official SDK has an equivalent.

**Values are parsed once.** `BudgetUsd`, `SessionId`, `MessageId`, `ModelId` and the rest validate at
construction and are proof thereafter, where the official SDKs pass strings and floats.

**Stricter elicitation validation.** `mcp_server_name` is structurally required
(`protocol/frames.rs:242`) where `sdk.mjs` reads it unvalidated. See
[divergences.md](../divergences.md).

---

## Deliberate divergences

Recorded in [divergences.md](../divergences.md) rather than here, so the reasoning lives in one place.
The Agent SDK entries are: required `mcp_server_name` on elicitation, the three process-hygiene
options, the two inert fields, and `SessionArchive::messages` returning a flat list with
`parent_uuid` rather than a reconstructed conversation DAG.

---

## Remaining gaps, ranked

Ranked by how likely a caller is to hit them.

1. **Hook events — 15 of 31.** The commonest events are covered; the 16 missing are mostly newer and
   narrower. Cheap to add individually.
2. **The pluggable `SessionStore`.** The extension point for replacing on-disk transcripts with a
   real backend. Both official SDKs have it and Python ships Redis, S3 and Postgres examples.
3. **Typed `supported*` and `accountInfo` returns.** The data arrives; only the types are missing.
4. **`streamInput`.** Adding turns to an already-running session. TypeScript only.
5. **Four session functions** — `deleteSession`, `forkSession`, `listSubagents`,
   `getSubagentMessages`.
6. **Seven `Options` fields both official SDKs have** — `tools` preset form, `extraArgs`,
   `enableFileCheckpointing`, `sessionStore`, `sessionStoreFlush`, `loadTimeoutMs`, `taskBudget`.
7. **System prompt from a file** (Python) or **as an array** (TypeScript).
8. **`AgentDefinition.mcpServers` wire shape** — implemented against an assumed element shape, needs
   a live round-trip to confirm.
9. **Zod-style typed tool arguments.** TypeScript only; no direct Rust analogue without a macro.

---

## Methodology

**clauders side** — read from source under `crates/clauders/src/agent/`: `options.rs`,
`client.rs`, `runtime/port.rs`, `runtime/cli/{argv,handshake,dispatch,demux,discovery,runtime}.rs`,
`message.rs`, `content.rs`, `capabilities.rs`, `hooks.rs`, `permissions/`, `subagents/`,
`elicitation/`, `mcp/`, `sessions/`, `warm.rs`, `process/`, `protocol/`, `types/`. Authoritative.

**Official side** — read from the shipped artifacts named at the top, not from documentation. Where
`sdk.d.ts` and `sdk.mjs` disagree, `sdk.mjs` wins and the row says so. Python-specific rows come from
the sdist source.

**Marks judge behaviour, not type shape.** A row is ✅ only when clauders produces the same observable
result for the same input, not merely when a same-named type exists.

**Absence claims name their search.** Every ❌ above was established by searching the clauders tree for
the capability and finding nothing, with a sibling capability found by the same search as the control.

The official SDKs iterate quickly — the TypeScript package alone moved from 0.3.215 to 0.3.221 during
the period this document covers. Re-verify against the pinned versions before treating any single ❌
as durable.
