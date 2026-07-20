# clauders Messages API — Feature Parity vs the Official Anthropic Base SDKs

Compares the `clauders` Messages API layer (`clauders::messages`, plus `clauders::models`,
`clauders::types`) against the **official Anthropic base SDKs** — the raw Messages API clients:

- **Python** — [`anthropic-sdk-python`](https://github.com/anthropics/anthropic-sdk-python)
- **TypeScript** — [`anthropic-sdk-typescript`](https://github.com/anthropics/anthropic-sdk-typescript)

This is **Pillar 1** of the [vision](../vision-and-strategy.md). The base SDK is a *different* official
product from the Claude Agent SDK covered in [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md):
the base SDK is a stateless `POST /v1/messages` client; the Agent SDK drives the `claude` CLI.
`clauders` targets both, in separate modules.

**As of:** 2026-07-20.

**Method — read this before trusting a row.** The previous revision of this document scored parity by
comparing *type surfaces* against prose documentation. That method produced false ✅s: a row can have
every type modelled correctly and still be broken at runtime, because parity is a property of
**behavior**, not of struct shape. This revision therefore grades against the **official SDK source**,
pinned to specific commits, and treats "does the SDK's accumulator/decoder do the same thing" as the
parity question. Three rows previously marked ✅ are ❌ or ⚠️ under that test.

**Sources, pinned:**

| Side | Version |
|---|---|
| clauders | working tree, `crates/clauders/src/` @ 2026-07-20 (no changes to `src/messages/` since 2026-07-13) |
| Python SDK | `anthropic-sdk-python` @ `3c8bdf14bc55377262f11d6c34b893834a02b3fc` (release 0.117.0, 2026-07-16) |
| TypeScript SDK | `anthropic-sdk-typescript` @ `f84e8638fc74268d602d729747f7fd9fcbadbc71` (2026-07-17) |
| REST reference | `platform.claude.com/docs/en/api/messages`, `.../build-with-claude/{streaming,vision,refusals-and-fallback,handling-stop-reasons}`, `.../api/models-list` — fetched 2026-07-20 |

Paths in the Python column are relative to `src/anthropic/`; TypeScript to `src/`.

---

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Full parity — equivalent capability, equivalent runtime behavior |
| ⚠️ | **Defect** — the capability is modelled but behaves incorrectly at runtime (silent data loss or hard failure) |
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
| **Streaming accumulation** | ⚠️ **defect — drops 3 of 5 delta kinds** |
| **Forward compatibility (every server-decoded enum)** | ⚠️ **defect — hard-fails where both official SDKs degrade; `StopReason`/`pause_turn` fails today** |
| **`thinking` / `output_config.effort`** | ❌ behind — blocks all current models |
| Response content-block taxonomy (12 official response members) | 🟡 4 of 12 |
| Request content-block taxonomy (17 official param members) | 🟡 4 of 17 — no vision, no PDF |
| Models API (`capabilities`, `max_input_tokens`, `max_tokens`) | ❌ behind — **was wrongly ✅ in the prior revision** |
| Response diagnostics (`container`, `stop_details`, `pause_turn`, usage sub-objects) | ❌ behind |
| GA request params (`service_tier`, `inference_geo`, `container`, top-level `cache_control`) | ❌ behind |
| Server-side & Anthropic-defined tools | ❌ behind |
| Files API / citations / context management / MCP connector | ❌ behind |

**One-line summary:** clauders is at genuine parity on the *non-streaming, text-and-custom-tools core*
— create, count-tokens, batches, caching, structured output, system prompts. It has **two runtime
defects** (streaming accumulation, forward compatibility) that make previously-claimed parity rows
false, and it cannot drive any current-generation model because the `thinking` surface is absent.

---

## 1. Resources & endpoints

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| `POST /v1/messages` | ✅ | ✅ | `messages::MessagesResource::create` (resource.rs:82) | ✅ |
| Streaming create (SSE) | ✅ | ✅ | `MessagesResource::stream` (resource.rs:172) | ✅ transport / ⚠️ accumulation — see §4 |
| `POST /v1/messages/count_tokens` | ✅ | ✅ | `count_tokens` (resource.rs:287) | ✅ |
| Message Batches — create/get/list/results/cancel/delete | ✅ | ✅ | `batches::BatchesResource` (all six) | ✅ |
| `GET /v1/models`, `GET /v1/models/{id}` | ✅ | ✅ | `models::ModelsResource::{list,get}` | ✅ endpoints / ❌ payload — see §9 |
| Files API (`/v1/files`) | ✅ (beta) | ✅ (beta) | ❌ | ❌ |

`count_tokens` is worth calling out as correct: `CountTokensBody` (token_counting.rs:48-62) projects
only `model`/`messages`/`system`/`tools`/`tool_choice`, matching the endpoint's accepted subset. Both
official SDKs do the same via a separate params type.

---

## 2. Request parameters

The official GA (non-beta) parameter set, verified identical in both SDKs
(`types/message_create_params.py:MessageCreateParamsBase`, `resources/messages/messages.ts:3055`):

| Param | Python | TS | clauders | Status |
|---|---|---|---|---|
| `model` | ✅ | ✅ | `model: ModelId` (request.rs:147) | ✅ |
| `max_tokens` | ✅ | ✅ | `max_tokens: MaxTokens` (request.rs:149) | 🟡 — rejects `0`; see below |
| `messages` | ✅ | ✅ | `messages: Vec<InputMessage>` (request.rs:151) | 🟡 — no `system` role; see §3 |
| `system` | ✅ | ✅ | `system: Option<SystemPrompt>` (request.rs:154) | ✅ |
| `stop_sequences` | ✅ | ✅ | (request.rs:166) | ✅ |
| `metadata` | ✅ | ✅ | `Metadata { user_id }` (request.rs:169) | ✅ |
| `stream` | ✅ | ✅ | hidden, resource-managed (request.rs:183) | ✅ |
| `tools` / `tool_choice` | ✅ | ✅ | (request.rs:172-175) | ✅ for custom tools — see §6 |
| `output_config.format` | ✅ | ✅ | `OutputConfig` (request.rs:178, structured_outputs.rs:43) | ✅ |
| **`output_config.effort`** | ✅ `low\|medium\|high\|xhigh\|max` | ✅ same | ❌ | ❌ |
| **`thinking`** | ✅ 3 variants | ✅ 3 variants | ❌ | ❌ |
| `temperature` | ✅ *(`@deprecated` in TS)* | ✅ *(`@deprecated`)* | `Temperature` (request.rs:157) | ⚠️ — see below |
| `top_p` | ✅ *(`@deprecated`)* | ✅ *(`@deprecated`)* | `TopP` (request.rs:160) | ⚠️ — see below |
| `top_k` | ✅ *(`@deprecated`)* | ✅ *(`@deprecated`)* | `TopK` (request.rs:163) | ⚠️ — see below |
| **`cache_control`** (top-level auto-place) | ✅ | ✅ | ❌ (per-block only) | ❌ |
| **`service_tier`** (`auto` \| `standard_only`) | ✅ | ✅ | ❌ | ❌ |
| **`inference_geo`** | ✅ | ✅ | ❌ | ❌ |
| **`container`** | ✅ | ✅ | ❌ | ❌ |
| `user_profile_id` (sent as `anthropic-user-profile-id` header) | ✅ | ✅ | ❌ | ❌ |
| `betas` / `anthropic-beta` header | ✅ | ✅ | multi, comma-joined (resource.rs:114-122) | ✅ |

Beta-gated params, absent from clauders, listed for completeness: `mcp_servers`, `context_management`,
`fallbacks`, `fallback_credit_token`, `speed`, `diagnostics`, `output_config.task_budget`.

### 2.1 ❌ The `thinking` / `effort` surface — still the top capability gap

`MessageRequest` has no `thinking` field. On every current-generation model (Fable 5, Mythos 5,
Opus 4.8/4.7, Sonnet 5) this means adaptive thinking cannot be enabled and `effort` cannot be set.

Official shape, identical in both SDKs (`types/thinking_config_param.py`, `messages.ts:1826`):

| Variant | Fields |
|---|---|
| `{"type": "adaptive"}` | optional `display: "summarized" \| "omitted"`; **no** `budget_tokens` |
| `{"type": "disabled"}` | `type` only — **no** `display` |
| `{"type": "enabled"}` | required `budget_tokens` (≥1024, `< max_tokens`); optional `display` |

`display` defaults to `omitted` on Fable 5 / Mythos 5 / Opus 4.8 / 4.7 / Sonnet 5, so a caller that
wants visible reasoning must set it explicitly.

### 2.2 ⚠️ Sampling params are exposed without the rejection semantics

`temperature` / `top_p` / `top_k` are first-class builder methods (request.rs:330-347) documented only
as "valid range 0.0..=1.0". The TypeScript SDK marks all three `@deprecated` **with the failure mode in
the annotation** (`messages.ts:3055` block): post-Opus-4.6 models accept only `temperature == 1.0`,
only `top_p >= 0.99`, and reject any `top_k` with a 400.

clauders carries the newtype validators (`Temperature::new` rejects `>1.0`, numeric.rs:77-82) but no
signal that setting these at all breaks current models. Combined with §2.1, the builder's documented
happy path produces a 400.

### 2.3 ⚠️ `max_tokens: 0` is rejected

`MaxTokens::new(0)` returns `Err(InvalidMaxTokens)` (numeric.rs:35-40). Official TS documents
`max_tokens` as *"set to `0` to pre-warm prompt cache without generating"* (`messages.ts:3055`), and
the prompt-caching guide uses `max_tokens: 0` as the canonical cache pre-warm call. clauders makes
that call unrepresentable.

---

## 3. Content blocks

The official SDKs use **two different unions** for the two directions. The prior revision of this doc
conflated them.

### 3.1 Response union — `ContentBlock`, 12 members

`types/content_block.py:ContentBlock` (Python, discriminated on `type`) and `messages.ts:847` (TS) —
identical membership:

| # | Official member | clauders | Status |
|---|---|---|---|
| 1 | `text` | `TextBlock` (content.rs:50) | ✅ — but no `citations` field |
| 2 | `thinking` | `ThinkingBlock` (content.rs:103) | ✅ |
| 3 | `redacted_thinking` | ❌ | ❌ |
| 4 | `tool_use` | `ToolUseBlock` (tools.rs:108) | ✅ |
| 5 | `server_tool_use` | ❌ | ❌ |
| 6 | `web_search_tool_result` | ❌ | ❌ |
| 7 | `web_fetch_tool_result` | ❌ | ❌ |
| 8 | `code_execution_tool_result` | ❌ | ❌ |
| 9 | `bash_code_execution_tool_result` | ❌ | ❌ |
| 10 | `text_editor_code_execution_tool_result` | ❌ | ❌ |
| 11 | `tool_search_tool_result` | ❌ | ❌ |
| 12 | `container_upload` | ❌ | ❌ |

Additional block types the API emits on GA paths that are **not** in the SDK response union (they arrive
via beta surfaces or fallback flows): `fallback` (`{type, from{model}, to{model}}`) and `connector_text`.

Official `TextBlock` carries `citations: Array<TextCitation> | null` (`messages.ts:1590`) — a 5-member
union of `CitationCharLocation`, `CitationPageLocation`, `CitationContentBlockLocation`,
`CitationsWebSearchResultLocation`, `CitationsSearchResultLocation`. clauders' `TextBlock` has `text` +
`cache_control` only.

### 3.2 Request union — `ContentBlockParam`, 17 members

`types/content_block_param.py` (Python — note: a **plain** union, no discriminator) and `messages.ts:864`:

All 12 response members above, **plus** five input-only members:

| Official input-only member | clauders | Status |
|---|---|---|
| `image` | ❌ | ❌ — no vision |
| `document` | ❌ | ❌ — no PDF |
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

### 3.3 clauders' single shared union

`ContentBlock` (content.rs:28-39) is one enum of 4 used for **both** directions: `Text`, `Thinking`,
`ToolUse`, `ToolResult`. That is a defensible Rust simplification (the API tolerates it, since
`tool_result` is only ever sent and `thinking` is only ever received), but it means the response path
accepts a block the API never returns and the request path accepts blocks it should not send. Parity
work should split the union or document the asymmetry explicitly.

### 3.4 ⚠️ `Role` has no `System` variant

`Role` (request.rs:56-61) is `User | Assistant`. Official `MessageParam.role` is
`'user' | 'assistant' | 'system'` (`messages.ts:1206`) — mid-conversation system messages are GA on
Claude Opus 4.8 with no beta header, and are the cache-preserving way to inject operator instructions
mid-session. Not representable in clauders.

---

## 4. Streaming

### 4.1 Event and delta taxonomy — ✅

`RawMessageStreamEvent`, 6 members, identical in both SDKs
(`types/raw_message_stream_event.py`, `messages.ts:1436`):

| Official | clauders | Status |
|---|---|---|
| `message_start` | `StreamEvent::MessageStart` (streaming.rs:57) | ✅ |
| `content_block_start` | (streaming.rs:62) | ✅ |
| `content_block_delta` | (streaming.rs:69) | ✅ |
| `content_block_stop` | (streaming.rs:76) | ✅ |
| `message_delta` | (streaming.rs:82) | ✅ |
| `message_stop` | (streaming.rs:89) | ✅ |

`ping` and `error` are **not** union members officially — both SDKs handle them in the transport layer
(`_streaming.py:151` / `streaming.ts:51-142`: `ping` → skip, `error` → throw an `APIError`). clauders
models them as `StreamEvent::Ping` / `StreamEvent::Error` (streaming.rs:91-96) and maps `Error` to
`Error::Api` in `collect()` (streaming.rs:251-262). Equivalent behavior, different placement. ✅

`RawContentBlockDelta`, 5 members (`types/raw_content_block_delta.py`, `messages.ts:1338`):

| Official | clauders | Status |
|---|---|---|
| `text_delta` | `ContentDelta::TextDelta` (streaming.rs:113) | ✅ |
| `input_json_delta` | `InputJsonDelta` (streaming.rs:129) | ✅ modelled |
| `thinking_delta` | `ThinkingDelta` (streaming.rs:118) | ✅ modelled |
| `signature_delta` | `SignatureDelta` (streaming.rs:123) | ✅ modelled |
| **`citations_delta`** | ❌ | ❌ |

> The prior revision listed `redacted_thinking_delta` as an official delta type that clauders was
> missing. **No such delta exists** in either SDK. That row was wrong and has been removed.

### 4.2 ⚠️ Accumulation — `MessageStream::collect()` drops 3 of 5 delta kinds

This is the most severe defect in the crate. `collect()` (streaming.rs:197-270) handles exactly one
delta kind (streaming.rs:224-233):

```rust
StreamEvent::ContentBlockDelta { index, delta } => {
    if let Some(ref mut m) = accumulated {
        let idx = index as usize;
        if let (Some(ContentBlock::Text(tb)), ContentDelta::TextDelta { text }) =
            (m.content.get_mut(idx), delta)
        {
            tb.text.push_str(&text);
        }
    }
}
```

`InputJsonDelta`, `ThinkingDelta`, and `SignatureDelta` fall through the `if let` and are discarded.
No error, no warning. Consequences:

- **Streaming tool use returns empty arguments.** `content_block_start` carries `"input": {}`; every
  `input_json_delta` is dropped; the assembled `ToolUseBlock.input` stays `{}`. A caller dispatches
  the tool with no arguments and cannot tell that anything went wrong.
- **Streaming extended thinking returns empty text**, and the `signature` is lost. The signature must
  round-trip verbatim on the next turn or the API rejects the request.

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

### 4.3 ⚠️ Index handling

clauders pads with placeholder text blocks when `index` exceeds the current length (streaming.rs:216-222):

```rust
while m.content.len() <= idx {
    m.content.push(ContentBlock::Text(TextBlock::new("")));
}
m.content[idx] = content_block;
```

Both official SDKs instead **append and ignore `index` on `content_block_start`** (Python
`_messages.py:456-463` with a literal `# TODO: check index`; TS `MessageStream.ts:601`
`snapshot.content.push({ ...event.content_block })`), then index into the array on delta. clauders'
padding leaves fabricated empty text blocks in `Message.content` for any gapped or out-of-order index.

### 4.4 🟡 `message_delta` usage merge

`UsageDelta` (streaming.rs:151-155) models `output_tokens` only, and `collect()` (streaming.rs:242-248)
preserves `message_start`'s input-side counts.

Official `MessageDeltaUsage` carries `input_tokens`, `cache_creation_input_tokens`,
`cache_read_input_tokens`, `output_tokens`, `output_tokens_details`, `server_tool_use`. Both SDKs
**overwrite** the snapshot with the cumulative value when non-null, never sum
(`_messages.py:503-518`, `MessageStream.ts:575-600`). Neither copies `delta.container`,
`usage.cache_creation`, `usage.service_tier`, or `usage.inference_geo` into the snapshot — so those
omissions in clauders are *not* parity gaps; the missing input/cache/server-tool merge is.

`RawMessageDeltaEvent.Delta` also carries `container` and `stop_details` officially; clauders'
`MessageMetaDelta` (streaming.rs:139-145) has `stop_reason` + `stop_sequence` only.

---

## 5. Forward compatibility — ⚠️ defect

**This is a behavioral contract, not a nice-to-have.** The Anthropic versioning policy states new
content-block types and new SSE event types may be added within `anthropic-version: 2023-06-01`, and
the streaming guide says verbatim: *"new event types may be added, and your code should handle unknown
event types gracefully."* Both official SDKs implement that. clauders implements the opposite.

| Scenario | Python | TypeScript | clauders |
|---|---|---|---|
| Unknown `type` on a content block | Coerced into the **first union variant** (`TextBlock`), unknown keys retained on `__pydantic_extra__`; `.type` preserved verbatim (`_models.py:578 construct_type`, fallback loop `:638-642`) | **No validation at all** — `defaultParseResponse` (`internal/parse.ts:18`) returns raw parsed JSON; the object arrives intact and simply fails to narrow (`messages.ts:847`) | **`Error::Serde`, whole `Message` lost** (content.rs:28-39 closed `#[serde(tag = "type")]`) |
| Unknown `type` on a content-block delta | Coerced to `TextDelta`; accumulator no-ops on it | Passed through untouched; accumulator `default: checkNever(...)` no-ops (`MessageStream.ts:651`) | **`Error::Serde`, stream terminated** (streaming.rs:109-133, 293-304) |
| Unknown SSE `event:` name | **Silently skipped** — allowlist chain in `_streaming.py:86`, no branch matches, nothing yielded | **Silently skipped** — same allowlist shape in `core/streaming.ts:51-142` | **`Error::Serde`, stream terminated** (streaming.rs:309-314) |
| Unknown field on a known object | Retained (`model_config = ConfigDict(extra="allow")`, `_models.py:107`) | Retained (no stripping) | Ignored — serde default. ✅ |

Python's behavior is asserted by the SDK's own test, `tests/test_models.py:691
test_discriminated_unions_unknown_variant`, whose inline comment reads `# just chooses the first
variant`:

```python
assert isinstance(m, A)
assert m.type == "c"
assert m.data == None
assert m.new_thing == "bar"
```

clauders has the mirror-image test, `parse_unknown_type_returns_serde_error` (streaming.rs:411),
asserting the *failure* as intended behavior. That test encodes the defect.

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
| **`StopReason`** | response.rs:61 | **Live failure.** `pause_turn` is missing (§8.1) and the API returns it on every server-tool turn that hits the 10-iteration limit. An unrecognized value fails the entire `Message`, so both `create()` and `collect()` return `Error::Serde`. |
| `ErrorType` | error.rs:70 | Already tolerant — a presence-only `#[serde(other)]` unit arm meant it never hard-failed. Its gap was payload retention, not decode failure. |
| `BatchStatus` | batches/types.rs:135 | Plausible. Batch lifecycle states have grown before. |
| `MessageKind` | response.rs:53 | Latent — single-valued, stable. |
| `BatchKind` / `DeletedBatchKind` | batches/types.rs:127 / :219 | Latent — single-valued. |
| `ModelInfoKind` | models/types.rs:31 | Latent — single-valued. |

`StopReason` is the one that fails today; the rest are latent. The distinction is timing, not kind.

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

`ContentBlock` is the one asymmetry: it alone also derives `Serialize` and is used on the request path,
so its unknown arm carries `#[serde(skip_serializing)]` and an attempt to echo an unknown block back
surfaces as `Error::Serde`. The other nine are `Deserialize`-only.

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
| `eager_input_streaming` (fine-grained tool streaming, GA) | ✅ | ✅ | ❌ | ❌ |
| `ToolUseBlock.caller` (`direct` / `server_tool` / `server_tool_20260120`) | ✅ | ✅ | ❌ | ❌ |
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
| Carriers: system segment / text / tool / tool_use / tool_result | ✅ | ✅ | system.rs:149, content.rs:58, tools.rs:57/118/152 | ✅ |
| Cache-aware usage counters | ✅ | ✅ | `Usage.cache_creation/read` (response.rs:107-113) | ✅ |
| `cache_creation` per-tier breakdown | ✅ | ✅ | `CacheCreation` (response.rs:78-84) | ✅ |
| Top-level `cache_control` (auto-place on last cacheable block) | ✅ | ✅ | ❌ | ❌ |

Explicit per-block caching is at genuine parity, including both TTL tiers and the tier-split accounting.
Only the top-level convenience form is missing — and note §2.3: the documented cache **pre-warm** call
(`max_tokens: 0`) is currently unrepresentable, which makes this gap larger in practice than it looks.

---

## 8. Response & usage

Official `Message` — 10 fields, identical in both SDKs (`types/message.py`, `messages.ts:1020`):

| Field | clauders | Status |
|---|---|---|
| `id` / `type` / `role` / `model` / `content` / `stop_sequence` | `Message` (response.rs:26-45) | ✅ |
| `stop_reason` | `StopReason` (response.rs:61-72) | 🟡 — 5 of 6, see below |
| **`stop_details`** | ❌ | ❌ |
| **`container`** | ❌ | ❌ |
| `usage` | `Usage` (response.rs:99-114) | 🟡 — see below |

### 8.1 `stop_reason` — non-beta is exactly 6 values

`types/stop_reason.py:StopReason` and `messages.ts:1588`, both closed with no catch-all:

`end_turn` · `max_tokens` · `stop_sequence` · `tool_use` · **`pause_turn`** · `refusal`

clauders has five (response.rs:61-72), missing **`pause_turn`** — which the API returns whenever a
server-tool loop hits its 10-iteration limit, i.e. on every long server-tool turn.

**This is a decode failure, not a missing field.** `StopReason` is a closed enum, so an unrecognized
value fails the enclosing `Message`, and both `create()` and `collect()` return `Error::Serde` for the
whole response. It therefore appears twice in §12: as part of row 2 (add the unknown arm, stop the hard
failure) and as row 10 (add the typed `PauseTurn` variant so a caller can act on it). See §5.1.

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
| `input_tokens` / `output_tokens` | ✅ (response.rs:102-104) | ✅ |
| `cache_creation_input_tokens` / `cache_read_input_tokens` | ✅ (response.rs:107-110) | ✅ |
| `cache_creation.{ephemeral_5m,ephemeral_1h}_input_tokens` | ✅ (response.rs:78-84) | ✅ |
| **`output_tokens_details.thinking_tokens`** | ❌ | ❌ |
| **`server_tool_use.{web_search_requests,web_fetch_requests}`** | ❌ | ❌ |
| **`service_tier`** (`standard` \| `priority` \| `batch`) | ❌ | ❌ |
| **`inference_geo`** | ❌ | ❌ |
| `iterations[]` (beta, server-side fallback) | ❌ | — beta |

`Usage::total_input_tokens` (response.rs:135-139) is a clauders-only convenience with no official
counterpart. Harmless, keep.

### 8.4 Typed parse helper

Python ships `client.messages.parse()` + `ParsedMessage`/`ParsedTextBlock.parsed_output` and parses at
`content_block_stop` during streaming (`_messages.py:499-502`). TypeScript ships `zodOutputFormat` +
`parsed_output`. clauders exposes the raw `output_config.format.schema` only — the caller parses the
first text block by hand. 🟡

---

## 9. Models API — ❌ (regression against the prior revision's ✅)

Official `ModelInfo` — `types/model_info.py`, `resources/models.ts:177`, confirmed against
`GET /v1/models`:

| Field | clauders (`models/types.rs:53-64`) | Status |
|---|---|---|
| `id` | ✅ | ✅ |
| `display_name` | ✅ | ✅ |
| `created_at` | ✅ (kept as `String`) | ✅ |
| `type` | ✅ `ModelInfoKind` | ✅ |
| **`max_input_tokens`** | ❌ | ❌ |
| **`max_tokens`** | ❌ | ❌ |
| **`capabilities`** | ❌ | ❌ |

`ModelCapabilities` — 9 fields (`types/model_capabilities.py`, `models.ts:130`):

```
batch, citations, code_execution, image_input, pdf_input, structured_outputs  → CapabilitySupport { supported: bool }
context_management → { clear_thinking_20251015?, clear_tool_uses_20250919?, compact_20260112?, supported }
effort             → { low, medium, high, max, xhigh?, supported }
thinking           → { supported, types: { adaptive, enabled } }
```

This is the API's live capability-discovery surface — the supported way to answer "does this model take
`xhigh` effort / adaptive thinking / PDF input" without hardcoding a table. clauders returns none of it,
so the Models resource is currently a name-and-date lookup rather than a capability probe.

`ModelList` pagination (`data`/`has_more`/`first_id`/`last_id`) matches. ✅

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
| Open model-id type | TS `Model = <15 known ids> \| (string & {})` (`messages.ts:1258`); Python `ModelParam` accepts `str` | `ModelId::custom` + 4 headline ctors (types/model_id.rs:46-92) | ✅ same escape hatch |
| Headline model constructors | 15 ids incl. `claude-fable-5`, `claude-opus-4-8`, `claude-sonnet-5` | `claude_opus_4_7`, `claude_sonnet_4_6`, `claude_sonnet_4_5`, `claude_haiku_4_5` | 🟡 stale set |
| `system` as string or block array | ✅ | `SystemPrompt::{Text,Segments}` (types/system.rs:57-64) | ✅ |
| Per-segment system `cache_control` | ✅ | `SystemSegment.cache_control` (types/system.rs:149) | ✅ |
| Deprecation signalling on sampling params | ✅ `@deprecated` + 400 semantics in TS | none | ❌ — see §2.2 |
| Unknown-field tolerance on known objects | ✅ | ✅ serde default | ✅ |
| Client-side model/thinking mismatch warning | TS `console.warn` on `enabled` thinking + Opus 4.6 (`messages.ts:79-87`) | ❌ | ❌ — optional |

The `ModelId` gap is cosmetic (`custom()` covers everything) but the doctests and examples throughout
the crate all use `claude_sonnet_4_5()`, which steers callers to a model two generations back.

---

## 12. Ranked gaps

Ranked by *caller impact*, not by official-checklist order. Defects outrank absences because a missing
feature fails loudly at the call site while a defect corrupts data silently.

Every row cites the body section that establishes it. **Every body section that records a gap has a row
here** — that invariant is the point of this table, since implementation plans are built from it and a
finding that exists only in the prose gets dropped.

**Status:** rows 2 and 4 are **delivered** — every server-decoded enum now carries a payload-carrying
unknown arm (§5.1). Rows 1 and 3 remain open: they are accumulator work, and the accumulator has not
been built yet. The rows are kept here rather than deleted so the ranking stays legible.

### Defects — incorrect runtime behavior

| # | Item | Class | Why here |
|---|---|---|---|
| 1 | Streaming accumulator: buffer `input_json_delta`, concat `thinking_delta`, replace on `signature_delta` (§4.2) | ⚠️ defect | Silent wrong data. Streaming tool use is unusable and gives no signal that it failed. |
| 2 | Unknown-variant tolerance on `ContentBlock` / `ContentDelta` / `StreamEvent` / **`StopReason`**; skip or surface — never error on — unknown SSE event names (§5, §5.1) | ⚠️ defect | Hard decode failure on values the API emits **today**; violates the documented versioning contract. Both official SDKs degrade instead. `StopReason` is the live one: `pause_turn` fails the entire `Message`. |
| 3 | Index handling: append on `content_block_start` and ignore `index`, instead of padding with fabricated `TextBlock::new("")` (§4.3) | ⚠️ defect | Gapped or out-of-order indices leave placeholder blocks in `Message.content`. Both official SDKs append and ignore the index. |
| 4 | Unknown arms on the remaining server-decoded enums: `BatchStatus`, `MessageKind`, `BatchKind`, `DeletedBatchKind`, `ModelInfoKind`, and payload retention on `ErrorType` (§5.1) | ⚠️ latent defect | Same failure mode as #2, not yet triggered. `BatchStatus` is the plausible grower; the rest are single-valued today. `ErrorType` is the exception — a presence-only `#[serde(other)]` arm already kept it from hard-failing, so its gap is payload retention only. `SystemSegmentKind` is **not** in scope: it is `Serialize`-only, so an unknown arm there is unreachable in both directions. Applying the §5.1 policy uniformly is cheaper than re-deciding per enum. |

### Blocks current-generation models

| # | Item | Class | Why here |
|---|---|---|---|
| 5 | `thinking` (3 variants + `display`) and `output_config.effort` (§2.1) | ❌ capability | Cannot correctly drive any current-generation model. |
| 6 | Guard/deprecate `temperature`/`top_p`/`top_k`; allow `max_tokens: 0` (§2.2, §2.3) | ⚠️ ergonomics | The documented happy path 400s; cache pre-warm is unrepresentable. Cheap to fix alongside #5. |

### Structural

| # | Item | Class | Why here |
|---|---|---|---|
| 7 | Split the shared `ContentBlock` union into response (12 members) and param (17 members) directions (§3.3) | 🟡 structural | Prerequisite for the taxonomy work in #8/#9/#14; today one 4-member enum serves both directions, so the request path accepts blocks it must not send and the response path accepts one the API never returns. |

### Capability

| # | Item | Class | Why here |
|---|---|---|---|
| 8 | Response blocks: `redacted_thinking`, `server_tool_use`, the five `*_tool_result` kinds, `container_upload` (§3.1) | ❌ capability | Largely subsumed by #2 — once unknown blocks stop being fatal, these become progressive typing work. |
| 9 | Vision (`image`) + PDF (`document`) input blocks (§3.2) | ❌ capability | The most-requested everyday base-SDK feature. |
| 10 | Response diagnostics: typed `pause_turn`, `stop_details`, `container`, `usage.{output_tokens_details,server_tool_use,service_tier,inference_geo}` (§8) | ❌ capability | Cheap. `pause_turn` stops being a decode failure under #2, but still needs its typed variant here before a caller can act on a paused server-tool turn. |
| 11 | Models API `capabilities` / `max_input_tokens` / `max_tokens` (§9) | ❌ capability | Restores the row this doc previously mis-scored as ✅. |
| 12 | GA request params: `service_tier`, `inference_geo`, `container`, top-level `cache_control` (§2, §7) | ❌ capability | Small, mechanical. |
| 13 | `message_delta` usage merge: carry `input_tokens`, `cache_*`, `server_tool_use` on `UsageDelta` and overwrite-cumulative (§4.4) | 🟡 partial | Streaming callers currently lose every input-side counter update after `message_start`. |
| 14 | `citations_delta` + `TextBlock.citations` (§3.1, §4.1) | ❌ capability | Pairs with the citations feature as a whole. |
| 15 | `eager_input_streaming` (GA fine-grained tool streaming); `ToolUseBlock.caller` (§6) | ❌ capability | Both are GA on the custom-tool path clauders already claims parity on. |
| 16 | `Role::System` mid-conversation messages (§3.4); refreshed `ModelId` constructors (§11) | ❌ small | Low effort, low risk. |

### Deferred — large independent surfaces

| # | Item | Class | Why here |
|---|---|---|---|
| 17 | Files API; server-side & Anthropic-defined tools; context management; MCP connector; typed parse helper (§1, §6, §8.4) | ❌ large | Evaluate as demand appears. Each is its own multi-cycle surface. |

---

## 13. Workstreams

The §12 rows grouped into implementable units. This section is **derived and opinionated** — §12 is the
factual record, this is a proposed sequencing over it. It goes stale on a different cadence than the
parity rows; re-derive it rather than trusting it after any §12 revision.

| WS | §12 rows | Primary files | Depends on |
|---|---|---|---|
| **A — decode-path correctness** | 1, 2, 3, 4 | `messages/streaming.rs`, `messages/content.rs`, `messages/response.rs`, plus one-line arms in `messages/batches/types.rs`, `models/types.rs`, `error.rs` | — |
| **B — current-model request surface** | 5, 6 | `messages/request.rs`, `messages/structured_outputs.rs`, `types/numeric.rs` | — |
| **C — response diagnostics & discovery** | 10, 11, 12, 13, 16 | `messages/response.rs`, `models/types.rs`, `messages/request.rs` | — |
| **D — content-block taxonomy** | 7, 8, 9, 14, 15 | `messages/content.rs`, `messages/tools.rs` | **A** |

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

Recommended order: **A → B → C → D.** A first because it is the only tier that corrupts data silently
and because it unblocks D; B second because until it lands the crate cannot drive any current-generation
model; C and D are additive from there.

---

## 14. Methodology & caveats

- **clauders side** — read from source at 2026-07-20 (`crates/clauders/src/messages/`:
  `request.rs`, `response.rs`, `content.rs`, `tools.rs`, `streaming.rs`, `token_counting.rs`,
  `structured_outputs.rs`, `resource.rs`, `batches/`; `models/types.rs`; `types/{caching,model_id,
  system,numeric,version}.rs`). Authoritative. `src/messages/` is unchanged since the prior revision,
  so every difference between the two revisions of this document is a correction to the *official*
  column or to the *method*, never a change in clauders.
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
