# Agent SDK — development phases

**Set by the author on 2026-07-20. This split is fixed. Do not re-scope, merge, reorder, or
renumber it.** Every phase gets exactly one spec and one plan, written and executed in order.

This supersedes the earlier `phase-5-parity-completion.md` workstream numbering (WS 1–9). References
to "WS 9" below are historical only — the work itself is Phase 2 here.

| Phase | Scope | Items | Estimate |
| --- | --- | --- | --- |
| 1 | Fix the 7 reply-reading bugs | 7 | ~1 day |
| 2 | Mid-session control operations (formerly WS 9) | 20 | 2–3 days |
| 3 | Session file operations | 5 | 2–3 days |
| 4 | Resume-at-a-point, dropped message data, handler-hang check | 3 | ~1.5 days |
| 5 | Startup options not wired | 7 | ~1 day |

Deliverable per phase: **1 spec + 1 plan**, then execution.

---

## Phase 1 — fix the 7 reply-reading bugs

Every one was proved on 2026-07-20 by running code against the decoder, or by probing the live
`claude` binary v2.1.215 and reading `@anthropic-ai/claude-agent-sdk@0.3.215`.

1. **Unknown content block destroys the whole assistant message.**
   `content.rs:17` has no catch-all variant. The binary contains `redacted_thinking`,
   `web_search_tool_result`, `mcp_tool_use`, `mcp_tool_result`, `container_upload` — none modelled.
   Proved: an assistant frame carrying `text` + `redacted_thinking` decodes to `Message::Other`,
   losing the text as well. The doc comment at `content.rs:10` also misstates the failure mode (it
   claims `AgentError::Protocol`; the actual path is `Message::Other` via `codec.rs:115`).

2. **Error results lose their diagnostics.**
   `message.rs:91` models neither `subtype` nor `errors`. `sdk.d.ts`'s `SDKResultError` declares
   `errors: string[]` and four error subtypes; `sdk.mjs` reads `e.errors` for exactly this case.
   Proved: an `error_max_turns` frame decodes to `result: ""` with no way to recover the reason.

3. **The capability manifest is never populated.**
   `capabilities.rs:55-61` binds `protocol_version`, `supported_hook_events`,
   `supported_control_methods`. The live `initialize` control response returns
   `account, agents, available_output_styles, commands, ide_rc_auto_enable_gate, models,
   output_style, pid, remote_control_auto_enable, remote_control_auto_on_by_default` — none of ours.
   `supported_hook_events` and `supported_control_methods` each return **0 hits** across the 236 MB
   binary (search proven discriminating: `server_tool_use` returns 97). The real capability list
   arrives on the `system`/`init` **message frame** — live probe returned
   `capabilities: ["interrupt_receipt_v1", "msg_lifecycle_v1"]`. Consequence: `supports_hook()` is
   permanently `false` and `warn_unsupported_hooks` warns about every registered hook every session.

4. **Valid elicitation requests are rejected.**
   `frames.rs:178-184` requires `elicitation_id` and `mode`; `sdk.d.ts`'s
   `SDKControlElicitationRequest` marks both optional. Proved: a request without `mode` is rescued to
   `Malformed` and answered with an error instead of reaching the elicitation policy. Two existing
   tests (`elicitation_missing_mode_rescues_to_malformed`,
   `elicitation_missing_elicitation_id_rescues_to_malformed`) currently lock the wrong behaviour in
   and must be rewritten.

5. **`can_use_tool` drops two fields the official SDKs forward.**
   `sdk.mjs` passes `permission_suggestions` (→ `suggestions`) and `matched_ask_rule` (→
   `matchedAskRule`) to the permission callback. `frames.rs:128-155` parses neither.

6. **`control_cancel_request` is ignored.**
   `sdk.d.ts` types it as `{ type: 'control_cancel_request'; request_id: string }`; `sdk.mjs` aborts
   the in-flight handler on receipt. Proved: ours falls through to `Message::Other` in the caller's
   message stream and the handler keeps running. Requires per-request cancellation state in the
   dispatcher.

