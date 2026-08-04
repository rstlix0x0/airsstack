# Messages API — parity with the official Anthropic base SDKs

What `clauders::messages` (plus `clauders::models` and `clauders::types`) does and does not match in
Anthropic's official base SDKs — the raw `POST /v1/messages` clients, a different product from the
Claude Agent SDK covered in [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md).

**Pinned versions**

| Side | Artifact |
|---|---|
| TypeScript | `@anthropic-ai/sdk@0.115.0` — `resources/messages/messages.d.ts`, `resources/messages/batches.d.ts`, `resources/models.d.ts`, `client.d.ts` |
| Python | `anthropic-sdk-python` @ `3c8bdf14bc55377262f11d6c34b893834a02b3fc` (release 0.117.0) — **not re-verified against a local copy**; the package was not available. Rows resting on Python-only evidence say so. |
| Go (tiebreaker) | `anthropic-sdk-go` `messageutil.go` @ `0ce94bd583a556abfc18ccde1e132be5fd9e32f4` — consulted only where Python and TypeScript disagree. Also not re-verified locally. |
| REST reference | `platform.claude.com/docs/en/api/messages`, `.../build-with-claude/{streaming,vision,refusals-and-fallback,handling-stop-reasons}`, `.../api/models-list` |
| clauders | `crates/clauders/src/`; every `file:line` citation resolves against that tree |

TypeScript paths below are relative to the `0.115.0` tarball root; Python paths to `src/anthropic/`.

**Parity is graded on behaviour, not on type shape.** A row is ✅ only when clauders produces the same
observable result for the same server response — not merely when a same-named type exists. That
That distinction is the point: comparing type surfaces against prose documentation produces false
✅s, because a type can be modelled correctly and still behave wrongly at runtime.

---

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Full parity — equivalent capability, equivalent runtime behavior |
| ⚠️ | **Defect** — the capability is modelled but behaves incorrectly at runtime (silent data loss or hard failure) |
| 🔶 | **Deliberately different** — the behaviour is chosen and does not match the pinned SDKs. A caller porting from Python or TypeScript will observe a difference. Used where the official SDKs disagree with each other and clauders had to pick one. See [`../divergences.md`](../divergences.md). |
| 🟡 | Partial — core exists, narrower than official |
| ❌ | Absent in clauders |
| — | Not applicable |

There are no ⚠️ rows at present; the mark is kept in the legend because it outranks ❌ — a missing
feature degrades gracefully, wrong behaviour does not.

---

## 0. Scorecard

| Area | Status |
|---|---|
| Messages create / count-tokens | ✅ parity |
| Batches (full CRUD) | ✅ parity |
| Custom tools (+ `strict`, all 4 `tool_choice`) | ✅ parity |
| Prompt caching (per-block, TTL tiers, usage counters) | ✅ parity (minus top-level auto-place) |
| JSON-schema structured output | ✅ parity (minus typed-parse helper) |
| System prompt (string + segments + per-segment cache) | ✅ parity |
| **Streaming accumulation** | ✅ parity on all six modelled delta kinds, including `input_json_delta` buffering for both `ToolUse` and `ServerToolUse` blocks (accumulator.rs:222-225, 305-309) — with five deliberate divergences, see [`../divergences.md`](../divergences.md) |
| **Forward compatibility (every server-decoded enum)** | ✅ parity — payload-retaining unknown arm on all ten |
| **`thinking` / `output_config.effort`** | ✅ parity — both request params present |
| **Response diagnostics (`container`, `stop_details`, typed `pause_turn`, usage sub-objects)** | ✅ parity (§8, §8.1, §8.3) |
| **`message_delta` usage merge (input-side counters, `stop_details`)** | ✅ parity — overwrite-cumulative, matching Python/TypeScript (§4.4) |
| Response content-block taxonomy (12 official response members) | ✅ 12 of 12 |
| Request content-block taxonomy (17 official param members) | 🟡 6 of 17 — vision and PDF present; `search_result` and `mid_conversation_system` absent |
| Models API (`capabilities`, `max_input_tokens`, `max_tokens`) | ✅ parity — one divergence: `context_management`'s dated strategies are a flattened map, not named fields (§9.1) |
| GA request params (`service_tier`, `inference_geo`, `container`, top-level `cache_control`) | ✅ parity — all four are builder setters, serialized when set (§2) |
| Server-side & Anthropic-defined tool *definitions* (18 of 19 `ToolUnion` members) | ❌ behind |
| Files API / context management / MCP connector / typed `parse()` helper | ❌ behind |
| Citations on responses (`TextBlock.citations`, `citations_delta`) | ✅ parity |
| Batch list pagination (`BatchListParams`) | ❌ `list()` takes no arguments |
| Legacy Text Completions (`client.completions`) | ❌ — deprecated upstream, not pursued |
| The entire `beta.*` namespace | — out of scope, see below |

**One-line summary:** clauders is at genuine parity on the non-streaming text-and-custom-tools core —
create, count-tokens, batches, caching, structured output, system prompts — and on streaming
accumulation and forward compatibility. The response content-block taxonomy is complete at 12 of 12. What remains is request-side block
breadth (6 of 17), the server-side and Anthropic-defined tool tier, and a handful of independent
surfaces listed in the ranked gaps.

**On the beta namespace.** The official base SDK ships `beta.{agents,sessions,environments,vaults,
memory_stores,skills,tunnels,deployments,files,user_profiles,webhooks}`. Most of that is the Managed
Agents product, which is a separate clauders pillar with no code yet — see
[`../architecture.md`](../architecture.md). It is not scored here. The one beta resource that belongs
to *this* pillar is the Files API, which is listed as a gap above.

---

## 1. Resources & endpoints

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| `POST /v1/messages` | ✅ | ✅ | `messages::MessagesResource::create` (resource.rs:82) | ✅ |
| Streaming create (SSE) | ✅ | ✅ | `MessagesResource::stream` (resource.rs:172) | ✅ transport / ✅ accumulation (`MessageAccumulator`) — index policy follows Go, see §4.3 |
| `POST /v1/messages/count_tokens` | ✅ | ✅ | `count_tokens` (resource.rs:287) | ✅ |
| Message Batches — create/get/list/results/cancel/delete | ✅ | ✅ | `batches::BatchesResource` (all six) | ✅ |
| `GET /v1/models`, `GET /v1/models/{id}` | ✅ | ✅ | `models::ModelsResource::{list,get}` | ✅ endpoints and payload — see §9 |
| Files API (`/v1/files`) | ✅ (beta) | ✅ (beta) | ❌ | ❌ |

`count_tokens` projects `model` / `messages` / `system` / `tools` / `tool_choice` / `thinking` /
`output_config` (`token_counting.rs`), matching the endpoint's accepted set apart from
`cache_control`. `thinking` in particular
must be forwarded because it changes the resulting count.

---

## 2. Request parameters

The official GA (non-beta) parameter set, verified identical in both SDKs
(`types/message_create_params.py:MessageCreateParamsBase`, `resources/messages/messages.ts:3055`):

