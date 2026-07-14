# clauders — Vision & Strategy

**Status:** Active — adopted 2026-07-13. **Supersedes** the token-efficiency / mixed-routing
thesis previously encoded in the repo `README.md`, `CLAUDE.md` ("Project intent" / "Scope
discipline"), and the `agent-sdk/` roadmap docs. Those documents must be reconciled to this one (see
§10). This is the authoritative statement of what `clauders` is for.

---

## 1. Vision

**`clauders` is a faithful Rust client for Anthropic's official SDK surface — nothing more, until the
core is complete.** The single objective is **100% feature parity and behavioral compatibility with
Anthropic's official SDKs**, so a Rust caller gets the same capabilities a Python or TypeScript caller
gets, with idiomatic Rust ergonomics.

Everything that is not required for that parity is **removed now** and **re-introduced only after** the
core is stable and at full parity. We are deliberately trading breadth of ambition for depth of
correctness on a well-defined target.

### Why the pivot

The prior vision (suppress tokens via native multi-provider execution and mixed routing to cheaper
models) produced a large **superset** of the official SDKs: bespoke native runtimes, an extension
framework, an evals harness, and an orchestration pool. That superset:

- had **no official counterpart to measure against**, so "done" was undefined and drift was constant;
- coupled unrelated subsystems (the native `ApiRuntime` was the substrate under structured output,
  permission enforcement, subagents, and sessions), inflating the change surface for every feature;
- carried the feature-combinatorics / dead-code maintenance cost the team had already flagged.

A **fixed, external, versioned parity target** (the official SDKs) replaces an open-ended internal
one. Parity is a checklist we can complete; a novel framework is not.

---

## 2. The three pillars

`clauders` targets exactly **three** official Anthropic products, each a distinct parity surface with
a distinct clauders home.

| # | Official product | What it is | clauders home | State |
|---|---|---|---|---|
| **1. Messages API** | Anthropic base SDK (`anthropic` / `@anthropic-ai/sdk`) | Raw `POST /v1/messages` client — messages, streaming, tools, thinking, vision, batches, files, models, token counting | `crates/clauders/src/messages/` + `models/` + `client/` | Strong core; gaps in §7 |
| **2. Agent SDK** | `claude-agent-sdk` / `@anthropic-ai/claude-agent-sdk` | A **thin client that drives the `claude` Code CLI binary as a subprocess** — query/client, tools, hooks, permissions, subagents, sessions, system prompt | `crates/clauders/src/agent/` (`CliRuntime` + `MockRuntime` only) | Core landed; gaps in §7 |
| **3. Managed Agents** | Managed Agents beta API (`/v1/agents`, `/v1/sessions`, `/v1/environments`, …) | Server-managed stateful agents: Anthropic runs the loop and hosts the per-session container; the client drives sessions, streams events, handles tool results | **New** — not yet built | Net-new |

**Key architectural fact that makes this clean:** the official Agent SDK does **not** implement a
native Messages loop — it only drives the CLI subprocess. So Pillar 2 is `CliRuntime` alone. Once the
bespoke native runtimes are removed (§5), Pillar 1 (`messages::`) and Pillar 2 (`agent::CliRuntime`)
become **fully independent** — the native `ApiRuntime` was the only thing that coupled them. Pillar 3
is a fresh client against a server API and depends on neither.

---

## 3. Scope discipline

### In scope (the parity target)

- **Messages API:** everything the base SDK exposes on the `messages`, `models`, `batches`, and
  `files` resources — including **vision (image)**, **PDF/document** input, the **`thinking` /
  `effort` / `task_budget`** surface, **server-side tools** (web search/fetch, code execution, tool
  search, bash, text editor, memory, computer, advisor), **citations**, **context management /
  compaction**, the **MCP connector**, **prompt caching** (as an SDK-parity feature, not a bespoke
  policy), **structured outputs** with a typed parse helper, and the full **streaming** event set.