7. **`keep_alive` frames leak into the caller's message stream.**
   `sdk.d.ts` types `SDKKeepAliveMessage`; `sdk.mjs` skips it. Proved: ours arrives as
   `Message::Other`.

---

## Phase 2 — mid-session control operations

Formerly WS 9. The prior spec
(`~/.airsstack/cc/plugins/sdd/airsstack-c82d435a/specs/2026-07-20-clauders-live-control-tail.md`)
is **superseded** — it carries ~11 uncorrected review findings, and its startup-data section assumes
the capability parsing works, which Phase 1 bug 3 disproves. Re-spec from the item list below.

Scope was derived from the binary's stdin control chain (50 subtypes, exhaustively enumerated), not
from the `SDKControlRequestInner` union — the union also contains bridge-only and inbound members.

**Live MCP control (5)**

| Method | subtype | request fields (wire spelling) | success payload |
| --- | --- | --- | --- |
| `toggle_mcp_server` | `mcp_toggle` | `serverName`, `enabled` | *(empty)* |
| `reconnect_mcp_server` | `mcp_reconnect` | `serverName` | *(empty)* |
| `set_mcp_servers` | `mcp_set_servers` | `servers` | `{added, removed, errors}` |
| `set_mcp_permission_mode_override` | `set_mcp_permission_mode_override` | `serverName`, `mode` | `{warning?}` |

Plus: widen `ServerStatus` — add `serverInfo?`, `error?`, `config?`, and model `status` as a
`#[non_exhaustive]` enum over the closed upstream set (`connected`, `failed`, `needs-auth`,
`pending`, `disabled`).

**Tasks, turns, workspace (6)**

| Method | subtype | request fields | success payload |
| --- | --- | --- | --- |
| `stop_task` | `stop_task` | `task_id` | *(empty)* |
| `background_tasks` | `background_tasks` | `tool_use_id?` | `{backgrounded}` |
| `rewind_files` | `rewind_files` | `user_message_id`, `dry_run?` | `{canRewind, error?, filesChanged?, insertions?, deletions?}` |
| `read_file` | `read_file` | `path`, `max_bytes?`, `encoding?` | `{contents, absPath, truncated?, encoding?}` |
| `seed_read_state` | `seed_read_state` | `path`, `mtime` | *(empty)* |

Plus: `interrupt` returns a receipt. `SDKControlInterruptResponse = { still_queued: string[] }`.
`sdk.mjs` reads it by **field presence only**, with a defensive filter — no capability gate:

```js
let t=(await this.request({subtype:"interrupt"})).response?.still_queued;
return Array.isArray(t)?{still_queued:t.filter((r)=>typeof r==="string")}:void 0
```

Signature becomes `Result<Option<InterruptReceipt>, AgentError>`; `None` when absent or not an array
of strings.

**Introspection and config (8)**

| Method | subtype | request fields | success payload |
| --- | --- | --- | --- |
| `get_context_usage` | `get_context_usage` | *(none)* | large — `sdk.d.ts:3028-3118` |
| `get_usage` | `get_usage` | *(none)* | large — `sdk.d.ts:3151-3359` |
| `reload_plugins` | `reload_plugins` | *(none)* | `{commands, agents, plugins[], mcpServers, error_count}` |
| `reload_skills` | `reload_skills` | *(none)* | `{skills}` |
| `apply_flag_settings` | `apply_flag_settings` | `settings` | *(empty)* |
| `set_max_thinking_tokens` | `set_max_thinking_tokens` | `max_thinking_tokens?`, `thinking_display?` | *(empty)* |

Plus: retain the `initialize` response instead of discarding it (`handshake.rs:61` currently keeps
only the capability struct), and expose six accessors over it — `initialize_result`,
`supported_commands`, `supported_models`, `supported_agents`, `account_info`, `reinitialize`. The
first five cost zero wire traffic; only `reinitialize` re-sends `initialize`. Verbatim, `sdk.mjs`:

```js
initializationResult(){return this.initialization}
reinitialize(){return Rr("sdk_reinitialize",()=>this.initialize())}
async supportedCommands(){return(await this.initialization).commands}
async supportedModels(){return(await this.initialization).models}
async supportedAgents(){return(await this.initialization).agents}
```

