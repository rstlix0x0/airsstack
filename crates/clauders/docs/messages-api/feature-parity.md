# clauders Messages API — Feature Parity vs the Official Anthropic Base SDK

Compares the `clauders` Messages API layer (module `clauders::messages`, plus `clauders::models` and
`clauders::client`) against the **official Anthropic base SDKs** — the raw Messages API clients:

- **Python** — `anthropic` (`client.messages.*`, `client.models.*`, `client.beta.files.*`)
- **TypeScript** — `@anthropic-ai/sdk`

This is **Pillar 1** of the [vision](../vision-and-strategy.md). The base SDK is a *different* official
product from the Claude Agent SDK covered in [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md):
the base SDK is a stateless `POST /v1/messages` client; the Agent SDK drives the `claude` CLI. `clauders`
targets both, in separate modules.

**As of:** 2026-07-13. clauders side read from source; official side cross-checked against the Anthropic
SDK reference (Python/TypeScript). The base SDKs iterate quickly — re-verify any single ❌ against the
live reference before treating it as a hard commitment.

---

## Legend

| Mark | Meaning |
|------|---------|
| ✅ | Full parity — equivalent capability exists |
| 🟡 | Partial — core exists, narrower than official |
| ❌ | Absent in clauders |
| — | Not applicable |

---

## 1. Resources & endpoints

| Capability | Python | TS | clauders | Status |
|---|---|---|---|---|
| `POST /v1/messages` (create) | ✅ | ✅ | `messages::MessagesResource::create` (resource.rs:82) | ✅ |
| Streaming create (SSE) | ✅ `.stream()` | ✅ `.stream()` | `MessagesResource::stream` (resource.rs:172) | ✅ |
| `POST /v1/messages/count_tokens` | ✅ | ✅ | `count_tokens` (resource.rs:287) | ✅ |
| Message Batches — create/get/list/results/cancel/delete | ✅ | ✅ | `batches::BatchesResource` (all six) | ✅ |
| `GET /v1/models`, `GET /v1/models/{id}` | ✅ | ✅ | `models::ModelsResource::{list,get}` | ✅ |
| Files API (`/v1/files` — upload/list/get/download/delete) | ✅ (beta) | ✅ (beta) | ❌ | ❌ |

**Verdict:** ✅ full parity on the core endpoints (messages, streaming, count-tokens, batches, models).
Only the **Files API** is absent.

---

## 2. Request parameters (`MessageRequest`)

| Param (official) | Python | TS | clauders field | Status |
|---|---|---|---|---|
| `model` / `max_tokens` / `messages` | ✅ | ✅ | `model` / `max_tokens` / `messages` (request.rs:147–151) | ✅ |
| `system` | ✅ | ✅ | `system: SystemPrompt` (request.rs:154) | ✅ |
| `temperature` / `top_p` / `top_k` | ✅ | ✅ | (request.rs:157–163) | ✅ |
| `stop_sequences` | ✅ | ✅ | (request.rs:166) | ✅ |
| `metadata` | ✅ | ✅ | `Metadata` (request.rs:169) | ✅ |
| `stream` | ✅ | ✅ | hidden; managed by resource (request.rs:183) | ✅ |
| `tools` / `tool_choice` | ✅ | ✅ | `tools` / `tool_choice` (request.rs:172–175) | ✅ |
| `output_config.format` (JSON schema) | ✅ | ✅ | `output_config: OutputConfig` (request.rs:178) | ✅ |
| **`thinking`** (`adaptive` / `display`) | ✅ | ✅ | ❌ | ❌ |
| **`output_config.effort`** (`low`…`max`) | ✅ | ✅ | ❌ | ❌ |
| **`output_config.task_budget`** (beta) | ✅ | ✅ | ❌ | ❌ |
| Top-level auto `cache_control` | ✅ | ✅ | ❌ (per-block only) | 🟡 |
| `container` (skills / code-exec) | ✅ | ✅ | ❌ | ❌ |
| `context_management` (edits / compaction) | ✅ | ✅ | ❌ | ❌ |
| `mcp_servers` (MCP connector) | ✅ | ✅ | ❌ | ❌ |
| `fallbacks` (refusal fallback, beta) | ✅ | ✅ | ❌ | ❌ |
| `speed` (fast mode, beta) | ✅ | ✅ | ❌ | ❌ |
| `betas` / `anthropic-beta` header | ✅ | ✅ | multi, comma-joined (resource.rs:116–122) | ✅ |