- **Agent SDK:** everything the official Agent SDK exposes by driving the CLI — streaming input,
  live-control ops (interrupt, set-model, set-permission-mode, MCP status), the full `Options`
  surface, hooks, permission modes (incl. `auto` as a CLI passthrough), programmatic subagents
  (`AgentDefinition`), sessions (continue/resume/fork/list), setting-sources, in-process MCP tools.
- **Managed Agents:** the agent/session/environment lifecycle, event streaming (SSE), tool-result
  round-trips, resources (files/repos/memory stores), vaults, outcomes, deployments — to the parity
  bar the official SDKs set for `client.beta.{agents,sessions,environments,…}`.

### Out of scope (removed now; see §5)

Native Messages-loop agent execution, non-Anthropic model execution, cross-provider routing, an
in-SDK middleware/decorator framework, an evals harness, and a multi-process orchestration pool.
**None of these exist in the official SDKs**, so none belong in a parity-first `clauders`.

### Non-goals (for this phase)

- Token-efficiency levers that require a native loop (per-subtask model downgrade, programmable
  prompt-cache policy, context pruning). These were the old north star; they return only under §8.
- Any capability whose only justification is "the official SDK doesn't have it, but it would be nice."
  That is precisely the complexity we are removing.

---

## 4. Design principles

1. **Parity is the spec.** For any feature, the official SDK's behavior — not our judgment — defines
   correct. When in doubt, verify against the live SDK reference, not memory.
2. **Idiomatic Rust, equivalent capability.** We match *capability*, not wire/name identity: builders,
   exhaustive enums, trait objects, `Option` over magic values. A feature carries a Rust-shaped name.
3. **Simplicity now, extensibility later.** Prefer the smallest design that reaches parity. Do not
   pre-build seams for the removed subsystems; re-introduce them under §8 when they earn their place.
4. **Three independent pillars.** Keep Messages / Agent / Managed Agents decoupled. No pillar should
   depend on another's internals; shared value types live in a common `types` module.
5. **Featureless, zero-warning, test-first.** The existing Rust Definition-of-Done and guidelines
   still bind every change.

---

## 5. Removal plan

Removed **now** (docs-first; code removal is a follow-up execution phase). Each item lists what parity
surface, if any, it was serving — so we know exactly what is lost.

| Removed | Location | Was serving | Parity impact |
|---|---|---|---|
| **`ApiRuntime`** (native `POST /v1/messages` agentic loop) | `agent/runtime/api/` | 🟣 clauders-only native execution | None — no official Agent SDK counterpart |
| **`OpenRouterRuntime`** | `agent/runtime/openrouter/` | 🟣 non-Anthropic models | None — off-target (non-Anthropic) |
| **`RoutingRuntime`** (+ `Classifier`, `ModelCard`, `RoutingSummary`) | `agent/runtime/routing/` | 🟣 cross-provider routing | None — off-target |
| **Middleware framework** (`Layer`, `Stack`, `Trace`, `Retry`, `TokenMeter`, `Tap`) | `agent/middleware/` | 🟣 extension tier | None — no official counterpart |
| **Evals harness** (`Case`, `EvalSuite`, `Scorer`, `Grader`, `Judge`, `Report`) | `agent/evals/` | 🟣 framework ambition | None |
| **Orchestration pool** (`Pool`, `Limiter`, `SemaphoreLimiter`) | `agent/orchestration/` | 🟣 bounded-concurrency fan-out | None |
| **Native permission engine** (`RuleStore`, `evaluate`) | `agent/runtime/permission_engine.rs` | WS C **native** enforcement | CLI passthrough survives (see below) |
| **Auto-permission judge** (`AutoPermissionPolicy`, `RuntimeJudge`, `JudgeRubric`) | `agent/runtime/permission_judge.rs` | WS D **native** model-judge | `PermissionMode::Auto` may survive as a CLI passthrough value |
| **Native subagent loop** (Agent tool, nested `drive`) | `agent/subagents/` native path | WS E2 **native** downgrade | CLI `--agents` passthrough survives |
| **Native session store** (`SessionStore`, `SessionSink`, `resolve_session`) | native path on `ApiRuntime` | WS F2 **native** history | CLI `--continue/--resume/--fork` passthrough survives |