**Warm start (1)**

`startup()` pre-spawns the subprocess and blocks on the initialize round-trip so the first query has
no startup latency. `sdk.d.ts:6665`:

```ts
export declare function startup(_params?: {
    options?: Options;
    initializeTimeoutMs?: number;
}): Promise<WarmQuery>;
```

`WarmQuery` is single-shot (`query()` throws on a second call), `close()` is idempotent, and it
implements `AsyncDisposable`. TypeScript-only; Python has no equivalent. Process lifecycle, not a
control request.

### Wire-mapping rule (carries into every phase)

The binary's field naming is inconsistent by design — `serverName` and `enabled` are camelCase while
`task_id`, `user_message_id`, `max_bytes`, `max_thinking_tokens` are snake_case, in the same
protocol. A container-level `rename_all` would silently break half the operations.

1. Rust field names are always idiomatic snake_case.
2. Every field whose wire name differs carries an explicit `#[serde(rename = "…")]`. The attribute's
   presence is the greppable marker of divergence; its absence asserts wire == Rust.
3. **`#[serde(default)]` only where the official type marks the field optional (`field?:`).** Never
   on a required field.
4. Optionality mirrors the official type exactly: `field?: T` → `Option<T>`; `field: T` → `T`.

Rule 3 is not stylistic. `McpStatus::servers` bound the key `servers` while the binary sends
`mcpServers`; `#[serde(default)]` supplied an empty list with no error, so `mcp_status()` reported
zero servers unconditionally. Fixed 2026-07-20.

### Testing rule (carries into every phase)

**Every response type ships a round-trip test against a verbatim wire fixture** — a payload copied
from `sdk.d.ts`, `sdk.mjs`, or a live binary probe, never hand-written from memory. A hand-written
fixture encodes the same assumption as the code it tests, so it cannot catch a naming mistake. The
pre-existing `mcp_status` test asserted `ServerStatus` from an invented payload and passed while the
envelope was broken.

Where a fixture is expected to produce a non-empty result, assert non-emptiness explicitly — an
empty-collection assertion passes under exactly that bug class.

---

## Phase 3 — session file operations

Five operations both official SDKs implement as **plain local file I/O** over
`~/.claude/projects/<encoded-cwd>/*.jsonl`, using `require("fs/promises")`. They never touch the
control protocol. The `claude` binary embeds the same JS SDK module and the same implementation.

1. `listSessions`
2. `getSessionMessages`
3. `getSessionInfo`
4. `renameSession`
5. `tagSession`

`SDKSessionInfo` (from `sdk.d.ts`) is the metadata shape returned by `listSessions` and
`getSessionInfo`: `sessionId`, `summary`, `lastModified`, `fileSize?`, `customTitle`, …

These were cut from WS 8 on a false premise: a grep of the binary's control protocol found no session
subtypes and concluded "no parity path". The official SDKs never used the control protocol for this,
so absence there proved nothing. This also falsifies `vision-and-strategy.md` §5's stated reason for
removing the native `SessionStore` ("a superset with no official counterpart") — both official SDKs
expose it.

Note: `renameSession` is *additionally* reachable as a live `rename_session` control subtype on the
stdin chain, but that renames only the **running** session and has no official-SDK counterpart. The
official `renameSession` takes an arbitrary session id and edits the file. Do not conflate them.

---

## Phase 4 — resume-at-a-point, dropped message data, handler-hang check

**1. Resume at a point.** `resumeSessionAt` — TypeScript-only, lowers to the CLI's
`--resume-session-at` flag. The flag is `.hideHelp()`-hidden, so it does not appear in
`claude --help`; it requires `--resume`.

**2. Data we drop from inbound messages.**

- `AssistantMessage` lifts only `content` out of the wire's nested `message` object. The live probe
  shows that object also carries `id`, `model`, `role`, `stop_reason`, `stop_details`,
  `stop_sequence`, `usage`, `context_management`, `diagnostics`. The frame itself also carries
  `request_id`, `timestamp`, `uuid`, `session_id`.