**⚠️ Correctness-critical gap — the `thinking` surface.** clauders models `temperature`/`top_p`/`top_k`
but has **no `thinking` field at all**. On the current-generation models (Opus 4.8/4.7, Sonnet 5,
Fable 5): sampling parameters are **rejected with 400**, and **adaptive thinking is the only thinking
mode**. So driving those models through clauders today means sampling params must be left unset (fine —
they're `Option`), but adaptive thinking and `effort` **cannot be enabled at all**. This is a capability
gap, not merely a missing knob, and it is the highest-priority Messages-API parity item.

---

## 3. Content blocks (input & output)

| Block | Python | TS | clauders | Status |
|---|---|---|---|---|
| `text` | ✅ | ✅ | `TextBlock` (content.rs:32) | ✅ |
| `thinking` (output) | ✅ | ✅ | `ThinkingBlock` (content.rs:34) | ✅ |
| `tool_use` (output) | ✅ | ✅ | `ToolUseBlock` (tools.rs:109) | ✅ |
| `tool_result` (input) | ✅ | ✅ | `ToolResultBlock` (tools.rs:142) | ✅ |
| **`image`** (base64 / url) — vision | ✅ | ✅ | ❌ | ❌ |
| **`document`** / PDF | ✅ | ✅ | ❌ | ❌ |
| `redacted_thinking` | ✅ | ✅ | ❌ | ❌ |
| `server_tool_use` + web-search / web-fetch / code-exec results | ✅ | ✅ | ❌ | ❌ |
| `citations` on blocks | ✅ | ✅ | ❌ | ❌ |
| `compaction` / `fallback` blocks | ✅ | ✅ | ❌ | ❌ |
| `cache_control` on text / tool / tool_use / tool_result | ✅ | ✅ | ✅ (content.rs:58, tools.rs:57/118/152) | ✅ |

**Verdict:** ✅ on the text/tool block core with full `cache_control` placement. The gaps are the
**input modalities** (image, document/PDF — the most common everyday miss) and the richer
server-tool / citation / compaction block kinds.

---

## 4. Prompt caching

| Aspect | Python | TS | clauders | Status |
|---|---|---|---|---|
| `cache_control: ephemeral` on content blocks | ✅ | ✅ | `CacheControl::ephemeral` (content.rs:58 + tools.rs) | ✅ |
| 5-minute / 1-hour TTL tiers | ✅ | ✅ | `ephemeral_5m` / `ephemeral_1h` (response.rs:79–84) | ✅ |
| Cache-aware usage counters | ✅ | ✅ | `Usage.cache_creation/read` (response.rs:107–113) | ✅ |
| Top-level auto `cache_control` (auto-place on last block) | ✅ | ✅ | ❌ | 🟡 |

**Verdict:** ✅ parity on explicit per-block caching incl. TTL tiers and usage accounting; only the
convenience top-level auto-placement is missing.

---

## 5. Tools

| Aspect | Python | TS | clauders | Status |
|---|---|---|---|---|
| Custom tool (`name`/`description`/`input_schema`) | ✅ | ✅ | `Tool` (tools.rs:46) | ✅ |
| `strict: true` (schema-guaranteed input) | ✅ | ✅ | `Tool.strict` (tools.rs:62) | ✅ |
| `tool_choice`: auto / any / tool / none | ✅ | ✅ | `ToolChoice` all four (tools.rs:74–88) | ✅ |
| `tool_result` content (text / blocks / `is_error`) | ✅ | ✅ | `ToolResultBlock` / `ToolResultContent` (tools.rs:142–190) | ✅ |
| **Server-side tools** — web_search, web_fetch, code_execution, tool_search | ✅ | ✅ | ❌ | ❌ |
| **Anthropic-defined tools** — bash, text_editor, memory, computer, advisor | ✅ | ✅ | ❌ | ❌ |

**Verdict:** ✅ full parity on **custom** tool use (incl. strict mode and every `tool_choice` variant).
❌ on the entire **server-side / Anthropic-defined** tool tier.

---

## 6. Streaming events

| Event | Python | TS | clauders | Status |
|---|---|---|---|---|
| `message_start` | ✅ | ✅ | `StreamEvent::MessageStart` (streaming.rs:57) | ✅ |
| `content_block_start` / `_delta` / `_stop` | ✅ | ✅ | (streaming.rs:62/69/76) | ✅ |
| `message_delta` / `message_stop` | ✅ | ✅ | (streaming.rs:82/89) | ✅ |
| `ping` / `error` | ✅ | ✅ | (streaming.rs:91/93) | ✅ |
| Deltas: `text` / `thinking` / `signature` / `input_json` | ✅ | ✅ | `ContentDelta` all four (streaming.rs:113–129) | ✅ |
| `redacted_thinking_delta` | ✅ | ✅ | ❌ | 🟡 |

**Verdict:** ✅ parity on the full SSE event set and delta kinds; only `redacted_thinking_delta` is
missing (paired with the absent `redacted_thinking` block).

---

## 7. Response & usage

| Field | Python | TS | clauders | Status |
|---|---|---|---|---|
| `id` / `role` / `model` / `content` / `stop_sequence` | ✅ | ✅ | `Message` (response.rs:27–45) | ✅ |
| `stop_reason`: end_turn / max_tokens / stop_sequence / tool_use / **refusal** | ✅ | ✅ | `StopReason` (response.rs:61–72) | ✅ |
| `stop_reason`: **pause_turn**, **model_context_window_exceeded** | ✅ | ✅ | ❌ | 🟡 |
| `stop_details` (refusal category / explanation) | ✅ | ✅ | ❌ | ❌ |
| `usage`: input / output / cache_creation / cache_read (+ tiers) | ✅ | ✅ | `Usage` (response.rs:99–114) | ✅ |
| `usage`: `server_tool_use`, `iterations` (fallback) | ✅ | ✅ | ❌ | 🟡 |
| `container.id` on response | ✅ | ✅ | ❌ | ❌ |
| Typed parse helper (`messages.parse()` / `parsed_output`) | ✅ (Pydantic) | ✅ (Zod) | ❌ (raw json_schema only) | 🟡 |

**Verdict:** ✅ parity on the core frame taxonomy incl. cache-aware usage and the `refusal` stop reason.
🟡 on the newer diagnostics (`stop_details`, `pause_turn`, `model_context_window_exceeded`,
server-tool usage) and the ergonomic typed-parse helper.

---

## Overall scorecard

| Area | clauders vs official base SDK |
|---|---|
| Messages create / streaming / count-tokens | ✅ parity |
| Batches (full CRUD) / Models API | ✅ parity |
| Custom tools (+ strict, all tool_choice) | ✅ parity |
| Prompt caching (per-block, TTL tiers, usage) | ✅ parity (minus top-level auto-place) |
| JSON-schema structured output | ✅ parity (minus typed parse helper) |
| Streaming event taxonomy | ✅ parity |
| Core response frame + `refusal` | ✅ parity |
| **`thinking` / `effort` / `task_budget`** | ❌ behind — *correctness-critical* |
| **Vision (image) / PDF (document) input** | ❌ behind |
| **Server-side & Anthropic-defined tools** | ❌ behind |
| **Files API** | ❌ behind |
| **Citations / context-management / MCP connector** | ❌ behind |
| Response diagnostics (`stop_details`, `pause_turn`, container, typed parse) | 🟡 leaner |

**One-line summary:** clauders' Messages layer is at solid parity on the *stateless core* — create,
streaming, tools, batches, models, caching, structured output — and trails on **input modalities
(vision/PDF)**, the **`thinking`/`effort` surface** (the one gap that blocks correctly driving current
models), **server-side tools**, and the **Files / citations / context-management / MCP** tier.

---

## Candidate gaps worth closing (ranked by leverage)

Ranked for the parity-first vision, not by official-checklist order:

1. **`thinking` / `effort` / `task_budget`** — *correctness-critical*. Without it, clauders cannot
   correctly drive Opus 4.8/4.7, Sonnet 5, or Fable 5 (they reject sampling params and require adaptive
   thinking). Highest priority.
2. **Vision (image) + PDF/document input** — the most common everyday base-SDK feature; broad demand.
3. **Files API** — prerequisite for document/vision-by-reference workflows and code-execution uploads.
4. **Server-side tools** (web_search, web_fetch, code_execution, tool_search) — the "advanced Messages"
   tier; large surface, add incrementally.
5. **Response diagnostics** — `stop_details`, `pause_turn`, `model_context_window_exceeded`; cheap and
   improves correctness of caller error handling.
6. **Typed parse helper** — ergonomic wrapper over the existing json-schema `output_config`.
7. **Top-level auto `cache_control`**, **citations**, **context management / compaction**, **MCP
   connector** — evaluate as demand appears.

---

## Methodology & caveats

- **clauders side** — read from source at 2026-07-13 (`crates/clauders/src/messages/`:
  `request.rs`, `response.rs`, `content.rs`, `tools.rs`, `streaming.rs`, `token_counting.rs`,
  `structured_outputs.rs`, `resource.rs`, `batches/`; `models/resource.rs`). Authoritative.
- **Official side** — the Anthropic base SDK iterates quickly; exact param keys, tool-type version
  suffixes, and beta headers drift between releases. Re-verify against the live reference before
  treating any single ❌ as a hard commitment.
- Parity marks judge *capability*, not wire/name identity. clauders is idiomatic Rust (builders,
  exhaustive enums, newtypes), so equivalent features carry Rust-shaped names.

## Sources

- Vision & pillar mapping — [`../vision-and-strategy.md`](../vision-and-strategy.md)
- Agent SDK parity (the other pillar) — [`../agent-sdk/feature-parity.md`](../agent-sdk/feature-parity.md)
- Official base SDK reference — Anthropic SDK docs (Python `anthropic`, TypeScript `@anthropic-ai/sdk`).