| Param | Python | TS | clauders | Status |
|---|---|---|---|---|
| `model` | ✅ | ✅ | `model: ModelId` (request.rs:147) | ✅ |
| `max_tokens` | ✅ | ✅ | `max_tokens: MaxTokens` (request.rs:149) | ✅ — `0` accepted; see §2.3 |
| `messages` | ✅ | ✅ | `messages: Vec<InputMessage>` (request.rs:151) | 🟡 — no `system` role; see §3 |
| `system` | ✅ | ✅ | `system: Option<SystemPrompt>` (request.rs:154) | ✅ |
| `stop_sequences` | ✅ | ✅ | (request.rs:166) | ✅ |
| `metadata` | ✅ | ✅ | `Metadata { user_id }` (request.rs:169) | ✅ |
| `stream` | ✅ | ✅ | hidden, resource-managed (request.rs:183) | ✅ |
| `tools` / `tool_choice` | ✅ | ✅ | (request.rs:172-175) | ✅ for custom tools — see §6 |
| `output_config.format` | ✅ | ✅ | `OutputConfig` (request.rs:178, structured_outputs.rs:43) | ✅ |
| `output_config.effort` | ✅ `low\|medium\|high\|xhigh\|max` | ✅ same | `EffortLevel` (structured_outputs.rs:52) | ✅ |
| `thinking` | ✅ 3 variants | ✅ 3 variants | `ThinkingConfig` (request.rs:181) | ✅ |
| `temperature` | ✅ *(`@deprecated` in TS)* | ✅ *(`@deprecated`)* | `Temperature`, `#[deprecated]` (request.rs:157, 343) | ✅ — see §2.2 |
| `top_p` | ✅ *(`@deprecated`)* | ✅ *(`@deprecated`)* | `TopP`, `#[deprecated]` (request.rs:160, 361) | ✅ — see §2.2 |
| `top_k` | ✅ *(`@deprecated`)* | ✅ *(`@deprecated`)* | `TopK`, `#[deprecated]` (request.rs:163, 377) | ✅ — see §2.2 |
| **`cache_control`** (top-level auto-place) | ✅ | ✅ | `MessageRequest.cache_control: Option<CacheControl>` (request.rs:252), `.cache_control()` builder setter (request.rs:586) | ✅ — see §2.4 |
| **`service_tier`** (`auto` \| `standard_only`) | ✅ | ✅ | `RequestServiceTier` (request.rs:73-78, 242, 565) | ✅ — see §2.4 |
| **`inference_geo`** | ✅ | ✅ | `InferenceGeo` newtype (request.rs:86-100, 245, 572) | ✅ — see §2.4 |
| **`container`** | ✅ | ✅ | `ContainerId` newtype (request.rs:105-119, 248, 579) | ✅ — see §2.4 |
| `user_profile_id` (sent as `anthropic-user-profile-id` header) | ✅ | ✅ | ❌ | ❌ |
| `betas` / `anthropic-beta` header | ✅ | ✅ | multi, comma-joined (resource.rs:114-122) | ✅ |

Beta-gated params, absent from clauders, listed for completeness: `mcp_servers`, `context_management`,
`fallbacks`, `fallback_credit_token`, `speed`, `diagnostics`, `output_config.task_budget`.

### 2.1 ✅ `thinking` and `effort`

`MessageRequest` carries a `thinking: Option<ThinkingConfig>` field (request.rs:181), set through the
builder's `.thinking(ThinkingConfig)` method (request.rs:469). `output_config.effort` is set through
`.effort(EffortLevel)` (request.rs:462) or through `.output_config(OutputConfig)` (request.rs:452),
sharing the `EffortLevel` type with the Agent SDK pillar. Adaptive thinking can now be configured or
disabled, and `effort` can be set, on every current-generation model. `count_tokens` forwards both
parameters as well.

Official shape, identical in both SDKs (`types/thinking_config_param.py`, `messages.ts:1826`):

| Variant | Fields |
|---|---|
| `{"type": "adaptive"}` | optional `display: "summarized" \| "omitted"`; **no** `budget_tokens` |
| `{"type": "disabled"}` | `type` only — **no** `display` |
| `{"type": "enabled"}` | required `budget_tokens` (≥1024, `< max_tokens`); optional `display` |

`display` defaults to `summarized` on every model — the default is not model-dependent. Both SDK
docstrings (`types/thinking_config_enabled_param.py`, `messages.ts:1774-1812`) and the REST
reference state this. A caller who wants thinking redacted must set `omitted` explicitly.

### 2.2 ✅ Sampling params now carry the rejection semantics

`temperature` / `top_p` / `top_k` are first-class builder methods (request.rs:349, 366, 380) that now
carry `#[deprecated(note = …)]` (request.rs:343, 361, 377) stating the exact failure mode: post-Opus-4.6
models accept only `temperature == 1.0`, only `top_p >= 0.99`, and reject any `top_k` with a 400. This
matches the TypeScript SDK's own `@deprecated` annotation (`messages.ts:3055` block), which documents
the same failure mode.

clauders still carries the newtype validators (`Temperature::new` rejects out-of-range or NaN input,
numeric.rs:73-77) for the values these setters still accept. The rejection semantics are now signalled
at the call site: a caller who uses any of the three sees a compiler `deprecated` warning quoting the
failure mode, not just a docs-only note.

### 2.3 ✅ `max_tokens: 0` is accepted

`MaxTokens::new` is infallible (`pub const fn new(n: u32) -> Self`, numeric.rs:34-36); `InvalidMaxTokens`
has been deleted from the crate. Official TS documents `max_tokens` as *"set to `0` to pre-warm prompt
cache without generating"* (`messages.ts:3055`), and the prompt-caching guide uses `max_tokens: 0` as
the canonical cache pre-warm call. clauders now serializes that call onto the wire like any other value.

### 2.4 ✅ `service_tier` / `inference_geo` / `container` / top-level `cache_control`

`MessageRequest` carries all four GA fields (request.rs:240-252), each gated with
`#[serde(skip_serializing_if = "Option::is_none")]` so an unset field never appears on the wire. Each
has a dedicated builder setter: `.service_tier(RequestServiceTier)` (request.rs:565), `.inference_geo
(InferenceGeo)` (request.rs:572), `.container(ContainerId)` (request.rs:579), and `.cache_control
(crate::types::CacheControl)` (request.rs:586).

`RequestServiceTier` is a two-variant enum, `#[serde(rename_all = "snake_case")]`, serializing `Auto` →
`"auto"` and `StandardOnly` → `"standard_only"` (request.rs:71-78), tested at
`request_service_tier_serializes_snake_case` (request.rs:778). `InferenceGeo` and `ContainerId` are
`#[serde(transparent)]` newtypes over `String` (request.rs:84-119), serializing as a bare JSON string,
tested at `inference_geo_and_container_id_serialize_transparently` (request.rs:790). Top-level
`cache_control` reuses the existing per-block `crate::types::CacheControl` type (request.rs:252) rather
than introducing a parallel type — the same `ephemeral` breakpoint shape already used on content
blocks, tools, and system segments (§7).

---

## 3. Content blocks

The official SDKs use **two different unions** for the two directions. Conflating them is a common
mistake and the source of a runtime failure class described in §3.3.

### 3.1 Response union — `ContentBlock`, 12 members

`types/content_block.py:ContentBlock` (Python, discriminated on `type`) and `messages.ts:847` (TS) —
identical membership:

| # | Official member | clauders | Status |
|---|---|---|---|
| 1 | `text` | `TextBlock` (content/text.rs:18) | ✅ — carries `citations: Option<Vec<TextCitation>>` (content/text.rs:28, content/citation.rs:25) |
| 2 | `thinking` | `ThinkingBlock` (content/text.rs:71) | ✅ |
| 3 | `redacted_thinking` | `RedactedThinkingBlock` (content/server_tool.rs:62) | ✅ |
| 4 | `tool_use` | `ToolUseBlock` (tools.rs:108) | ✅ |
| 5 | `server_tool_use` | `ServerToolUseBlock` (content/server_tool.rs:69) | ✅ — `name` closed to the seven official server-tool names (`ServerToolName`, content/server_tool.rs:43); an unrecognized `name` falls to `ContentBlock::Unknown` rather than failing the block |
| 6 | `web_search_tool_result` | `WebSearchToolResultBlock` (content/server_tool.rs:90) | ✅ |
| 7 | `web_fetch_tool_result` | `WebFetchToolResultBlock` (content/server_tool.rs:102) | ✅ |
| 8 | `code_execution_tool_result` | `CodeExecutionToolResultBlock` (content/server_tool.rs:114) | ✅ |
| 9 | `bash_code_execution_tool_result` | `BashCodeExecutionToolResultBlock` (content/server_tool.rs:123) | ✅ |
| 10 | `text_editor_code_execution_tool_result` | `TextEditorCodeExecutionToolResultBlock` (content/server_tool.rs:132) | ✅ |
| 11 | `tool_search_tool_result` | `ToolSearchToolResultBlock` (content/server_tool.rs:141) | ✅ |
| 12 | `container_upload` | `ContainerUploadBlock` (content/server_tool.rs:83) | ✅ |