- `ResultMessage` drops `modelUsage` (per-model cost breakdown), `permission_denials`, `duration_ms`,
  `duration_api_ms`, `ttft_ms`, `terminal_reason`, `uuid`, `api_error_status`, `fast_mode_state`.
  (`subtype` and `errors` are Phase 1 bug 2 — already covered there, do not duplicate.)
- The live `usage` object is richer than our four counters: `cache_creation`,
  `cache_creation_input_tokens`, `cache_read_input_tokens`, `inference_geo`, `input_tokens`,
  `iterations`, `output_tokens`, `server_tool_use`, `service_tier`, `speed`.

**3. Handler-hang check.** Recorded 2026-07-19, never verified: `dispatch.rs:120` awaits the handler
outcome before `write_response`, so a `PermissionPolicy` / `Hook` / `ElicitationPolicy` that never
returns may hang the binary, which blocks on the correlated response. The TypeScript changelog hints
the official SDKs bound this. **Source-verify before designing a fix** — the claim that they bound it
is not verified.

---

## Phase 5 — startup options not wired

Seven `Options` fields the official SDKs accept and `clauders` does not lower to argv or to the
`initialize` payload:

1. `setting_sources` — which settings files to load (and, with it, project `CLAUDE.md` / on-disk agent
   definitions)
2. `skills`
3. `plugins`
4. `sandbox`
5. `betas`
6. `thinking` / `max_thinking_tokens` as a startup option (distinct from Phase 2's mid-session
   `set_max_thinking_tokens` control request)
7. `agents` from disk

---

## Standing rules for every phase

- **Cite or drop it.** Every wire shape written into a spec, a plan, or code carries a `file:line` or
  a byte offset read in that task. Prefer the shipped artifact (`sdk.d.ts`, `sdk.mjs`, the Python
  sdist, the binary, a live probe) over documentation, and documentation over recollection.
- **An absence claim needs a search that could have found the thing.** Name the exact command, and
  confirm the method by checking it finds a sibling known to be present.
- **A passing test proves nothing until it has been seen to fail.** Break the fix, watch it go red,
  restore.
- Definition of Done green before any phase closes: `cargo fmt --check`;
  `cargo clippy -p clauders --all-features --all-targets -- -D warnings`;
  `cargo test -p clauders --all-features --all-targets`; `cargo test -p clauders --all-features --doc`;
  `cargo doc -p clauders --no-deps --all-features` — all zero-warning.

## Out of scope for all five phases

Recorded so it is not rediscovered as a surprise:

- The 17 control operations TypeScript emits but does not publish (`get_settings`, `set_cwd`,
  `cancel_async_message`, `remote_control`, `submit_feedback`, `generate_session_title`,
  `side_question`, `ultrareview_launch`, `message_rated`, `channel_enable`, `mcp_authenticate`,
  `mcp_clear_auth`, `mcp_oauth_callback_url`, `claude_authenticate`, `claude_oauth_callback`,
  `claude_oauth_wait_for_completion`). Stripped from `sdk.d.ts` by api-extractor — the SDK's own
  signal that they are not public API. 16 of the 17 have no published response type.
- `list_models`, `mcp_call`, and `rename_session` as control requests — typed in `sdk.d.ts` but with
  zero SDK callers in either language.
- `rewind_conversation`, `end_session`, `get_binary_version`, `get_session_cost`,
  `get_workspace_diff`, `get_plan`, `stage_file`, `register_repo_root`, `add_directory`,
  `file_suggestions` — on the control chain, no SDK surface in either language.
- The inbound control subtypes `request_user_dialog`, `oauth_token_refresh`, and
  `host_auth_token_refresh`. We currently answer all three with an error control response; the
  official SDK deliberately sends **nothing** for `request_user_dialog` (its handler returns a
  `Symbol("suppressControlResponse")` sentinel). A known divergence, deliberately deferred.
- `transcript_mirror` frames, which the official SDK batches to a file and we forward to the caller.
- The other two pillars named in `vision-and-strategy.md` — the Messages API and Managed Agents.
  Neither was assessed on 2026-07-20; their state is unknown, which is not a claim that they are fine.
