# Deliberate divergences

Every place `clauders` knowingly behaves differently from the official Python and TypeScript SDKs,
with the reasoning. This file exists so the next reader does not "fix" a decision.

A divergence is not a gap. A gap is something the official SDKs do that clauders does not; those live
in the parity documents ([Messages](messages-sdk/feature-parity.md),
[Agent](agent-sdk/feature-parity.md)). A divergence is something clauders does *differently*, on
purpose, having looked at what the official SDKs do first.

Two of these exist because the official SDKs disagree with **each other**. When Python raises and
TypeScript silently continues, there is no "the official behaviour" to copy — a choice has to be
made, and it should be made once and written down.

## Streaming accumulation

The Messages API's SSE accumulator is where most of the divergence lives, because it is where the
three official SDKs differ most from one another.

### `content_block_start` asserts the index

clauders requires `index == content.len()` and returns `Error::Stream` otherwise
(`src/messages/accumulator.rs:197-202`). Python never reads `index` at all and blind-appends —
its source carries a literal `# TODO: check index`. TypeScript also blind-appends. The Go SDK
asserts, and clauders follows Go.

Why: the alternative clauders used to implement was padding the content vector with empty text
blocks, which leaves fabricated blocks indistinguishable from ones the model actually produced. Once
padding was ruled out, the choice was between erroring and appending-regardless. Appending regardless
misaligns every subsequent index-addressed delta with no signal at all. Against a conforming server
the three behaviours are identical; they differ only on a malformed stream, where clauders reports
and the other two corrupt.

**Porting note:** code moving from Python or TypeScript will see an `Error::Stream` where those SDKs
returned a misaligned message.

### An out-of-range delta or stop index is a silent no-op

A delta addressing a block that does not exist is ignored (`src/messages/accumulator.rs:236-238`).
TypeScript does the same — `.at()` returns `undefined` and nothing happens. Python raises
`IndexError`. Go returns an error.

Why: the server is the authority on which block/delta pairings are valid. A mismatch is far more
likely to mean this release does not model the block than that the response is corrupt, and taking
down a whole stream over an unmodelled block is the failure mode forward compatibility exists to
prevent. Note this is the *opposite* choice from the row above — deliberately. A misaddressed
`content_block_start` corrupts everything after it; a misaddressed delta loses one fragment.

### A truncated stream returns the partial message

A stream that ends without `message_stop` yields `Ok(partial)` with `stop_reason: None`. Only a
stream that ends before `message_start` is an error
(`src/messages/accumulator.rs:319-322` — `"stream ended before message_start event"`). This follows
Python and Go, both of which have no completeness check at all. TypeScript throws
`AnthropicError('stream ended without producing a Message with role=assistant')`.

A related consequence: a tool-argument buffer never closed by `content_block_stop` is never parsed,
so `input` keeps its `content_block_start` value. Python's eager per-delta parse would salvage
complete key/value pairs from it. clauders is strictly less salvaging here, reachable only on an
already-broken stream.

### A duplicate `message_start` replaces the snapshot

Both the snapshot and the JSON buffers are replaced wholesale
(`src/messages/accumulator.rs:113-117`), so nothing accumulated under a prior `message_start`
survives. All three official SDKs differ: TypeScript throws, Go replaces, and Python ignores the
second event entirely — appending the second message's content blocks onto the first message's list,
interleaving two messages into one snapshot with no error and no id check.

clauders follows Go. Python's interleaving is the one outcome worth ruling out outright: it produces
a plausible-looking `Message` that is silently wrong.

### `message_delta` writes stop fields only when present

`stop_reason` and `stop_sequence` are written only when the delta actually carries them
(`src/messages/accumulator.rs:222-229`). Python, TypeScript and Go all assign unconditionally,
including overwriting an already-resolved value with `null`.

The guard stops a stray later `message_delta` from clobbering a resolved `stop_reason`. It rests on
an assumption stated in the source comment: that the terminal delta is the one carrying these fields.
A server sending a later empty `message_delta` after the resolving one is **not** something we have
verified either way — neither official SDK guards, because neither needed to.

## Type modelling

### Two content-block unions, not one

The official SDKs use one loose union for both directions — a 12-member `ContentBlock` for responses
and a 17-member `ContentBlockParam` superset for requests, with nothing stopping you passing the
wrong one. clauders splits them (`src/messages/content/block.rs`, `src/messages/content/param.rs`)
and joins them with a fallible `TryFrom<ContentBlock> for ContentBlockParam`
(`src/messages/content/param.rs:81`).

Echoing a response block back into the next request is the common multi-turn pattern, so the
conversion has to exist — but a `server_tool_use` or `container_upload` block cannot be sent, and the
conversion returns `UnsendableBlock` naming the offending wire `type`
(`src/messages/content/param.rs:89-110`). The `Vec` form, `try_from_response`
(`src/messages/content/param.rs:143`), is all-or-nothing: it fails on the first unsendable block
rather than silently dropping it.