**Kept:**

- `messages::`, `models::`, `client::`, `batches` — **Pillar 1**, grows to full parity (§7).
- `agent::CliRuntime` + `MockRuntime` (the test seam) — **Pillar 2**.
- `agent::mcp` (in-process MCP tools — `SdkMcpServer`, `tool()`) — this **is** official Agent SDK
  surface (`create_sdk_mcp_server`); keep.
- CLI-passthrough halves of WS A–F: `SystemPrompt::Preset`→flags, `Options::output_format`→CLI,
  `PermissionMode::DontAsk`/deny-interrupt/`updated_permissions`→CLI, `Options::agents`→`--agents`,
  `SessionControl`→`--continue/--resume/--fork`. These **are** Agent-SDK parity.
- Core CLI plumbing: `agent/process`, `agent/protocol`, `agent/permissions` (types), `agent/hooks`,
  `agent/types`, `agent/capabilities`.

**The `Runtime` trait simplifies.** With `RoutingRuntime` gone, the object-safety constraint it forced
(it held `HashMap<ModelId, Arc<dyn Runtime>>`) no longer binds. Only `CliRuntime` + `MockRuntime`
remain; keep the trait solely as the mock seam, or fold it away if the mock can be expressed without
it. This is a decision for the removal-execution plan.

---

## 6. What this costs — consciously accepted trade-offs

Recorded so the decision is not silently reversed later.

- **The token-efficiency thesis is shelved.** The official SDKs are Claude-only CLI-drivers (Agent
  SDK) and a raw Messages client (base SDK); neither can route to cheaper non-Claude models or expose
  a programmable cache policy. Adopting parity-first means those levers (per-subtask downgrade,
  context pruning, mixed routing) are **out** until §8.
- **A large slice of just-landed native work is removed.** The native halves of WS B–F (structured
  output on `ApiRuntime`, native permission enforcement, the auto-permission judge, the native
  subagent loop, the native session store) — much of the current unpushed commit stack — are deleted.
  Their CLI-passthrough halves survive as Agent-SDK parity. This is sunk cost; it does not justify
  keeping the native runtime.
- **We give up a genuine differentiator.** `ApiRuntime` was built on the stable public Messages API
  (a versioned contract), whereas the surviving `CliRuntime` chases the less-stable `claude` binary.
  We are accepting a more volatile parity target in exchange for a smaller, well-defined one.

The bet: a correct, complete, trustworthy parity client is worth more right now than a broad,
half-defined superset — and the removed capabilities return on a firmer base once the core is stable.

---

## 7. Parity backlog (per pillar)

High-level; the detailed gap tables live in `agent-sdk/feature-parity.md` (Agent SDK) and should be
mirrored by a new `messages-api/feature-parity.md` (Messages API). Ranked roughly by leverage.

### Pillar 1 — Messages API (base SDK)

Strong core already: create, streaming, tools (+ `strict`, all four `tool_choice`), batches (full
CRUD), token counting, models, prompt caching (5m/1h tiers), JSON-schema structured output, beta
headers. **Gaps:**

1. **`thinking` / `effort` / `task_budget`** — *correctness-critical*: current-gen models (Opus
   4.8/4.7, Sonnet 5, Fable 5) require adaptive thinking and **reject** `temperature`/`top_p`/`top_k`.
   Without a `thinking` surface, clauders cannot correctly drive the models it targets.
2. **Vision (image)** and **PDF/document** input blocks — the most common everyday gap.
3. **Server-side tools** — web search/fetch, code execution, tool search, bash, text editor, memory,
   computer, advisor.
4. **Files API** (`/v1/files`) — upload/reference/download.
5. **Citations**, **context management / compaction**, **MCP connector**.
6. **Response diagnostics** — `stop_details`, `pause_turn`, `model_context_window_exceeded`,
   `server_tool_use` usage; typed `messages.parse()` helper.

### Pillar 2 — Agent SDK (CLI runtime)

Core at parity (query/client, tools, hooks, permissions core, system prompt, messages). **Gaps:**