Additional block types the API emits on GA paths that are **not** in the SDK response union (they arrive
via beta surfaces or fallback flows): `fallback` (`{type, from{model}, to{model}}`) and `connector_text`.

Official `TextBlock` carries `citations: Array<TextCitation> | null` (`messages.ts:1590`) — a 5-member
union of `CitationCharLocation`, `CitationPageLocation`, `CitationContentBlockLocation`,
`CitationsWebSearchResultLocation`, `CitationsSearchResultLocation`. clauders' `TextBlock` carries
the matching `citations: Option<Vec<TextCitation>>` (`content/text.rs:28`); `TextCitation`
(`content/citation.rs:25`) models the same five variants plus a payload-carrying `Unknown` floor for a
citation kind this release does not model, following the §5.1 forward-compatibility policy.

### 3.2 Request union — `ContentBlockParam`, 17 members

`types/content_block_param.py` (Python — note: a **plain** union, no discriminator) and `messages.ts:864`:

All 12 response members above, **plus** five input-only members:

| Official input-only member | clauders | Status |
|---|---|---|
| `image` | `ImageBlock` (`content/image.rs:59`), via `ContentBlockParam::Image` (`content/param.rs:37`) | ✅ — `base64`/`url` sources typed; Files API `file` source (beta) intentionally omitted |
| `document` | `DocumentBlock` (`content/document.rs:81`), via `ContentBlockParam::Document` (`content/param.rs:39`) | ✅ — `base64`/`text`/`url` sources typed; `content` (embedded-content) source retained as raw `serde_json::Value` |
| `search_result` | ❌ | ❌ |
| `tool_result` | `ToolResultBlock` (tools.rs:141) | ✅ |
| `mid_conversation_system` | ❌ | ❌ |

`image` source types (verified against `/docs/en/build-with-claude/vision`):

| `source.type` | Fields |
|---|---|
| `base64` | `media_type` ∈ `image/jpeg` \| `image/png` \| `image/gif` \| `image/webp`; `data` |
| `url` | `url` |
| `file` | `file_id` (Files API, beta header `files-api-2025-04-14`) |

High-resolution tier (Fable 5, Mythos 5, Opus 4.8, Opus 4.7, Sonnet 5): 2576 px long edge, 4784 visual
tokens; standard tier 1568/1568. Automatic, no beta header.

### 3.3 ✅ The response/request union split

clauders now uses **two** unions, matching the official SDKs' two-direction shape (§3.1, §3.2), joined
by a fallible carry-forward conversion — not one enum shared by both directions.

`ContentBlock` (`messages/content/block.rs:27-68`) is the response union: `Text`, `Thinking`,
`ToolUse`, the nine response-only variants from §3.1 (`content/server_tool.rs`),
and a payload-retaining `Unknown` fallback (§5.1). It has no `ToolResult` variant — the API never
returns that block kind, so a `tool_result` on the response path decodes into `Unknown`.

`ContentBlockParam` (`messages/content/param.rs:27-36`) is the request union: `Text`, `Thinking`,
`ToolUse`, `ToolResult`. It is closed — `#[non_exhaustive]` reserves room for downstream crates only,
there is no `Unknown` arm — because a caller only ever constructs block kinds this crate names.
`MessageContent::Blocks` (request.rs:130-135) and `ToolResultContent::Blocks` (tools.rs:185-190) both
carry `Vec<ContentBlockParam>` now, so sending a response-only block is a compile error rather than the
runtime "unserializable request block" failure a single shared enum allows.

Both unions share their leaf structs — `TextBlock` and `ThinkingBlock` (`messages/content/text.rs`) —
defined once and reused by each direction rather than duplicated per direction.

The multi-turn carry-forward path — echoing a response's content blocks back into the next request —
is `TryFrom<ContentBlock> for ContentBlockParam` (`messages/content/param.rs:80-119`), with a `Vec`
convenience, `ContentBlockParam::try_from_response` (`messages/content/param.rs:143-147`). `Text`,
`Thinking`, and `ToolUse` convert; the nine response-only blocks and `Unknown`
each fail with `UnsendableBlock`, which names the block's wire `type` — one `Err` arm per response-only
block, built via the private `UnsendableBlock::of` constructor (`messages/content/param.rs:72-77`). The
conversion is all-or-nothing: `try_from_response` fails the whole batch on the first unsendable block
instead of silently dropping it.

`ContentBlockParam`'s membership is narrower than official's 17 — that is a gap, §12; the *split* into
two unions is the divergence, [`../divergences.md`](../divergences.md).

### 3.4 ✅ `Role::System`

`Role` (request.rs:54-63) now has three variants: `User`, `Assistant`, `System`. Official
`MessageParam.role` is `'user' | 'assistant' | 'system'` (`messages.ts:1206`) — mid-conversation system
messages are GA on Claude Opus 4.8 with no beta header, and are the cache-preserving way to inject
operator instructions mid-session. The builder gained `.add_system_text(impl Into<String>)`
(request.rs:402), which appends an `InputMessage { role: Role::System, .. }` the same way
`.add_user_text()` / `.add_assistant_text()` already do, so a `system`-role turn requires no manual
`InputMessage` construction. `Role::System` round-trips through serde like the other two variants
(`#[serde(rename_all = "lowercase")]`, request.rs:55), pinned by
`add_system_text_emits_system_role_on_the_wire` (request.rs:766).

---

## 4. Streaming

### 4.1 Event and delta taxonomy — ✅

`RawMessageStreamEvent`, 6 members, identical in both SDKs
(`types/raw_message_stream_event.py`, `messages.ts:1436`):

| Official | clauders | Status |
|---|---|---|
| `message_start` | `StreamEvent::MessageStart` (streaming.rs:87) | ✅ |
| `content_block_start` | (streaming.rs:92) | ✅ |
| `content_block_delta` | (streaming.rs:99) | ✅ |
| `content_block_stop` | (streaming.rs:106) | ✅ |
| `message_delta` | (streaming.rs:112) | ✅ |
| `message_stop` | (streaming.rs:119) | ✅ |

`ping` and `error` are **not** union members officially — both SDKs handle them in the transport layer
(`_streaming.py:151` / `streaming.ts:51-142`: `ping` → skip, `error` → throw an `APIError`). clauders
models them as `StreamEvent::Ping` / `StreamEvent::Error` (streaming.rs:121, 123-126) and maps `Error` to
`Error::Api` in `collect()` (streaming.rs:260-271). Equivalent behavior, different placement. ✅

`RawContentBlockDelta`, 5 members (`types/raw_content_block_delta.py`, `messages.ts:1338`):

| Official | clauders | Status |
|---|---|---|
| `text_delta` | `ContentDelta::TextDelta` (streaming.rs:151) | ✅ |
| `input_json_delta` | `InputJsonDelta` (streaming.rs:167) | ✅ modelled |
| `thinking_delta` | `ThinkingDelta` (streaming.rs:156) | ✅ modelled |
| `signature_delta` | `SignatureDelta` (streaming.rs:161) | ✅ modelled |
| **`citations_delta`** | `ContentDelta::CitationsDelta` (streaming.rs:174) | ✅ merged onto `TextBlock.citations` (accumulator.rs:259) |