The result is that sending a response-only block is a compile error or a named `Err`, never a
runtime "unserializable request block".

**Porting note:** clauders' request union has six members
(`src/messages/content/param.rs:27` — Text, Thinking, ToolUse, ToolResult, Image, Document) where the
official one has 17. That part is a gap, tracked in the parity doc, not a divergence. The *split* is
the divergence.

### `Unknown` arms retain the payload

Every enum decoded from a server response carries an unknown arm, and that arm carries the raw JSON
rather than merely recording that something was unrecognised. Anthropic's versioning policy states
new content-block and event types may be added within `anthropic-version: 2023-06-01`, so failing the
decode is not an option.

Retention is the part worth defending. Python retains via `__pydantic_extra__`, TypeScript by not
validating at all, Go via `RawJSON()`. A presence-only arm would make clauders the only client in the
family that throws away data the server sent.

`ContentBlock::Unknown` is the one asymmetric case: it alone also derives `Serialize`, so it carries
`#[serde(untagged, skip_serializing)]` (`src/messages/content/block.rs:69`). An unknown block can be
inspected but not echoed back — round-tripping a block this release does not understand is worse than
refusing.

### `ModelCapabilities.context_management` is a map

Both official SDKs hardcode each dated context-management strategy as its own named optional field —
`clear_thinking_20251015?`, `clear_tool_uses_20250919?`, `compact_20260112?`. clauders models
`supported: bool` plus `#[serde(flatten)] strategies: BTreeMap<String, CapabilitySupport>`
(`src/models/capabilities.rs:39-49`), so every dated key the server sends lands in the map under its
wire key.

No data is lost either way, and a newly dated strategy needs no clauders change to be represented.

**Porting note:** `caps.context_management.clear_thinking_20251015` has no equivalent field. Use
`caps.context_management.strategies.get("clear_thinking_20251015")`.

## Agent SDK

### Elicitation requires `mcp_server_name`

`sdk.d.ts` declares `SDKControlElicitationRequest.mcp_server_name` as `string`, not `string?`. The
shipped `sdk.mjs` runtime does not enforce that: its elicitation branch reads the field off the raw
request object unvalidated and calls `onElicitation` regardless, so a frame missing it reaches the
callback with `serverName: undefined`.

clauders requires it structurally — `mcp_server_name: String` at
`src/agent/protocol/frames.rs:242`, the only field in that variant without `#[serde(default)]`. A
request missing it is rescued to `Malformed` and answered with an error rather than reaching the
policy.

clauders follows the type declaration rather than the looser runtime. Forwarding an undefined server
name into a policy that expects a `String` has no good outcome in Rust.

### `SessionArchive::messages` returns the whole transcript

The official `getSessionMessages` reconstructs a single active thread: it indexes entries by uuid,
finds the terminals, walks `parentUuid` back with a cycle guard, and returns one linear conversation.
Branches are resolved for you and discarded.

clauders returns the full flat transcript instead — every user, assistant and (on request) system
entry, deduped by uuid, in file order, each carrying its own `parent_uuid`
(`src/agent/sessions/archive.rs:120-127`, `src/agent/sessions/message.rs:19-31`). Branch
reconstruction is left to the caller.

The reasoning is that the official reconstruction is lossy and its selection rules are not
documented anywhere stable — it prefers non-sidechain, non-meta, non-team leaves and breaks ties on
file index. A caller who wants the active thread can apply those rules to the flat list; a caller who
wants the branches cannot recover them from the reconstructed one.

**Porting note:** this is the one session function where output shape differs, not just naming.
Expect more entries than the official SDK returns.

### Process-hygiene options with no official counterpart

Three `Options` fields exist in clauders and in neither official SDK:

| Field | What it does |
|---|---|
| `require_min_version` | promotes a too-old `claude` binary from a warning to a hard error |
| `shutdown_grace` | the graceful-exit window before the supervisor forces a kill |
| `control_request_timeout` | bounds how long a control request waits for its correlated response |

These are not parity features and are not claimed as such. They exist because a Rust caller embedding
a subprocess in a long-lived service needs bounded teardown and bounded waits, and because a silently
too-old binary produces confusing failures much later.

### Two `Options` fields are inert

`Options::max_tokens` (`src/agent/options.rs:54`) and `Options::user`
(`src/agent/options.rs:120-122`) can be set and are carried on the struct, but nothing consumes them
on the CLI runtime. `user` documents this in its own doc comment — the binary exposes no matching
flag. `max_tokens` describes itself as "forwarded to the Messages API", but a search of `src/agent/`
outside `options.rs` finds no reader: it is neither lowered to argv nor sent in the handshake.

They are kept for shape compatibility with the official `Options`. Setting either changes nothing
about the spawned process, which is worth knowing before you debug why it had no effect.

## Reading this list

The tests pin these behaviours. If you change one, a test goes red — that is the intended signal, not
an obstacle. Change the decision here first, then the code, then the test.