1. **Streaming input** (`AsyncIterable` prompt) — now *much simpler*: only `CliRuntime` (NDJSON to
   stdin) + `MockRuntime`, no 5-adapter problem. (The paused WS G brainstorm collapses to this.)
2. **`Options` breadth** — the ~25 missing knobs (`fallback_model`, `add_dirs`, `setting_sources`,
   `settings`, `include_partial_messages`, `max_buffer_size`, `stderr`, sandbox/plugins/skills/betas
   passthroughs, etc.).
3. **Live-control tail** — reconnect/toggle/set MCP live, warm start, `reinitialize`,
   `supportedCommands/Models/Agents`, `rewindFiles`, `stopTask`.
4. **Setting sources** (filesystem `CLAUDE.md`/settings) — evaluate; may stay a deliberate omission.
5. **Hook event edges** — `SessionStart`/`SessionEnd`, `mcp_elicitation`.
6. **MCP elicitation** (needs streaming input first).

### Pillar 3 — Managed Agents (new)

Everything is net-new. Build order suggestion: agent/environment/session CRUD → event streaming
(SSE) → tool-result round-trips (`user.custom_tool_result`, `tool_confirmation`) → resources
(files/repos/memory stores) → vaults → outcomes → deployments. Model against the official
`client.beta.{agents,sessions,environments,vaults,memory_stores,deployments}` surface.

---

## 8. Re-introduction criteria (when the removed complexity may return)

A removed subsystem may be proposed for re-introduction **only when all** hold:

1. All three pillars are at (or demonstrably near) **100% parity** and stable.
2. There is a **concrete, in-hand use case** — not a hypothetical.
3. It is designed as an **additive layer** that does not fragment or fork the parity surface.

The native Messages loop, mixed routing, and token-efficiency levers are the most likely returnees —
they serve the original mission and would sit cleanly on top of a complete Pillar-1 Messages client.

---

## 9. Decisions (resolved 2026-07-13)

1. **`openrouter-rs` crate fate → KEEP the crate, SEVER the integration.** Remove `OpenRouterRuntime`
   and `RoutingRuntime` from `clauders`; keep `openrouter-rs` as an independent standalone SDK crate
   (it is not clauders complexity, and deleting a working SDK is a separate call).
2. **CLI-passthrough halves of WS A–F → KEEP.** They are already landed and *are* Agent-SDK parity.
   Delete only the native halves (built on `ApiRuntime`).
3. **`Runtime` trait — retain as mock seam, or remove?** With one real impl + one mock, the trait may
   be redundant. *Still open — decide during removal execution.*
4. **Doc reconciliation (§10) → DONE (2026-07-13).** `README.md`, `CLAUDE.md`, and the agent-sdk
   roadmap / phase-4 / feature-parity docs were reconciled to this vision after the removal landed;
   `docs/messages-api/feature-parity.md` already existed. This file remains the source of truth.

---

## 10. Documents to reconcile

This vision conflicts with existing repo docs. Until reconciled, **this file wins.**

- ✅ DONE — `README.md` — rewritten to the three-pillar parity vision.
- ✅ DONE — `CLAUDE.md` — "Project intent" and "Scope discipline" updated to the parity vision and the
  §9.1 decision.
- ✅ DONE — `docs/agent-sdk-roadmap.md`, `docs/agent-sdk/phase-4-cli-parity.md`,
  `docs/agent-sdk/feature-parity.md` — rescoped to Pillar 2; the 🟣 rows re-legended as removed.
- ✅ ALREADY PRESENT — `docs/messages-api/feature-parity.md` — the Pillar-1 gap table exists and is
  already Pillar-1 framed.

---

## Sources

- Official Agent SDK parity gap analysis — [`agent-sdk/feature-parity.md`](./agent-sdk/feature-parity.md)
- Official base SDK (Messages API) surface — Anthropic SDK reference (Python/TypeScript), cross-checked
  2026-07-13.
- Official Managed Agents surface — Anthropic Managed Agents reference (`/v1/agents`, `/v1/sessions`,
  `/v1/environments`, events, vaults, outcomes, deployments).