> There is no `redacted_thinking_delta` in either SDK, despite the name appearing in some
> third-party summaries. The five above are the whole union.

### 4.2 ✅ Accumulation

Assembly lives in `MessageAccumulator` (`messages/accumulator.rs`); `collect()` is a thin wrapper
over it (streaming.rs:249-277). All five delta kinds clauders models are handled, and the observable
end state matches the official SDKs — graded against SDK source, not prose, per the rule table at the
end of this section.

Getting this wrong is quiet rather than loud, which is why it is graded carefully. A `signature_delta`
handled as a concat instead of a replace produces an extended-thinking block that looks right and is
rejected on the *next* request. Dropped `input_json_delta` fragments produce a tool call with empty
arguments and no error at all.

**Reference behavior — both official SDKs buffer the partial JSON and parse it tolerantly.**

Python (`lib/streaming/_messages.py:433 accumulate_event`), re-parsing the whole buffer per delta with
`jiter` in partial mode:

```python
json_buf = cast(bytes, getattr(content, JSON_BUF_PROPERTY, b""))
json_buf += bytes(event.delta.partial_json, "utf-8")
if json_buf:
    content.input = from_json(json_buf, partial_mode=True)
setattr(content, JSON_BUF_PROPERTY, json_buf)
```

TypeScript (`lib/MessageStream.ts:626-631`), buffering and installing a memoized lazy getter that calls
a vendored tolerant parser on first read, then materializing it at `content_block_stop`
(`MessageStream.ts:657-668`):

```ts
if (snapshotContent && tracksToolInput(snapshotContent)) {
  const jsonBuf = ((snapshotContent as any)[JSON_BUF_PROPERTY] || '') + event.delta.partial_json;
  snapshot.content[event.index] = withLazyInput(snapshotContent, jsonBuf);
}
```

Both gate on `tool_use` **or** `server_tool_use` (`TRACKS_TOOL_INPUT` / `tracksToolInput`), so
server-tool inputs accumulate too.

"Both SDKs buffer and parse tolerantly" is true but hides that the two parse at *different times*, and
that neither parses where clauders does:

| SDK | When the buffer is parsed |
|---|---|
| Python | **Eagerly, on every delta** — `from_json(json_buf, partial_mode=True)` re-parses the whole buffer per fragment (`_messages.py:479-480`). `content_block_stop` does **not** re-parse (`:499-502`), so the final `input` is simply the last partial parse. |
| TypeScript | **Lazily and memoized** — `withLazyInput` installs a getter that parses on first read of `.input` (`internal/message-stream-utils.ts:21-27`). In practice that fires at `content_block_stop`, because line `MessageStream.ts:661` reads the property to freeze it. |
| Go | **Never** — `cb.Input` is left as raw `json.RawMessage` for the caller to unmarshal (`messageutil.go:67-74`). |
| clauders | **Once, strictly, at `content_block_stop`** (`accumulator.rs:257-278`). |

No SDK parses "once at `content_block_stop`". For a block that receives its stop event the end state
is identical across all four, which is what parity is graded on; the divergence is confined to
truncated streams (see the rule table below).

Per-delta semantics, identical across both SDKs — the contract clauders must meet:

| Delta | Operation on the snapshot block |
|---|---|
| `text_delta` | **concat** onto `text` |
| `thinking_delta` | **concat** onto `thinking` |
| `signature_delta` | **replace** `signature` (not concat) |
| `citations_delta` | **append** the citation object to `citations`, coercing `null` → `[]` |
| `input_json_delta` | **buffer** raw partial JSON, tolerant-parse to `input` |
| unknown | **no-op** (`assert_never` under `TYPE_CHECKING` / `checkNever` — compile-time only) |

Error handling divergence worth noting: non-beta TS `MessageStream` lets a malformed buffer throw out
of the accumulator, while `BetaMessageStream.ts:684-702` catches it, substitutes `input = {}`, and
emits a `#toolInputParseError`. Python does not have the recovery path either. So "throw on malformed
tool JSON" is acceptable parity for the non-beta surface.

**Rule by rule** (clauders column read from `accumulator.rs`; official column from pinned source):

| Rule | Python / TypeScript | clauders | |
|---|---|---|---|
| `text_delta` | concat | concat (accumulator.rs:216-220) | ✅ |
| `thinking_delta` | concat | concat (accumulator.rs:221-225) | ✅ |
| `signature_delta` | **replace** (`_messages.py:494`, `MessageStream.ts:646`); Go concats | replace (accumulator.rs:226-233) | ✅ follows the two pinned SDKs |
| `input_json_delta` | buffer + tolerant parse | buffer + strict parse at stop (accumulator.rs:193-207, 257-278) | ✅ same end state |
| delta index out of range | Python **raises `IndexError`** (`_messages.py:465`); TS silent no-op (`.at()` → `undefined`) | silent no-op (accumulator.rs:212-214) | 🔶 follows TS, diverges from Python |
| delta kind ≠ block kind | silent drop | silent drop (accumulator.rs:215-235) | ✅ |
| empty tool buffer | Python leaves the start-event `input`; TS substitutes `{}` | leaves the start-event `input` (accumulator.rs:262-264) | ✅ follows Python; identical in practice, the API opens the block with `"input": {}` |
| malformed tool JSON | both raise (non-beta) | `Error::Serde` (accumulator.rs:265-269) | ✅ |
| `citations_delta` | accumulate onto `TextBlock.citations` | append (accumulator.rs:259) | ✅ |
| gating of `input_json_delta` | `tool_use` **or** `server_tool_use` | `ContentBlock::ToolUse` **or** `ContentBlock::ServerToolUse` (accumulator.rs:222-225, 305-309) | ✅ |
| truncated buffer, no `content_block_stop` | Python's last eager parse salvages **complete** key/value pairs (`{"a": 1,` → `{"a": 1}`); TS same on `.input` read, though `finalMessage()` throws first | never parsed, `input` stays at the start-event value | 🔶 reachable only on an already-broken stream |

### 4.3 🔶 Index handling on `content_block_start`

**This one does not match the pinned SDKs, on purpose.**

clauders requires `index == content.len()` on `content_block_start` and returns `Error::Stream`
otherwise (`accumulator.rs:197-202`), pushing nothing. The three official SDKs do not agree on this
rule, so there was no single behaviour to port:

| SDK | `content_block_start` policy |
|---|---|
| Python | **Blind append, `index` never read** — ships a literal `# TODO: check index` (`_messages.py:456-463`) |
| TypeScript | **Blind append** — `snapshot.content.push({ ...event.content_block })` (`MessageStream.ts:601-603`) |
| Go | **Hard error** unless `event.Index == len(acc.Content)` (`messageutil.go:47-58`) |
| clauders | **Hard error**, following Go |

So clauders follows the one SDK out of three that checks, and diverges from both SDKs this document
pins. The reasoning: Python and TS are permissive-and-silently-corrupting — a gapped or out-of-order
start misaligns the content list, and every subsequent index-addressed delta then lands on the wrong
block with no signal. Go's in-source comment is also the strongest available evidence about the actual
wire contract: *"Content blocks start in index order with no gaps: a start event always addresses the
slot right after the previous block, even when deltas and stops for still-open blocks interleave after
it."* Against a conforming server the three behaviors are indistinguishable; they differ only on a
malformed stream, where clauders reports and Python/TS corrupt.

Graded 🔶 rather than ✅ because a caller porting from Python or TypeScript **will** see different
behavior on a malformed stream: an `Error::Stream` where the official SDKs return a misaligned message.
Recorded in [`../divergences.md`](../divergences.md).

Delta and stop events are a separate question, and there clauders follows TypeScript: an out-of-range
index is a silent no-op (`accumulator.rs:212-214`), where Python raises `IndexError` and Go returns an
error. Also 🔶, by the same standard.

