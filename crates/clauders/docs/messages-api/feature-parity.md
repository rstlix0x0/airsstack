# clauders Messages API — Feature Parity vs the Official Anthropic Base SDKs

Compares the `clauders` Messages API layer (`clauders::messages`, plus `clauders::models`,
`clauders::types`) against the **official Anthropic base SDKs** — the raw Messages API clients:

- **Python** — [`anthropic-sdk-python`](https://github.com/anthropics/anthropic-sdk-python)
- **TypeScript** — [`anthropic-sdk-typescript`](https://github.com/anthropics/anthropic-sdk-typescript)

This is **Pillar 1** of the [vision](../vision-and-strategy.md). The base SDK is a *different* official
product from the Claude Agent SDK covered in [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md):
the base SDK is a stateless `POST /v1/messages` client; the Agent SDK drives the `claude` CLI.
`clauders` targets both, in separate modules.

**As of:** 2026-07-23 (WS C — response diagnostics & discovery: Models API `capabilities` /
`max_input_tokens` / `max_tokens`, GA request params, `Role::System`, refreshed headline model
constructors).

**Method — read this before trusting a row.** The previous revision of this document scored parity by
comparing *type surfaces* against prose documentation. That method produced false ✅s: a row can have
every type modelled correctly and still be broken at runtime, because parity is a property of
**behavior**, not of struct shape. This revision therefore grades against the **official SDK source**,
pinned to specific commits, and treats "does the SDK's accumulator/decoder do the same thing" as the
parity question. Three rows previously marked ✅ are ❌ or ⚠️ under that test.

**Sources, pinned:**

| Side | Version |
|---|---|
| clauders | `crates/clauders/src/` @ the commit carrying this revision (2026-07-23). Decode-path work landed across `a155625`, `0cb629f`, `f0aab9d`; the response-diagnostics-and-discovery surface (WS C: `Role::System`, GA request params, `ModelInfo` token limits and `capabilities`, refreshed headline constructors) landed across `b743013`..`bee2083`; **`file:line` citations resolve against this commit's tree**, not against either prior pin. |
| Python SDK | `anthropic-sdk-python` @ `3c8bdf14bc55377262f11d6c34b893834a02b3fc` (release 0.117.0, 2026-07-16) |
| TypeScript SDK | `anthropic-sdk-typescript` @ `f84e8638fc74268d602d729747f7fd9fcbadbc71` (2026-07-17) |
| Go SDK (tiebreaker only) | `anthropic-sdk-go` `messageutil.go` @ `0ce94bd583a556abfc18ccde1e132be5fd9e32f4` (branch `main`, **not** a pinned release) — consulted only where Python and TypeScript disagree, or where both blind-append |
| REST reference | `platform.claude.com/docs/en/api/messages`, `.../build-with-claude/{streaming,vision,refusals-and-fallback,handling-stop-reasons}`, `.../api/models-list` — fetched 2026-07-20 |

Paths in the Python column are relative to `src/anthropic/`; TypeScript to `src/`.

---

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Full parity — equivalent capability, equivalent runtime behavior |
| ⚠️ | **Defect** — the capability is modelled but behaves incorrectly at runtime (silent data loss or hard failure) |
| 🔶 | **Delivered, diverging** — the defect is fixed and the behavior is deliberate, but it does not match the pinned SDKs. A caller porting from Python or TypeScript will observe a difference. Used where the official SDKs disagree with each other and clauders had to choose. |
| 🟡 | Partial — core exists, narrower than official |
| ❌ | Absent in clauders |
| — | Not applicable |

⚠️ rows are ranked above ❌ rows: a missing feature degrades gracefully, a defect does not.

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
| **Streaming accumulation** | ✅ parity on all six modelled delta kinds, including `input_json_delta` buffering for both `ToolUse` and `ServerToolUse` blocks (accumulator.rs:222-225, 305-309) — with 5 recorded divergences (§12 rows 3, 18-21) |
| **Forward compatibility (every server-decoded enum)** | ✅ parity — payload-carrying unknown arm on all ten; `pause_turn` no longer fails |
| **`thinking` / `output_config.effort`** | ✅ parity — both request params delivered |
| **Response diagnostics (`container`, `stop_details`, typed `pause_turn`, usage sub-objects)** | ✅ parity — all delivered (§8, §8.1, §8.3) |
| **`message_delta` usage merge (input-side counters, `stop_details`)** | ✅ parity — overwrite-cumulative, matching Python/TypeScript (§4.4) |
| Response content-block taxonomy (12 official response members) | ✅ 12 of 12 |
| Request content-block taxonomy (17 official param members) | 🟡 4 of 17 — no vision, no PDF |
| Models API (`capabilities`, `max_input_tokens`, `max_tokens`) | ✅ delivered — one divergence: `context_management`'s dated strategies are a flatten map, not named fields (§9.1) |
| GA request params (`service_tier`, `inference_geo`, `container`, top-level `cache_control`) | ✅ delivered — all four are builder setters, serialized when set (§2) |
| Server-side & Anthropic-defined tools | ❌ behind |
| Files API / citations / context management / MCP connector | ❌ behind |

**One-line summary:** clauders is at genuine parity on the *non-streaming, text-and-custom-tools core*
— create, count-tokens, batches, caching, structured output, system prompts — **and now on streaming
accumulation and forward compatibility**, the two runtime defects the prior revision found. The
`thinking` and `output_config.effort` request parameters, the GA request params
(`service_tier`/`inference_geo`/`container`/top-level `cache_control`), `Role::System`, and the Models
API capability-discovery surface are all delivered; the response content-block taxonomy is now 12 of
12 — what remains is request-side content-block breadth (4 of 17) and the server-side/Anthropic-defined
tool tier, not correctness.

---

## 1. Resources & endpoints

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| `POST /v1/messages` | ✅ | ✅ | `messages::MessagesResource::create` (resource.rs:82) | ✅ |
| Streaming create (SSE) | ✅ | ✅ | `MessagesResource::stream` (resource.rs:172) | ✅ transport / ✅ accumulation (`MessageAccumulator`) — index policy follows Go, see §4.3 |
| `POST /v1/messages/count_tokens` | ✅ | ✅ | `count_tokens` (resource.rs:287) | ✅ |
| Message Batches — create/get/list/results/cancel/delete | ✅ | ✅ | `batches::BatchesResource` (all six) | ✅ |
| `GET /v1/models`, `GET /v1/models/{id}` | ✅ | ✅ | `models::ModelsResource::{list,get}` | ✅ endpoints / ❌ payload — see §9 |
| Files API (`/v1/files`) | ✅ (beta) | ✅ (beta) | ❌ | ❌ |

`count_tokens` projects `model` / `messages` / `system` / `tools` / `tool_choice` / `thinking` /
`output_config` (`token_counting.rs`), matching the endpoint's accepted set apart from
`cache_control`, which needs the top-level caching type tracked as row 12. `thinking` in particular
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

### 2.1 ✅ The `thinking` / `effort` surface — delivered

`MessageRequest` carries a `thinking: Option<ThinkingConfig>` field (request.rs:181), set through the
builder's `.thinking(ThinkingConfig)` method (request.rs:469). `output_config.effort` is set through
`.effort(EffortLevel)` (request.rs:462) or through `.output_config(OutputConfig)` (request.rs:452),
sharing the `EffortLevel` type with the Agent SDK pillar. Adaptive thinking can now be configured or
disabled, and `effort` can be set, on every current-generation model. `count_tokens` forwards both
parameters as well (§12 row 5).

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

### 2.4 ✅ `service_tier` / `inference_geo` / `container` / top-level `cache_control` — delivered

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

The official SDKs use **two different unions** for the two directions. The prior revision of this doc
conflated them.

### 3.1 Response union — `ContentBlock`, 12 members

`types/content_block.py:ContentBlock` (Python, discriminated on `type`) and `messages.ts:847` (TS) —
identical membership:

| # | Official member | clauders | Status |
|---|---|---|---|
| 1 | `text` | `TextBlock` (content/text.rs:18) | ✅ — `citations: Option<Vec<TextCitation>>` delivered (content/text.rs:28, content/citation.rs:25) |
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
`CitationsWebSearchResultLocation`, `CitationsSearchResultLocation`. clauders' `TextBlock` now carries
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

### 3.3 ✅ Response/request union split — delivered

clauders now uses **two** unions, matching the official SDKs' two-direction shape (§3.1, §3.2), joined
by a fallible carry-forward conversion — not one enum shared by both directions.

`ContentBlock` (`messages/content/block.rs:27-68`) is the response union: `Text`, `Thinking`,
`ToolUse`, the nine response-only variants delivered by §3.1 row 3 and rows 5-12 (`content/server_tool.rs`),
and a payload-carrying `Unknown` fallback (§5.1). It no longer carries `ToolResult` — the API never
returns that block kind, so a `tool_result` on the response path now decodes into `Unknown` rather than
a typed variant.

`ContentBlockParam` (`messages/content/param.rs:27-36`) is the request union: `Text`, `Thinking`,
`ToolUse`, `ToolResult`. It is closed — `#[non_exhaustive]` reserves room for downstream crates only,
there is no `Unknown` arm — because a caller only ever constructs block kinds this crate names.
`MessageContent::Blocks` (request.rs:130-135) and `ToolResultContent::Blocks` (tools.rs:185-190) both
carry `Vec<ContentBlockParam>` now, so sending a response-only block is a compile error rather than the
runtime "unserializable request block" failure the single shared enum used to allow.

Both unions share their leaf structs — `TextBlock` and `ThinkingBlock` (`messages/content/text.rs`) —
defined once and reused by each direction rather than duplicated per direction.

The multi-turn carry-forward path — echoing a response's content blocks back into the next request —
is `TryFrom<ContentBlock> for ContentBlockParam` (`messages/content/param.rs:80-119`), with a `Vec`
convenience, `ContentBlockParam::try_from_response` (`messages/content/param.rs:143-147`). `Text`,
`Thinking`, and `ToolUse` convert; the nine response-only blocks (§3.1 row 3, rows 5-12) and `Unknown`
each fail with `UnsendableBlock`, which names the block's wire `type` — one `Err` arm per response-only
block, built via the private `UnsendableBlock::of` constructor (`messages/content/param.rs:72-77`). The
conversion is all-or-nothing: `try_from_response` fails the whole batch on the first unsendable block
instead of silently dropping it.

This closes §12 row 7. `ContentBlockParam`'s membership is still the pragmatic subset already in scope
before the split — no `image` or `document` — which is tracked separately as a deliberate divergence,
§12 row 24, and as capability row 9.

### 3.4 ✅ `Role::System` — delivered

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

> The prior revision listed `redacted_thinking_delta` as an official delta type that clauders was
> missing. **No such delta exists** in either SDK. That row was wrong and has been removed.

### 4.2 ✅ Accumulation — delivered

**Status: fixed.** Assembly moved out of `collect()` into `MessageAccumulator`
(`messages/accumulator.rs`); `collect()` is now a thin wrapper over it (streaming.rs:249-277).
All five delta kinds that clauders models are handled, and the observable end state matches the
official SDKs. Verified against pinned SDK source, not against prose — see the per-rule table at the
end of this section.

The defect this section previously recorded, kept for the record: `collect()` handled exactly one
delta kind, so `InputJsonDelta`, `ThinkingDelta`, and `SignatureDelta` fell through an `if let` and
were discarded. Streaming tool use returned empty arguments (`"input": {}` from
`content_block_start`, every fragment dropped) and streaming extended thinking returned empty text
with a lost `signature` — which must round-trip verbatim or the API rejects the next turn.

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

**Correction to the prior revision's framing.** "Both SDKs buffer and parse tolerantly" is true but
hides that the two parse at *different times*, and that neither parses where clauders does:

| SDK | When the buffer is parsed |
|---|---|
| Python | **Eagerly, on every delta** — `from_json(json_buf, partial_mode=True)` re-parses the whole buffer per fragment (`_messages.py:479-480`). `content_block_stop` does **not** re-parse (`:499-502`), so the final `input` is simply the last partial parse. |
| TypeScript | **Lazily and memoized** — `withLazyInput` installs a getter that parses on first read of `.input` (`internal/message-stream-utils.ts:21-27`). In practice that fires at `content_block_stop`, because line `MessageStream.ts:661` reads the property to freeze it. |
| Go | **Never** — `cb.Input` is left as raw `json.RawMessage` for the caller to unmarshal (`messageutil.go:67-74`). |
| clauders | **Once, strictly, at `content_block_stop`** (`accumulator.rs:257-278`). |

No SDK parses "once at `content_block_stop`". For a block that receives its stop event the end state
is identical across all four, which is what parity is graded on; the divergence is confined to
truncated streams (see the delivered-state table below).

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

**Delivered state, rule by rule** (clauders column read from `accumulator.rs`; official column from
pinned source):

| Rule | Python / TypeScript | clauders | |
|---|---|---|---|
| `text_delta` | concat | concat (accumulator.rs:216-220) | ✅ |
| `thinking_delta` | concat | concat (accumulator.rs:221-225) | ✅ |
| `signature_delta` | **replace** (`_messages.py:494`, `MessageStream.ts:646`); Go concats | replace (accumulator.rs:226-233) | ✅ follows the two pinned SDKs |
| `input_json_delta` | buffer + tolerant parse | buffer + strict parse at stop (accumulator.rs:193-207, 257-278) | ✅ same end state |
| delta index out of range | Python **raises `IndexError`** (`_messages.py:465`); TS silent no-op (`.at()` → `undefined`) | silent no-op (accumulator.rs:212-214) | 🔶 follows TS, diverges from Python — §12 row 18 |
| delta kind ≠ block kind | silent drop | silent drop (accumulator.rs:215-235) | ✅ |
| empty tool buffer | Python leaves the start-event `input`; TS substitutes `{}` | leaves the start-event `input` (accumulator.rs:262-264) | ✅ follows Python; identical in practice, the API opens the block with `"input": {}` |
| malformed tool JSON | both raise (non-beta) | `Error::Serde` (accumulator.rs:265-269) | ✅ |
| `citations_delta` | accumulate onto `TextBlock.citations` | append (accumulator.rs:259) | ✅ §12 row 14 |
| gating of `input_json_delta` | `tool_use` **or** `server_tool_use` | `ContentBlock::ToolUse` **or** `ContentBlock::ServerToolUse` (accumulator.rs:222-225, 305-309) | ✅ |
| truncated buffer, no `content_block_stop` | Python's last eager parse salvages **complete** key/value pairs (`{"a": 1,` → `{"a": 1}`); TS same on `.input` read, though `finalMessage()` throws first | never parsed, `input` stays at the start-event value | 🔶 §12 row 19 — reachable only on an already-broken stream |

### 4.3 ⚠️→🔶 Index handling — fixed, by deliberate divergence

**Read this row carefully: it is closed, but NOT by matching the pinned SDKs.**

The original defect is gone. clauders used to pad with placeholder text blocks when `index` exceeded
the current length (`streaming.rs:216-222` at the time):

```rust
while m.content.len() <= idx {
    m.content.push(ContentBlock::Text(TextBlock::new("")));
}
m.content[idx] = content_block;
```

which left fabricated empty text blocks in `Message.content` for any gapped or out-of-order index.
It now asserts instead (`accumulator.rs:167-181`): `index != content.len()` returns
`Error::Stream` and pushes nothing.

The three official SDKs do not agree on this rule, so there is no single behavior to port:

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
Recorded as §12 row 3, which carries both the original defect and the divergence that replaced it.

Delta and stop events are a separate question, and there clauders follows TypeScript: an out-of-range
index is a silent no-op (`accumulator.rs:212-214`), where Python raises `IndexError` and Go returns an
error. Also 🔶, by the same standard — §12 row 18.

### 4.4 ✅ `message_delta` usage merge — delivered

`UsageDelta` (streaming.rs:207-226) now carries `input_tokens`, `cache_creation_input_tokens`,
`cache_read_input_tokens`, `output_tokens`, `output_tokens_details`, and `server_tool_use`.
`MessageAccumulator` (`accumulator.rs:137-172`) overwrites the snapshot's `input_tokens`,
`cache_creation_input_tokens`, `cache_read_input_tokens`, and `server_tool_use` when the delta
reports them, writes `output_tokens` unconditionally, and folds `stop_details` the same way — matching
the pinned Python SDK's fold policy.

**Correction to the prior revision's field list.** The prior revision of this section claimed both SDKs
overwrite `output_tokens_details` as part of the same merge. Re-reading the pinned source: Python's
`accumulate_event` (`_messages.py:503-518`) overwrites `input_tokens`, `cache_creation_input_tokens`,
`cache_read_input_tokens`, `output_tokens`, and `server_tool_use` when non-null, and separately folds
`delta.stop_details` (`:504-505` area) — but it does **not** assign `output_tokens_details` anywhere in
that block; TypeScript's `accumulateMessage` (`MessageStream.ts:575-600`) mirrors the same field list.
clauders decodes `output_tokens_details` on the wire type for completeness but does not fold it in the
accumulator, matching what the pinned source actually does rather than the field list previously
claimed here. That claim is struck.

`container`, `usage.service_tier`, and `usage.inference_geo` remain deliberately un-folded, unchanged
from the prior revision: neither pinned SDK copies `delta.container`, `usage.service_tier`, or
`usage.inference_geo` into its snapshot either, so clauders' `MessageMetaDelta.container`
(streaming.rs:196-199) and `Usage.{service_tier,inference_geo}` stay decoded-but-unfolded by design, not
by gap.

Row 13 is closed. The official policy was unanimous across Python, TypeScript, and Go, so this was pure
work, no design question.

One divergence introduced by the accumulator, small but real (§12 row 21): clauders writes
`stop_reason` and `stop_sequence` **only when the delta carries them** (`accumulator.rs:139-144`),
whereas all three SDKs assign unconditionally — including overwriting a resolved value with `null`
(`_messages.py:504-505`, `MessageStream.ts:576-577`). Kept deliberately: it makes a stray later
`message_delta` unable to clobber a resolved `stop_reason`. The difference is observable only if the
API sends a `null` after a non-null value — which we **assume** it does not, since the terminal delta
is the one that carries these fields. That assumption is not evidenced by either SDK's source; neither
guards, because neither needed to.

### 4.5 🔶 Stream-completeness and duplicate `message_start`

Two edge behaviors where the official SDKs disagree with each other, recorded so they are not
re-litigated. Both are pinned by tests in `accumulator.rs`, and both carry §12 rows — truncation is
row 19, duplicate `message_start` is row 20.

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

## 5. Forward compatibility — ✅ delivered

> **Status 2026-07-21.** This section is kept in its original diagnostic voice because §5.1's policy
> discussion is still the reference for *why* the arms are shaped the way they are. The defect it
> describes is **fixed**: all ten server-decoded enums carry a payload-carrying unknown arm plus
> `#[non_exhaustive]`, and `pause_turn` no longer fails. Read the present tense below as "before this
> was fixed", except in §5.1 from **Policy:** onward, which describes what shipped.

**This is a behavioral contract, not a nice-to-have.** The Anthropic versioning policy states new
content-block types and new SSE event types may be added within `anthropic-version: 2023-06-01`, and
the streaming guide says verbatim: *"new event types may be added, and your code should handle unknown
event types gracefully."* Both official SDKs implement that. clauders implements the opposite.

The clauders column below is **historical** — it records the pre-fix behavior at `afd1ab8`, and its
`file:line` citations resolve against that commit, not against the pinned `f0aab9d`. The delivered
behavior is the row beneath each one, added 2026-07-21.

| Scenario | Python | TypeScript | clauders (pre-fix, `afd1ab8`) | clauders now (`f0aab9d`) |
|---|---|---|---|---|
| Unknown `type` on a content block | Coerced into the **first union variant** (`TextBlock`), unknown keys retained on `__pydantic_extra__`; `.type` preserved verbatim (`_models.py:578 construct_type`, fallback loop `:638-642`) | **No validation at all** — `defaultParseResponse` (`internal/parse.ts:18`) returns raw parsed JSON; the object arrives intact and simply fails to narrow (`messages.ts:847`) | **`Error::Serde`, whole `Message` lost** (content.rs:28-39 closed `#[serde(tag = "type")]`) | ✅ `ContentBlock::Unknown(Value)` + `#[serde(skip_serializing)]` (content/block.rs:43-44) — payload retained, echo-back refused |
| Unknown `type` on a content-block delta | Coerced to `TextDelta`; accumulator no-ops on it | Passed through untouched; accumulator `default: checkNever(...)` no-ops (`MessageStream.ts:651`) | **`Error::Serde`, stream terminated** (streaming.rs:109-133, 293-304) | ✅ `ContentDelta::Unknown(Value)`, accumulator no-ops (streaming.rs:146-179, accumulator.rs:234) |
| Unknown SSE `event:` name | **Silently skipped** — allowlist chain in `_streaming.py:86`, no branch matches, nothing yielded | **Silently skipped** — same allowlist shape in `core/streaming.ts:51-142` | **`Error::Serde`, stream terminated** (streaming.rs:309-314) | ✅ `StreamEvent::Unknown(Value)`, yielded not dropped (streaming.rs:340-378) — clauders dispatches on `data.type`, not the `event:` name |
| Unknown field on a known object | Retained (`model_config = ConfigDict(extra="allow")`, `_models.py:107`) | Retained (no stripping) | Ignored — serde default. ✅ | unchanged ✅ |

Python's behavior is asserted by the SDK's own test, `tests/test_models.py:691
test_discriminated_unions_unknown_variant`, whose inline comment reads `# just chooses the first
variant`:

```python
assert isinstance(m, A)
assert m.type == "c"
assert m.data == None
assert m.new_thing == "bar"
```

clauders used to carry the mirror-image test, `parse_unknown_type_returns_serde_error`, asserting the
*failure* as intended behavior — a test that encoded the defect. It has been **inverted**: the test is
now `parse_unknown_type_yields_unknown_event_with_payload` (streaming.rs:498), asserting that the
payload survives.

Concrete blast radius today, before any future API change: a response containing `server_tool_use`,
`redacted_thinking`, `web_search_tool_result`, `container_upload`, `fallback`, or `connector_text`
fails to decode entirely. These are not hypothetical — they are emitted on GA paths.

### 5.1 The defect is not limited to content blocks

Every enum clauders decodes from a server response was closed — except `ErrorType`, which already
carried a presence-only `#[serde(other)]` unit arm — and each closed one hard-failed the enclosing
struct on an unrecognized value. The content-block and stream-event unions were the widest exposure,
but not the only one:

| Enum | Site | Exposure |
|---|---|---|
| **`StopReason`** | response.rs:64-87 | ✅ Delivered. `pause_turn` is now a typed `StopReason::PauseTurn` variant (§8.1) and an untagged `Unknown(String)` fallback arm retains the raw value instead of hard-failing the enclosing `Message`, so `create()` and `collect()` no longer return `Error::Serde` on an unrecognized stop reason. |
| `ErrorType` | error.rs:70 | Already tolerant — a presence-only `#[serde(other)]` unit arm meant it never hard-failed. Its gap was payload retention, not decode failure. |
| `BatchStatus` | batches/types.rs:140-152 | Plausible. Batch lifecycle states have grown before. |
| `MessageKind` | response.rs:53 | Latent — single-valued, stable. |
| `BatchKind` / `DeletedBatchKind` | batches/types.rs:127 / :230-237 | Latent — single-valued. |
| `ModelInfoKind` | models/types.rs:31 | Latent — single-valued. |

`StopReason` was the one that failed on GA paths; it is now closed (typed `PauseTurn` + `Unknown`
fallback). The rest remain latent — the distinction is timing, not kind.

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
split it is no longer used on the request path (that path now carries `ContentBlockParam`, which has no
`Unknown` arm); echoing a response-only block back is prevented at compile time by the
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
The top-level convenience form is delivered too: clauders sends the ephemeral `cache_control` breakpoint
at the top level and the server auto-places it on the last cacheable block, exactly as the official
SDKs describe it — the crate does not compute placement itself, it forwards the breakpoint. The
documented cache **pre-warm** call (`max_tokens: 0`) is representable (§2.3). Both rows in this table
are now closed.

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

**Status 2026-07-21 — delivered as `StopReason::PauseTurn`; `Unknown` retained for future values.**
This used to be a decode failure rather than a missing field: `StopReason` was closed, so `pause_turn`
failed the enclosing `Message` and both `create()` and `collect()` returned `Error::Serde` for the whole
response. Since `a155625` the enum carries `Unknown(String)` for forward compatibility, and since
`736363d` it also carries a first-class `PauseTurn` variant (response.rs:86) — pinned by a passing test
at response.rs:384. A caller now matches `StopReason::PauseTurn` directly and can act on a paused
server-tool turn through the type system; `Unknown(String)` remains for whatever value a future SDK
release adds next. §12 row 10 (WS C) is closed. See §5.1.

> **Correction to the prior revision.** That revision listed `model_context_window_exceeded` as a
> missing official stop reason. It is real, but it is **not** in the non-beta union — both SDKs type it
> only in their `beta` namespace. The API returns it without a beta header on Sonnet 4.5 and newer;
> older models need `model-context-window-exceeded-2025-08-26`. It is therefore out of scope for
> non-beta parity and in scope for a future beta surface.

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

## 9. Models API — ✅ delivered

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
(`capabilities.rs:8-37`, `52-71`, `74-103`). The ninth, `context_management`, is delivered but shaped
differently — see §9.1.

`ModelList` pagination (`data`/`has_more`/`first_id`/`last_id`) matches. ✅

### 9.1 🔶 `context_management` — delivered, diverging

Both official SDKs hardcode each dated context-management strategy as its own named optional field:
`clear_thinking_20251015?`, `clear_tool_uses_20250919?`, `compact_20260112?`, plus a `supported: bool`.
clauders instead models `ContextManagementCapability` as `supported: bool` plus a
`#[serde(flatten)] strategies: BTreeMap<String, CapabilitySupport>` (capabilities.rs:39-49) — every
dated key the server sends, named or not yet named, lands in the map under its wire key rather than a
struct field, pinned by `context_management_dated_keys_land_in_the_map` (capabilities.rs:139-150).

Graded 🔶, not a plain ✅, for the same reason as §4.3 row 3 and §12 rows 18-21: it is a deliberate
design choice, not a defect, but a caller porting field-access code from Python or TypeScript
(`caps.context_management.clear_thinking_20251015`) will not find a same-named field on
`ContextManagementCapability` and must index the map instead
(`caps.context_management.strategies.get("clear_thinking_20251015")`). The observable data is
equivalent — every dated key the server sends is retained, none dropped — but the access pattern
differs, and unlike the pinned SDKs a newly dated strategy needs no clauders code change to be
represented. Recorded as §12 row 23.

---

## 10. Message Batches — ✅

`batches::BatchesResource` implements all six operations; `Batch`, `BatchStatus`, `RequestCounts`,
`BatchList`, `BatchResultRow`, `BatchResult`, `DeletedMessageBatch` (batches/types.rs:45-219) match the
official shapes. No gaps identified.

One behavioral note for the future beta surface: a refused request inside a batch returns
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
so a caller copying an example no longer gets steered to a model two generations back. `custom()`
remains the escape hatch for any id without a dedicated constructor.

---

## 12. Ranked gaps

Ranked by *caller impact*, not by official-checklist order. Defects outrank absences because a missing
feature fails loudly at the call site while a defect corrupts data silently.

Every row cites the body section that establishes it. **Every body section that records a gap has a row
here** — that invariant is the point of this table, since implementation plans are built from it and a
finding that exists only in the prose gets dropped. As of 2026-07-21 the invariant extends to
**deliberate divergences** as well (row 3 and rows 18-21): a body section that records a chosen difference from
the official SDKs is just as easy to lose in prose as a gap, and losing it is worse — the next reader
"fixes" it.

**Status: WS A is complete — rows 1, 2, 3, and 4 are all delivered.** Rows 2 and 4 landed first (every
server-decoded enum carries a payload-carrying unknown arm, §5.1); rows 1 and 3 landed with
`MessageAccumulator` (§4.2, §4.3). Verified 2026-07-21 against pinned SDK source rather than against
this document's own prose. The rows are kept rather than deleted so the ranking stays legible.

**One qualification, which is the point of grading on behavior:** row 3 is closed by a *deliberate
divergence*, not by matching the pinned SDKs. clauders asserts the content-block index (Go's rule);
Python and TypeScript blind-append and ignore `index`. See §4.3 — it is graded 🔶, not ✅.

### Defects — incorrect runtime behavior

| # | Item | Class | Why here |
|---|---|---|---|
| 1 | Streaming accumulator: buffer `input_json_delta`, concat `thinking_delta`, replace on `signature_delta` (§4.2) | ✅ **delivered** | Was: silent wrong data, streaming tool use unusable with no signal of failure. Now handled in `MessageAccumulator`; end state matches Python/TS per-rule (§4.2 delivered-state table), including `input_json_delta` gating for both `ContentBlock::ToolUse` and `ContentBlock::ServerToolUse` (accumulator.rs:222-225, 305-309). Two rows in §4.2's delivered-state table are not ✅: recorded divergences, rows 18 and 19. |
| 2 | Unknown-variant tolerance on `ContentBlock` / `ContentDelta` / `StreamEvent` / **`StopReason`**; skip or surface — never error on — unknown SSE event names (§5, §5.1) | ✅ **delivered** | Was: hard decode failure on values the API emits **today**, violating the documented versioning contract, with `pause_turn` failing the entire `Message`. Now every one carries a payload-carrying `Unknown` arm + `#[non_exhaustive]`; `pause_turn` decodes as `StopReason::Unknown("pause_turn")` (§8.1). Payload fidelity matches TypeScript and exceeds Python, which mis-types the container (§5.1). |
| 3 | Index handling on `content_block_start`, instead of padding with fabricated `TextBlock::new("")` (§4.3) | 🔶 **delivered, diverging** | The fabricated-placeholder defect is gone. But clauders **asserts** `index == content.len()` and returns `Error::Stream`, following Go; Python and TypeScript blind-append and never read `index`. Against a conforming server the behaviors are indistinguishable; on a malformed stream clauders reports where the pinned SDKs silently misalign. Deliberate — see §4.3 for the full three-way comparison. |
| 4 | Unknown arms on the remaining server-decoded enums: `BatchStatus`, `MessageKind`, `BatchKind`, `DeletedBatchKind`, `ModelInfoKind`, and payload retention on `ErrorType` (§5.1) | ✅ **delivered** | Was the same failure mode as #2, not yet triggered. `BatchStatus` is the plausible grower; the rest are single-valued today. `ErrorType` is the exception — a presence-only `#[serde(other)]` arm already kept it from hard-failing, so its gap is payload retention only. `SystemSegmentKind` is **not** in scope: it is `Serialize`-only, so an unknown arm there is unreachable in both directions. Applying the §5.1 policy uniformly is cheaper than re-deciding per enum. |

### Blocks current-generation models

| # | Item | Class | Why here |
|---|---|---|---|
| 5 | `thinking` (3 variants + `display`) and `output_config.effort` (§2.1) | ✅ **delivered** | Both request parameters now exist, with `EffortLevel` shared between the two pillars. The prior framing — "cannot correctly drive any current-generation model" — was **overstated**: adaptive thinking is on by default and omitting `thinking` never produced a 400. The real gap, now closed, was the inability to *control* display, budget, and effort, or to disable thinking. `count_tokens` forwards both parameters. |
| 6 | Guard/deprecate `temperature`/`top_p`/`top_k`; allow `max_tokens: 0` (§2.2, §2.3) | ✅ **delivered** | All three setters carry `#[deprecated]` with the failure mode, so the warning reaches the call site rather than only the docs. `MaxTokens::new` is now infallible and `InvalidMaxTokens` is deleted — with `0` legal there is no invalid `u32`, so the cache pre-warm call is representable. |

### Structural

| # | Item | Class | Why here |
|---|---|---|---|
| 7 | Split the shared `ContentBlock` union into response and request-param directions (§3.3) | ✅ **delivered** | Two unions now: `ContentBlock` (response — `Text`/`Thinking`/`ToolUse`/`Unknown`) and `ContentBlockParam` (request — `Text`/`Thinking`/`ToolUse`/`ToolResult`, closed, no `Unknown`), joined by a fallible `TryFrom<ContentBlock> for ContentBlockParam` carry-forward conversion that fails with `UnsendableBlock` on a response-only block. Sending a response-only block is now a compile error, not a runtime failure. Prerequisite work for rows 8/9/14 is unblocked, not itself finished — see row 24 for the request union's still-pragmatic member count. |

### Capability

| # | Item | Class | Why here |
|---|---|---|---|
| 8 | Response blocks: `redacted_thinking`, `server_tool_use`, the six `*_tool_result` kinds, `container_upload` (§3.1) | ✅ **delivered** | All nine now decode as typed `ContentBlock` variants (`content/server_tool.rs`) instead of falling to `Unknown`. Each models its stable outer fields; the tool-specific result body stays `serde_json::Value` — a typed envelope, not full modeling of every server-tool result shape. `server_tool_use.name` is closed to the seven official server-tool names (`ServerToolName`); a `name` outside that set, or any wholly unmodeled block, still falls to `ContentBlock::Unknown` rather than failing decode — the graceful-degradation floor from #2 is unchanged. `ToolCaller` (`content/server_tool.rs:16`) models `direct` and the two dated `code_execution` server-tool callers, carried by `ServerToolUseBlock`, `WebSearchToolResultBlock`, and `WebFetchToolResultBlock`. |
| 9 | Vision (`image`) + PDF (`document`) input blocks (§3.2) | ✅ **delivered** | `ContentBlockParam` gained `Image(ImageBlock)` and `Document(DocumentBlock)` (`content/param.rs:37,39`). Image sources: `base64` (`ImageMediaType` closed to jpeg/png/gif/webp) and `url`; the Files API `file` source (beta) is omitted. Document sources: `base64` (`PdfMediaType::ApplicationPdf`), `text` (`PlainTextMediaType::TextPlain`), and `url` are fully typed; the `content` embedded-content source is kept as raw `serde_json::Value` — a bounded reduction, not a gap. `DocumentBlock` also carries `citations: Option<CitationsConfig>`, `context`, and `title`. |
| 10 | Response diagnostics: typed `pause_turn`, `stop_details`, `container`, `usage.{output_tokens_details,server_tool_use,service_tier,inference_geo}` (§8) | ✅ **delivered** | `StopReason::PauseTurn` is now a typed variant (`Unknown` retained for values a future release adds), and `Message.stop_details` / `Message.container` / `Usage.{output_tokens_details,server_tool_use,service_tier,inference_geo}` all exist (response.rs). A caller can act on a paused server-tool turn, a refusal category, or a container id through the type system rather than matching `Unknown("pause_turn")` or reading nothing at all. |
| 11 | Models API `capabilities` / `max_input_tokens` / `max_tokens` (§9) | ✅ **delivered** | `ModelInfo` now decodes `max_input_tokens`, `max_tokens` (both `Option<u32>`), and `capabilities: Option<ModelCapabilities>` (models/types.rs:70-78). Eight of the nine `ModelCapabilities` fields match the official named-field shape one-for-one; the ninth, `context_management`, is delivered but shaped as a dated-strategy map rather than named fields — a deliberate divergence, not a gap, recorded separately as row 23. |
| 12 | GA request params: `service_tier`, `inference_geo`, `container`, top-level `cache_control` (§2, §7) | ✅ **delivered** | All four are builder setters on `MessageRequest` (request.rs:565-589), each serialized only when set. Top-level `cache_control` reuses the existing per-block `CacheControl` type (§7): clauders sends the ephemeral breakpoint and the server auto-places it. |
| 13 | `message_delta` usage merge: carry `input_tokens`, `cache_*`, `server_tool_use` on `UsageDelta` and overwrite-cumulative (§4.4) | ✅ **delivered** | `UsageDelta` now carries the input-side counters and `server_tool_use`; `MessageAccumulator` overwrites them into the snapshot when the delta reports them, and folds `stop_details` the same way — matching the pinned Python/TypeScript overwrite-cumulative policy. Streaming callers no longer lose input-side counter updates after `message_start`. |
| 14 | `citations_delta` + `TextBlock.citations` (§3.1, §4.1) | ✅ **delivered** | `TextCitation` (`content/citation.rs:25`) models the five official citation-location kinds plus a payload-carrying `Unknown` floor. `TextBlock.citations: Option<Vec<TextCitation>>` (`content/text.rs:28`) carries them on the response path. `ContentDelta::CitationsDelta` (`streaming.rs:174`) decodes the streaming delta, and `MessageAccumulator` appends each citation onto the addressed text block's `citations` (`accumulator.rs:259`). |
| 15 | `eager_input_streaming` (GA fine-grained tool streaming); `ToolUseBlock.caller` (§6) | ✅ **delivered** | `Tool.eager_input_streaming` (tools.rs:70) and `ToolUseBlock.caller` (tools.rs:127), reusing `ToolCaller` (content/server_tool.rs:16). Both GA on the custom-tool path clauders already claims parity on. |
| 16 | `Role::System` mid-conversation messages (§3.4); refreshed `ModelId` constructors (§11) | ✅ **delivered** | `Role::System` is a first-class variant with `.add_system_text()` (request.rs:62, 402). `ModelId` gained `claude_opus_4_8()` / `claude_sonnet_5()` / `claude_fable_5()` (model_id.rs:96-110) and the crate's doctests were swept onto `claude_sonnet_5()`. |
| 22 | Client-side `DEPRECATED_MODELS` end-of-life warning on `create`/`stream` (§2) | ✅ **delivered** | `warn_if_deprecated_model` (`src/messages/resource.rs`) emits a `tracing::warn!` when `deprecated_model_eol` finds the request's model in the 20-entry `DEPRECATED_MODELS` table (verbatim copy of `messages.py:68` / `messages.ts:1305`), called from `create` and `stream` beside the sibling thinking warning. Warn-only, request unchanged. Divergence: `tracing::warn!` in place of `warnings.warn` / `console.warn` (same as the thinking warning). |

### Deferred — large independent surfaces

| # | Item | Class | Why here |
|---|---|---|---|
| 17 | Files API; server-side & Anthropic-defined tools; context management; MCP connector; typed parse helper (§1, §6, §8.4) | ❌ large | Evaluate as demand appears. Each is its own multi-cycle surface. |

### Accepted divergences — deliberate, recorded so they are not re-litigated

These carry the §12 invariant for body sections that record a *difference* rather than a *gap*: the
behavior is chosen, tested, and will not change without a decision. Listed because §4.2, §4.4 and §4.5
would otherwise be body-section findings with no row.

§4.3's `content_block_start` divergence is deliberately **not** repeated here: row 3 above already
carries it, because that row also carries the defect it replaced. Row 3 is 🔶 for exactly this reason.

| # | Item | Class | Why here |
|---|---|---|---|
| 18 | Delta/stop events with an out-of-range `index` are a silent no-op (§4.2, §4.3) | 🔶 divergence | Follows TypeScript (`.at()` → `undefined`); **Python raises `IndexError`** and Go returns an error. Same porting-surprise class as row 3's `content_block_start` policy, and graded the same way for consistency. |
| 19 | A stream ending without `message_stop` yields `Ok(partial)`; a tool-JSON buffer never closed by `content_block_stop` is never parsed (§4.5, §4.2) | 🔶 divergence | Follows Python and Go, which have no completeness check at all; TypeScript throws. The unclosed-buffer half is strictly *less* salvaging than Python, whose eager per-delta parse recovers complete key/value pairs (`{"a": 1,` → `{"a": 1}`). Reachable only on an already-broken stream. |
| 20 | A duplicate `message_start` replaces the snapshot and resets the JSON buffers (§4.5) | 🔶 divergence | All three official SDKs differ — TypeScript throws, Python ignores the second event and interleaves its blocks into the first message, Go replaces. clauders follows Go; Python's interleaving is the outcome worth avoiding outright. |
| 21 | `message_delta` writes `stop_reason`/`stop_sequence` only when present (§4.4) | 🔶 divergence | Python, TypeScript and Go all assign unconditionally, including overwriting a resolved value with `null`. The guard prevents a stray later delta from clobbering a resolved `stop_reason`. |
| 23 | `ModelCapabilities.context_management` is a flatten dated-strategy map, not named optional fields (§9.1) | 🔶 divergence | Both official SDKs hardcode `clear_thinking_20251015?` / `clear_tool_uses_20250919?` / `compact_20260112?` as named fields. clauders captures every dated key in a `BTreeMap<String, CapabilitySupport>` instead — no data is lost, but field-access code ported from Python or TypeScript must switch to a map lookup, and a caller gets forward compatibility with new dated keys the pinned SDKs would need a new field for. |
| 24 | `ContentBlockParam`'s membership is the pragmatic subset — `text`/`thinking`/`tool_use`/`tool_result` — not official's larger request-block superset (§3.2, §3.3) | 🔶 divergence | Both official SDKs' request unions are a 17-member superset of the 12-member response union, adding `image`, `document`, `search_result`, `tool_result`, and `mid_conversation_system`. clauders' request union is closed to the four block kinds already modelled today; `image` and `document` — vision and PDF input — are the two everyday members still missing, tracked separately as capability row 9. Not a defect: nothing sent today is malformed by the narrower membership, and widening the enum later is additive, not a breaking change to what already ships. |

---

## 13. Workstreams

The §12 rows grouped into implementable units. This section is **derived and opinionated** — §12 is the
factual record, this is a proposed sequencing over it. It goes stale on a different cadence than the
parity rows; re-derive it rather than trusting it after any §12 revision.

| WS | §12 rows | Primary files | Depends on |
|---|---|---|---|
| ~~**A — decode-path correctness**~~ ✅ **DONE 2026-07-21** | 1, 2, 3, 4 | `messages/accumulator.rs` (new), `messages/streaming.rs`, `messages/content.rs`, `messages/response.rs`, plus one-line arms in `messages/batches/types.rs`, `models/types.rs`, `error.rs` | — |
| ~~**B — current-model request surface**~~ ✅ **DONE 2026-07-21** | 5, 6 | `messages/request.rs`, `messages/structured_outputs.rs`, `types/numeric.rs` | — |
| ~~**C — response diagnostics & discovery**~~ ✅ **DONE 2026-07-23** | 10, 11, 12, 13, 16 | `messages/response.rs`, `models/types.rs`, `models/capabilities.rs` (new), `messages/request.rs`, `types/model_id.rs` | — |
| **D — content-block taxonomy** | 7, 8, 9, 14, 15 | `messages/content.rs`, `messages/tools.rs` | **A** |

**A, B, and C are delivered, so D is unblocked.** `ContentBlock::Unknown` exists, which is exactly the
arm that turns D from blocking work into progressive work. What remains is **D**.

**The one hard dependency is A → D.** Row 2 adds an unknown-variant arm to `ContentBlock`; that arm is
what turns D from blocking work into progressive work, because unknown blocks stop being fatal and the
remaining response members can land incrementally. Land D first and the unknown arm has to be
re-threaded through a much larger enum. Row 7 (the union split) is scoped into D as its first step for
the same reason.

Row 4 is scoped into A rather than spread across B/C/D because it is one mechanical pattern applied
crate-wide; splitting it by file would mean re-deciding the §5.1 policy in three later cycles.

A, B, and C are mutually independent and can be sequenced by preference. A and B overlap in no file.
A and C both touch `response.rs` — A adds `StopReason::Unknown`, C adds the typed `PauseTurn` variant
alongside it, so C is additive over A and the two do not conflict. B and C both touch `request.rs` but
in disjoint fields (`thinking`/`output_config` vs `service_tier`/`inference_geo`/`container`/`cache_control`).

Recommended order was **A → B → C → D.** A first because it is the only tier that corrupts data
silently and because it unblocks D; B second because, until it landed, the crate could not correctly
*drive* a current-generation model's thinking/effort surface; C and D are additive from there. A, B, and
C are all delivered, so what remains is D.

---

## 14. Methodology & caveats

- **clauders side** — read from source at 2026-07-20 (`crates/clauders/src/messages/`:
  `request.rs`, `response.rs`, `content.rs`, `tools.rs`, `streaming.rs`, `token_counting.rs`,
  `structured_outputs.rs`, `resource.rs`, `batches/`; `models/types.rs`; `types/{caching,model_id,
  system,numeric,version}.rs`). Authoritative. **Changed for the 2026-07-21 revision:** unlike the
  2026-07-20 revision — where `src/messages/` was unchanged and every delta was a correction to the
  *official* column or the *method* — this revision reflects real changes in clauders (`accumulator.rs`
  is new; `streaming.rs`, `content.rs`, `response.rs`, `error.rs`, `batches/types.rs`,
  `models/types.rs` all changed). Rows 1-4 of §12 moved because the code moved.
- **Official side** — read from pinned SDK source (commits in the header), cross-checked against the
  REST reference. Where the SDKs and the prose docs disagreed, **the SDK source wins** and the
  disagreement is noted inline (see §8.1 on `model_context_window_exceeded`).
- **Parity is graded on behavior.** A row is ✅ only if clauders produces the same observable result
  as the official SDKs for the same server response — not merely if the type exists. This is the
  change that reclassified §4.2, §5, and §9.
- Marks judge *capability and behavior*, not wire/name identity. clauders is idiomatic Rust (builders,
  exhaustive enums, newtypes), so equivalent features carry Rust-shaped names.
- **Beta surfaces are out of scope** for the ✅/❌ grading and are listed only for completeness:
  `mcp_servers`, `context_management`, `fallbacks`, `speed`, `diagnostics`, `task_budget`,
  `model_context_window_exceeded`, the Files API, and Agent Skills.
- The base SDKs iterate weekly. Re-verify against the pinned commits before treating any single ❌ as
  a hard commitment.

## Sources

- Vision & pillar mapping — [`../vision-and-strategy.md`](../vision-and-strategy.md)
- Agent SDK parity (the other pillar) — [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md)
- `anthropic-sdk-python` @ `3c8bdf14bc55377262f11d6c34b893834a02b3fc` — `types/`, `lib/streaming/_messages.py`, `_models.py`, `_streaming.py`
- `anthropic-sdk-typescript` @ `f84e8638fc74268d602d729747f7fd9fcbadbc71` — `resources/messages/messages.ts`, `resources/models.ts`, `lib/MessageStream.ts`, `core/streaming.ts`, `internal/parse.ts`
- REST reference — `platform.claude.com/docs/en/api/messages`, `/api/models-list`, `/build-with-claude/{streaming,vision,refusals-and-fallback,handling-stop-reasons}`