### 4.4 ✅ `message_delta` usage merge

`UsageDelta` (streaming.rs:207-226) carries `input_tokens`, `cache_creation_input_tokens`,
`cache_read_input_tokens`, `output_tokens`, `output_tokens_details`, and `server_tool_use`.
`MessageAccumulator` (`accumulator.rs:137-172`) overwrites the snapshot's `input_tokens`,
`cache_creation_input_tokens`, `cache_read_input_tokens`, and `server_tool_use` when the delta
reports them, writes `output_tokens` unconditionally, and folds `stop_details` the same way — matching
the pinned Python SDK's fold policy.

**What is deliberately not folded.** Python's `accumulate_event` (`_messages.py:503-518`) overwrites
`input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens` and
`server_tool_use` when non-null, and separately folds `delta.stop_details` — but it does **not** assign
`output_tokens_details`; TypeScript's `accumulateMessage` (`MessageStream.ts:575-600`) mirrors the same
field list. clauders decodes `output_tokens_details` on the wire type for completeness and does not
fold it, matching what the SDK source actually does.

`container`, `usage.service_tier` and `usage.inference_geo` are likewise decoded but not folded:
neither pinned SDK copies them into its snapshot either, so `MessageMetaDelta.container`
(streaming.rs:196-199) and `Usage.{service_tier,inference_geo}` stay decoded-but-unfolded by design,
not by omission.

One divergence, small but real: clauders writes
`stop_reason` and `stop_sequence` **only when the delta carries them** (`accumulator.rs:139-144`),
whereas all three SDKs assign unconditionally — including overwriting a resolved value with `null`
(`_messages.py:504-505`, `MessageStream.ts:576-577`). Kept deliberately: it makes a stray later
`message_delta` unable to clobber a resolved `stop_reason`. The difference is observable only if the
API sends a `null` after a non-null value — which we **assume** it does not, since the terminal delta
is the one that carries these fields. That assumption is not evidenced by either SDK's source; neither
guards, because neither needed to.

### 4.5 🔶 Stream-completeness and duplicate `message_start`

Two edge behaviours where the official SDKs disagree with each other, recorded so they are not
re-argued. Both are pinned by tests in `accumulator.rs` and both appear in
[`../divergences.md`](../divergences.md).

**Truncated stream — no `message_stop`, no error.** clauders returns `Ok(partial_message)`; the caller
sees a `Message` whose `stop_reason` is `None`. This matches **Python and Go**, and diverges from
TypeScript:

| SDK | Behavior |
|---|---|
| Python | **Returns the partial snapshot, no raise.** There is no completeness check on the path at all: `get_final_message()` → `until_done()` → `consume_sync_iterator` (`_utils/_streams.py:5-7`, literally `for _ in iterator: ...`), and `__final_message_snapshot` is rewritten on every event (`_messages.py:130`), so the `assert` at `:94` passes. A truncated response is indistinguishable from a complete one except by inspecting `stop_reason is None`. |
| Go | Same — `Accumulate` has no end-of-stream hook (`messageutil.go:11-19`); the caller keeps whatever accumulated. |
| TypeScript | **Throws** `AnthropicError('stream ended without producing a Message with role=assistant')` (`MessageStream.ts:329-333`), because `receivedMessages` is only populated by `message_stop`. |
| clauders | `Ok(partial)` — follows Python/Go. `Error::Stream` is returned only when the stream ends before `message_start` (`accumulator.rs:286-289`). |

**Duplicate `message_start`.** All three official SDKs behave differently, so no porting answer exists:

| SDK | Behavior |
|---|---|
| Python | **Ignores the second entirely.** `message_start` is handled only inside `if current_snapshot is None:` (`_messages.py:450-452`), which returns early; the second `if/elif` chain has no `message_start` branch. The second message's `id`/`model`/`usage` are discarded and its content blocks are **appended onto the first message's content list**, interleaving two messages into one snapshot with no error and no id-mismatch check. |
| TypeScript | **Throws** `Unexpected event order, got message_start before receiving "message_stop"` (`MessageStream.ts:562-564`). |
| Go | **Replaces** the whole message, discarding everything accumulated (`messageutil.go:26-27`). |
| clauders | **Replaces** the snapshot and resets the JSON buffers (`accumulator.rs:113-117`) — follows Go. Python's interleaving is the one behavior worth avoiding outright. |

---

## 5. Forward compatibility

**This is a behavioral contract, not a nice-to-have.** The Anthropic versioning policy states new
content-block types and new SSE event types may be added within `anthropic-version: 2023-06-01`, and
the streaming guide says verbatim: *"new event types may be added, and your code should handle unknown
event types gracefully."*

All ten server-decoded enums carry a payload-retaining unknown arm plus `#[non_exhaustive]`.

| Scenario | Python | TypeScript | clauders |
|---|---|---|---|
| Unknown `type` on a content block | Coerced into the **first union variant** (`TextBlock`), unknown keys retained on `__pydantic_extra__`; `.type` preserved verbatim (`_models.py:578 construct_type`, fallback loop `:638-642`) | **No validation at all** — `defaultParseResponse` (`internal/parse.ts:18`) returns raw parsed JSON; the object arrives intact and simply fails to narrow (`messages.ts:847`) | ✅ `ContentBlock::Unknown(Value)` + `#[serde(untagged, skip_serializing)]` (content/block.rs:69) — payload retained, echo-back refused |
| Unknown `type` on a content-block delta | Coerced to `TextDelta`; accumulator no-ops on it | Passed through untouched; accumulator `default: checkNever(...)` no-ops (`MessageStream.ts:651`) | ✅ `ContentDelta::Unknown(Value)` (streaming.rs:185), accumulator no-ops (accumulator.rs:267) |
| Unknown SSE `event:` name | **Silently skipped** — allowlist chain in `_streaming.py:86`, no branch matches, nothing yielded | **Silently skipped** — same allowlist shape in `core/streaming.ts:51-142` | ✅ `StreamEvent::Unknown(Value)` (streaming.rs:135), yielded not dropped — clauders dispatches on `data.type`, not the `event:` name |
| Unknown field on a known object | Retained (`model_config = ConfigDict(extra="allow")`, `_models.py:107`) | Retained (no stripping) | Ignored — serde default ✅ |

Python's behavior is asserted by the SDK's own test, `tests/test_models.py:691
test_discriminated_unions_unknown_variant`, whose inline comment reads `# just chooses the first
variant`:

```python
assert isinstance(m, A)
assert m.type == "c"
assert m.data == None
assert m.new_thing == "bar"
```

Note what that means: Python retains the payload but mistypes the container. clauders retains it under
a variant that is honest about not knowing the type, pinned by
`parse_unknown_type_yields_unknown_event_with_payload` (streaming.rs:498).

The stakes are not hypothetical. `server_tool_use`, `redacted_thinking`, `web_search_tool_result`,
`container_upload`, `fallback` and `connector_text` are all emitted on GA paths today, and a client
without unknown arms fails the whole response on any of them.

### 5.1 It applies to every server-decoded enum, not just content blocks

Every enum clauders decodes from a server response carries an unknown arm — except `ErrorType`, which
already
carried a presence-only `#[serde(other)]` unit arm — and each closed one hard-failed the enclosing
struct on an unrecognized value. The content-block and stream-event unions were the widest exposure,
but not the only one:

| Enum | Site | Exposure |
|---|---|---|
| **`StopReason`** | response.rs:64-87 | ✅ `pause_turn` is a typed `StopReason::PauseTurn` variant (§8.1); an untagged `Unknown(String)` arm retains the raw value for anything else, so an unrecognised stop reason never fails the enclosing `Message`. |
| `ErrorType` | error.rs:70 | Already tolerant — a presence-only `#[serde(other)]` unit arm meant it never hard-failed. Its gap was payload retention, not decode failure. |
| `BatchStatus` | batches/types.rs:140-152 | Plausible. Batch lifecycle states have grown before. |
| `MessageKind` | response.rs:53 | Latent — single-valued, stable. |
| `BatchKind` / `DeletedBatchKind` | batches/types.rs:127 / :230-237 | Latent — single-valued. |
| `ModelInfoKind` | models/types.rs:31 | Latent — single-valued. |

`StopReason` is the one that would otherwise fail on GA paths, since `pause_turn` is returned whenever
a server-tool loop hits its iteration limit. The rest are latent — the distinction is timing, not kind.

`SystemSegmentKind` (system.rs:159) is deliberately **absent** from this table. It derives `Serialize`
only — a request-side type the SDK sends and never decodes — so an unknown arm on it could be neither
produced by decoding nor serialized. Ten enums are in scope, not eleven.

**Policy:** every enum decoded from a server response carries an unknown arm, and that arm **retains the
payload**. Applying it uniformly is cheaper than re-deciding per enum, and payload retention is what all
five official SDKs do — Python via `__pydantic_extra__`, TypeScript by passthrough, Go via `RawJSON()`,
Java via `_json()`, C# via `.Json`. A presence-only arm would make clauders the only family member that
discards data the server sent.

Mechanically this is a **payload-carrying** variant with variant-level `#[serde(untagged)]` as a
fallback — `serde_json::Value` on the internally-tagged enums (`#[serde(tag = "type")]`), `String` on
the bare-string ones. The cost is that serde's derive buffers each value through its internal
`private::de::Content` type rather than deserializing straight into the typed form; that is the same
cost every official SDK pays to retain unknown payloads. Two consequences follow. A `String` payload is
not `Copy`, so the bare-string enums lose that derive. And a **known** discriminant whose payload fails
to satisfy its variant is absorbed by the fallback rather than raising an error — deliberate, and pinned
by test.

`ContentBlock` is the one asymmetry: it alone also derives `Serialize`, so its unknown arm carries
`#[serde(skip_serializing)]` — an `Unknown` block cannot be re-serialized. Since the content-block
It is not used on the request path — that path carries `ContentBlockParam`, which has no
`Unknown` arm — and echoing a response-only block back is prevented at compile time by the
`TryFrom<ContentBlock> for ContentBlockParam` conversion, which returns `UnsendableBlock` rather than
surfacing a runtime `Error::Serde`. The other nine are `Deserialize`-only.

Unrecognized SSE **event names** get the same treatment — surfaced as `StreamEvent::Unknown`, never an
error. The tolerance stops there: a payload that is not an object, or an object whose `type` matches a
**modelled** event but whose body does not satisfy it, still raises `Error::Serde`. Absorbing those
would silently swallow a malformed `error` event, and stream termination keys off `StreamEvent::Error`.
This is where `StreamEvent` deliberately diverges from `ContentDelta`, whose malformed-known fall-through
is harmless.

---

## 6. Tools

| Aspect | Python | TS | clauders | Status |
|---|---|---|---|---|
| Custom tool (`name`/`description`/`input_schema`) | ✅ | ✅ | `Tool` (tools.rs:45) | ✅ |
| `strict: true` | ✅ | ✅ | `Tool.strict` (tools.rs:62) | ✅ |
| `tool_choice`: auto / any / tool / none | ✅ | ✅ | `ToolChoice`, all four (tools.rs:74-88) | ✅ |
| `tool_result` content (text / blocks / `is_error`) | ✅ | ✅ | `ToolResultBlock` / `ToolResultContent` (tools.rs:141-190) | ✅ |
| `cache_control` on tool definitions | ✅ | ✅ | `Tool.cache_control` (tools.rs:57) | ✅ |
| `eager_input_streaming` (fine-grained tool streaming, GA) | ✅ | ✅ | `Tool.eager_input_streaming` (tools.rs:70) | ✅ |
| `ToolUseBlock.caller` (`direct` / `code_execution_20250825` / `code_execution_20260120`) | ✅ | ✅ | `ToolUseBlock.caller` (tools.rs:127), reuses `ToolCaller` (content/server_tool.rs:16) | ✅ |
| **Server-side tools** — web_search, web_fetch, code_execution, tool_search | ✅ | ✅ | ❌ | ❌ |
| **Anthropic-defined tools** — bash, text_editor, memory | ✅ | ✅ | ❌ | ❌ |

The official `ToolUnion` is 19 members (`messages.ts:2277`), versioned by date suffix — e.g.
`web_search_20250305` / `_20260209` / `_20260318`, `code_execution_20250522` / `_20250825` /
`_20260120` / `_20260521`. clauders models only the custom-tool variant.

`ServerToolUseBlock.name` is a closed 7-value union officially: `web_search`, `web_fetch`,
`code_execution`, `bash_code_execution`, `text_editor_code_execution`, `tool_search_tool_regex`,
`tool_search_tool_bm25`.

**Verdict:** ✅ full parity on **custom** tool use. ❌ on the entire server-side / Anthropic-defined tier.

---

## 7. Prompt caching

| Aspect | Python | TS | clauders | Status |
|---|---|---|---|---|
| `cache_control: ephemeral` on content blocks | ✅ | ✅ | `CacheControl::ephemeral` (types/caching.rs:73-85) | ✅ |
| 5-minute / 1-hour TTL tiers | ✅ `'5m' \| '1h'` | ✅ same | `CacheTtl::{FiveMinutes,OneHour}` (types/caching.rs:33-40) | ✅ |
| Carriers: system segment / text / tool / tool_use / tool_result | ✅ | ✅ | system.rs:149, content/text.rs:25, tools.rs:57/118/152 | ✅ |
| Cache-aware usage counters | ✅ | ✅ | `Usage.cache_creation/read` (response.rs:122-125) | ✅ |
| `cache_creation` per-tier breakdown | ✅ | ✅ | `CacheCreation` (response.rs:94-100) | ✅ |
| Top-level `cache_control` (auto-place on last cacheable block) | ✅ | ✅ | `MessageRequest.cache_control` (request.rs:249-252), `.cache_control()` builder setter (request.rs:586) | ✅ |

Explicit per-block caching is at genuine parity, including both TTL tiers and the tier-split accounting.
The top-level convenience form works too: clauders sends the ephemeral `cache_control` breakpoint
at the top level and the server auto-places it on the last cacheable block, exactly as the official
SDKs describe it — the crate does not compute placement itself, it forwards the breakpoint. The
documented cache **pre-warm** call (`max_tokens: 0`) is representable (§2.3).

---

## 8. Response & usage

Official `Message` — 10 fields, identical in both SDKs (`types/message.py`, `messages.ts:1020`):

| Field | clauders | Status |
|---|---|---|
| `id` / `type` / `role` / `model` / `content` / `stop_sequence` | `Message` (response.rs:27-51) | ✅ |
| `stop_reason` | `StopReason` (response.rs:73-91) | ✅ — all 6 official values **typed**, including `PauseTurn`; `Unknown(String)` retained for values a future release adds, see §8.1 |
| **`stop_details`** | `StopDetails` (response.rs:44-45, 99-109) | ✅ |
| **`container`** | `Container` (response.rs:47-48, 145-151) | ✅ |
| `usage` | `Usage` (response.rs:217-243) | ✅ — see §8.3 |

### 8.1 `stop_reason` — non-beta is exactly 6 values

`types/stop_reason.py:StopReason` and `messages.ts:1588`, both closed with no catch-all:

`end_turn` · `max_tokens` · `stop_sequence` · `tool_use` · **`pause_turn`** · `refusal`

clauders now types all six, including `pause_turn` — the value the API returns whenever a server-tool
loop hits its 10-iteration limit, i.e. on every long server-tool turn.

**All six are typed, and `Unknown(String)` is retained for future values.**
`PauseTurn` is a first-class variant (response.rs:86), pinned by a test at response.rs:384, so a caller
matches it directly and can act on a paused server-tool turn through the type system. `Unknown(String)`
covers whatever value a future release adds. See §5.1.

> **`model_context_window_exceeded` is not in this union.** It is real, but both SDKs type it only in
> their `beta` namespace. The API returns it without a beta header on Sonnet 4.5 and newer; older
> models need `model-context-window-exceeded-2025-08-26`. Out of scope for non-beta parity.

### 8.2 `stop_details`

Single shape, not a union — `types/refusal_stop_details.py:RefusalStopDetails` / `messages.ts:1459`:

```json
{
  "type": "refusal",
  "category": "cyber" | "bio" | "frontier_llm" | "reasoning_extraction" | null,
  "explanation": "string | null"
}
```

`null` for every stop reason other than `refusal`. Both SDKs and the docs are explicit that callers
must branch on `stop_reason`, never on `stop_details` — `explanation` is documented as not stable.
A `recommended_model` field appears when a server-side fallback attempt was skipped.

### 8.3 `usage`

Official (`types/usage.py`, `messages.ts:2345`):

| Field | clauders | Status |
|---|---|---|
| `input_tokens` / `output_tokens` | ✅ (response.rs:219, :221) | ✅ |
| `cache_creation_input_tokens` / `cache_read_input_tokens` | ✅ (response.rs:224, :227) | ✅ |
| `cache_creation.{ephemeral_5m,ephemeral_1h}_input_tokens` | ✅ (response.rs:158-163) | ✅ |
| **`output_tokens_details.thinking_tokens`** | `OutputTokensDetails` (response.rs:170-173, 233) | ✅ |
| **`server_tool_use.{web_search_requests,web_fetch_requests}`** | `ServerToolUse` (response.rs:177-182, 236) | ✅ |
| **`service_tier`** (`standard` \| `priority` \| `batch`) | `UsageServiceTier` (response.rs:191-201, 239) | ✅ |
| **`inference_geo`** | `Option<String>` (response.rs:242) | ✅ |
| `iterations[]` (beta, server-side fallback) | ❌ | — beta |

`Usage::total_input_tokens` (response.rs:246-268) is a clauders-only convenience with no official
counterpart. Harmless, keep.

### 8.4 Typed parse helper

Python ships `client.messages.parse()` + `ParsedMessage`/`ParsedTextBlock.parsed_output` and parses at
`content_block_stop` during streaming (`_messages.py:499-502`). TypeScript ships `zodOutputFormat` +
`parsed_output`. clauders exposes the raw `output_config.format.schema` only — the caller parses the
first text block by hand. 🟡

---

## 9. Models API

Official `ModelInfo` — `types/model_info.py`, `resources/models.ts:177`, confirmed against
`GET /v1/models`:

| Field | clauders (`models/types.rs:58-79`) | Status |
|---|---|---|
| `id` | ✅ | ✅ |
| `display_name` | ✅ | ✅ |
| `created_at` | ✅ (kept as `String`) | ✅ |
| `type` | ✅ `ModelInfoKind` | ✅ |
| **`max_input_tokens`** | ✅ `Option<u32>` (models/types.rs:70-72) | ✅ |
| **`max_tokens`** | ✅ `Option<u32>` (models/types.rs:73-75) | ✅ |
| **`capabilities`** | ✅ `Option<ModelCapabilities>` (models/types.rs:76-78) | ✅ |

`ModelCapabilities` — 9 fields (`types/model_capabilities.py`, `models.ts:130`), all decoded in
`models/capabilities.rs`:

```
batch, citations, code_execution, image_input, pdf_input, structured_outputs  → CapabilitySupport { supported: bool }
context_management → { clear_thinking_20251015?, clear_tool_uses_20250919?, compact_20260112?, supported }
effort             → { low, medium, high, max, xhigh?, supported }
thinking           → { supported, types: { adaptive, enabled } }
```

This is the API's live capability-discovery surface — the supported way to answer "does this model take
`xhigh` effort / adaptive thinking / PDF input" without hardcoding a table. Eight of the nine fields —
`batch`, `citations`, `code_execution`, `image_input`, `pdf_input`, `structured_outputs`, `effort`,
`thinking` — are modelled with named fields matching the official shape one-for-one
(`capabilities.rs:8-37`, `52-71`, `74-103`). The ninth, `context_management`, is present but shaped
differently — see §9.1.

`ModelList` pagination (`data`/`has_more`/`first_id`/`last_id`) matches. ✅

### 9.1 🔶 `context_management` is a map, not named fields

Both official SDKs hardcode each dated context-management strategy as its own named optional field:
`clear_thinking_20251015?`, `clear_tool_uses_20250919?`, `compact_20260112?`, plus a `supported: bool`.
clauders instead models `ContextManagementCapability` as `supported: bool` plus a
`#[serde(flatten)] strategies: BTreeMap<String, CapabilitySupport>` (capabilities.rs:39-49) — every
dated key the server sends, named or not yet named, lands in the map under its wire key rather than a
struct field, pinned by `context_management_dated_keys_land_in_the_map` (capabilities.rs:139-150).

Graded 🔶, not a plain ✅, for the same reason as §4.3: it is a deliberate design choice, not a defect,
but a caller porting field-access code from Python or TypeScript
(`caps.context_management.clear_thinking_20251015`) will not find a same-named field on
`ContextManagementCapability` and must index the map instead
(`caps.context_management.strategies.get("clear_thinking_20251015")`). The observable data is
equivalent — every dated key the server sends is retained, none dropped — but the access pattern
differs, and unlike the pinned SDKs a newly dated strategy needs no clauders code change to be
represented. Recorded in [`../divergences.md`](../divergences.md).

---

## 10. Message Batches — ✅

`batches::BatchesResource` implements all six operations — `create` (batches/resource.rs:88), `get`
(:106), `list` (:119), `results` (:137), `cancel` (:161), `delete` (:174) — against the official
`create` / `retrieve` / `list` / `results` / `cancel` / `delete` (`resources/messages/batches.d.ts:37,
53, 69, 85, 107, 124`). `Batch`, `BatchStatus`, `RequestCounts`, `BatchList`, `BatchResultRow`,
`BatchResult`, `DeletedMessageBatch` (batches/types.rs:45-219) match the official shapes.

**One gap: pagination.** Official `list` takes `BatchListParams` (`batches.d.ts:69`) and returns a
`MessageBatchesPage` the caller can walk. clauders' `list()` takes no arguments
(`batches/resource.rs:119`), so a caller cannot request the next page or set a page size. `BatchList`
decodes the cursor fields the API returns, but nothing consumes them. ❌

One behavioural note for the future beta surface: a refused request inside a batch returns
`result.type: "succeeded"` with `stop_reason: "refusal"`, and `stop_details` **may be `null` on batch
results** — so refusal detection in batches must key on `stop_reason`.

---

## 11. Type-level and ergonomic parity

| Aspect | Official | clauders | Status |
|---|---|---|---|
| Open model-id type | TS `Model = <15 known ids> \| (string & {})` (`messages.ts:1258`); Python `ModelParam` accepts `str` | `ModelId::custom` + 7 headline ctors (types/model_id.rs:70-110) | ✅ same escape hatch |
| Headline model constructors | 15 ids incl. `claude-fable-5`, `claude-opus-4-8`, `claude-sonnet-5` | `claude_opus_4_7`, `claude_sonnet_4_6`, `claude_sonnet_4_5`, `claude_haiku_4_5`, `claude_opus_4_8`, `claude_sonnet_5`, `claude_fable_5` (model_id.rs:70-110) | ✅ current headline set |
| `system` as string or block array | ✅ | `SystemPrompt::{Text,Segments}` (types/system.rs:57-64) | ✅ |
| Per-segment system `cache_control` | ✅ | `SystemSegment.cache_control` (types/system.rs:149) | ✅ |
| Deprecation signalling on sampling params | ✅ `@deprecated` + 400 semantics in TS | none | ❌ — see §2.2 |
| Unknown-field tolerance on known objects | ✅ | ✅ serde default | ✅ |
| Client-side model/thinking mismatch warning | TS `console.warn` on `enabled` thinking + Opus 4.6 (`messages.ts:79-87`) | ❌ | ❌ — optional |

`ModelId` gained `claude_opus_4_8()`, `claude_sonnet_5()`, and `claude_fable_5()` alongside the four
pre-existing constructors (model_id.rs:96-110), and the crate's doctests and examples were swept from
`claude_sonnet_4_5()` onto `claude_sonnet_5()` (e.g. `messages::MessageRequest`'s doctest, request.rs:195)
so a caller copying an example is not steered to an older model. `custom()` is the escape hatch for any
id without a dedicated constructor.

---

## 12. Gaps

What the official SDKs do that this crate does not. Ranked by how likely a caller is to hit them, not
by official-checklist order.

| # | Gap | Where it bites |
|---|---|---|
| 1 | **Server-side and Anthropic-defined tool definitions** — 18 of the 19 official `ToolUnion` members (`messages.d.ts:1535`). Only the custom `Tool` is modelled (`tools.rs:47`). | You cannot declare `web_search`, `web_fetch`, `code_execution`, `bash`, `text_editor`, `memory`, or `tool_search`. Their *results* decode fine (§3.1) — you just cannot ask for them. |
| 2 | **Request content blocks — 6 of 17** (`param.rs:27`). Missing `search_result`, `mid_conversation_system`, and the response-only kinds. | `search_result` blocks for grounded citation, and mid-conversation system turns. |
| 3 | **Files API** (`/v1/files`). | Uploading a file once and referencing it by id, instead of re-sending base64 on every request. Also the `file` image source. |
| 4 | **Typed parse helper.** Python ships `messages.parse()` with `ParsedMessage`; TypeScript ships `zodOutputFormat`. | With structured output you get schema-conforming JSON in the first text block and deserialize it yourself (§8.4). |
| 5 | **Batch list pagination.** `batches().list()` takes no arguments (`batches/resource.rs:119`) where official `list` accepts `BatchListParams` (`batches.d.ts:69`). | You cannot page through more batches than one response returns. |
| 6 | **Context management and the MCP connector.** | Both are beta request parameters; see the beta note in §0. |
| 7 | **`user_profile_id`**, sent as the `anthropic-user-profile-id` header. | Per-user attribution. |
| 8 | **Client-side model/thinking mismatch warning.** TypeScript emits a `console.warn` for `enabled` thinking on a model that rejects it (§11). | You get the server's 400 rather than a local warning. |
| 9 | **Legacy Text Completions** (`client.completions`). | Deprecated upstream; not pursued. |

Two things deliberately absent from this list. Deliberate differences from the official SDKs are not
gaps — they are in [`../divergences.md`](../divergences.md), and the body sections that establish them
are §3.3, §4.2, §4.3, §4.4, §4.5 and §9.1. And the `beta.*` namespace is out of scope for this
document; most of it is a separate product, see §0.

---

## 13. Where clauders goes beyond the official SDKs

Not parity claims. These exist because Rust's type system can rule out at compile time what the
scripting SDKs can only catch at runtime, or not at all.

**Unknown values are retained, on every server-decoded enum.** Ten enums carry a payload-carrying
unknown arm (§5.1). Python coerces an unrecognised discriminated-union variant into the *first*
member of the union — its own test asserts this, with the inline comment `# just chooses the first
variant` — so `.type` survives but the object is mistyped. TypeScript does not validate responses at
all. clauders keeps the raw JSON under a variant that is honest about not knowing what it is.

**Sending a response-only block is not representable.** The official SDKs share one loose union across
both directions, so echoing a `server_tool_use` or `container_upload` back into a request type-checks
and fails at serialization. clauders splits the unions and joins them with a fallible
`TryFrom<ContentBlock> for ContentBlockParam` that returns `UnsendableBlock` naming the offending wire
`type` (§3.3). The `Vec` form fails on the first bad block rather than dropping it silently.

**A request without `model` or `max_tokens` does not compile.** `MessageRequest::builder()` is
type-state; so is `Client::builder()`, whose `build()` does not exist until `api_key` is set
(`src/builder.rs:183`). There is no `MissingApiKey` error variant to handle because the state is
unreachable.

**Values are parsed once, at construction.** `ApiKey`, `BaseUrl`, `ModelId`, `Temperature`, `TopP`,
`TopK`, `BetaHeader` each validate in their constructor and are proof thereafter. The official SDKs
pass strings and floats and let the server reject them.

**The transport is a type parameter.** `Client<T = ReqwestTransport>` holds `Arc<ClientInner<T>>`
(`src/client.rs:42,46`), so a caller can substitute a tuned client or a test double with no dynamic
dispatch and no network in unit tests.

**A misaligned stream is reported, not silently corrupted.** See §4.3 — this one is a divergence as
well as an advantage, and is graded 🔶 for that reason.

---

## 14. Methodology & caveats

- **clauders side** — read from source under `crates/clauders/src/messages/` (`request.rs`,
  `response.rs`, `content/`, `tools.rs`, `streaming.rs`, `accumulator.rs`, `token_counting.rs`,
  `structured_outputs.rs`, `resource.rs`, `batches/`), plus `models/{types,capabilities,resource}.rs`,
  `types/`, `client.rs`, and `builder.rs`. Authoritative.
- **Official side** — read from the shipped `@anthropic-ai/sdk@0.115.0` tarball for the TypeScript
  column, cross-checked against the REST reference. Where an SDK and the prose documentation
  disagreed, **the SDK source wins** and the disagreement is noted inline (see §8.1 on
  `model_context_window_exceeded`).
- **The Python column is carried over and not re-verified in this pass.** The package was not
  available locally. Rows resting on Python-only evidence — chiefly the accumulator's per-delta timing
  in §4.2 and the unknown-variant coercion in §5 — should be re-checked against the pinned commit
  before being treated as current.
- **Parity is graded on behaviour**, not on whether a type exists.
- Marks judge capability and behaviour, not wire or name identity. clauders is idiomatic Rust —
  builders, exhaustive enums, newtypes — so equivalent features carry Rust-shaped names.
- **Beta surfaces are out of scope** for the ✅/❌ grading and are listed only for completeness:
  `mcp_servers`, `context_management`, `fallbacks`, `speed`, `diagnostics`, `task_budget`,
  `model_context_window_exceeded`, the Files API, and Agent Skills.
- The base SDKs iterate weekly. Re-verify against the pinned versions before treating any single ❌ as
  durable.

## Sources

- Architecture and pillar mapping — [`../architecture.md`](../architecture.md)
- Deliberate divergences from the official SDKs — [`../divergences.md`](../divergences.md)
- Agent SDK parity, the other implemented pillar — [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md)
- `@anthropic-ai/sdk@0.115.0` — `resources/messages/messages.d.ts`, `resources/messages/batches.d.ts`, `resources/models.d.ts`, `client.d.ts`, `resources/beta/`
- `anthropic-sdk-python` @ `3c8bdf14bc55377262f11d6c34b893834a02b3fc` — `types/`, `lib/streaming/_messages.py`, `_models.py`, `_streaming.py` (carried over, not re-verified)
- REST reference — `platform.claude.com/docs/en/api/messages`, `/api/models-list`, `/build-with-claude/{streaming,vision,refusals-and-fallback,handling-stop-reasons}`
