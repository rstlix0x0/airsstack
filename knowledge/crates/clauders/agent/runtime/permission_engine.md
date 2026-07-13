---
type: Rust Module
title: clauders::agent::runtime::permission_engine
description: Native permission enforcement for the HTTP-API runtimes — a session-scoped, tool-name-keyed RuleStore and an evaluate() step (bypass -> session rule -> DontAsk -> policy/allow) that returns the canonical PermissionDecision.
tags: [rust, sdk, agent, runtime, permissions, security]
timestamp: 2026-07-11T00:00:00Z
resource: crates/clauders/src/agent/runtime/permission_engine.rs
---

Lives beside the runtime consumers (`pub(crate) mod permission_engine;`
in [`agent::runtime`](/crates/clauders/agent/runtime/overview.md)), not
under any single adapter — "runtime-agnostic" per its own doc comment,
though today [`ApiRuntime`](/crates/clauders/agent/runtime/api/runtime.md)
is its only caller (the CLI-subprocess path instead has the binary itself
gate tool calls and only consults
[`Dispatcher::permission_outcome`](/crates/clauders/agent/cli/dispatch.md)
for the binary's own `can_use_tool` control request; the OpenRouter
runtime does not wire it in either). Returns the same
[`PermissionDecision`](/crates/clauders/agent/permissions.md) type a
policy returns — no parallel gate enum.

# Schema

```rust
pub(crate) struct RuleStore {
    rules: HashMap<String, PermissionBehavior>,
}

impl RuleStore {
    pub(crate) fn new(seed_allow: &[String]) -> Self;               // Options.allowed_tools -> Allow
    pub(crate) fn apply(&mut self, updates: &[PermissionUpdate]);   // last write wins per tool
    pub(crate) fn lookup(&self, tool: &str) -> Option<PermissionBehavior>;
}

pub(crate) async fn evaluate(
    mode: PermissionMode,
    store: &mut RuleStore,
    policy: Option<&Arc<dyn PermissionPolicy>>,
    tool: &str,
    input: &serde_json::Value,
    ctx: PermissionContext,
) -> Result<PermissionDecision, AgentError>;
```

`RuleStore` is in-memory and session-lived: no glob matching, no disk
persistence. `new` seeds it from the caller's `Options.allowed_tools` —
every named tool starts under an `Allow` rule. `apply` folds a decision's
[`updated_permissions`](/crates/clauders/agent/permissions/update.md)
into the store, last write wins per tool name; `scope` is ignored (every
update is treated as session-scoped, matching the store's own lifetime).

`evaluate` is a first-match-wins chain, in order:

1. **Bypass** — `mode == PermissionMode::BypassPermissions` allows
   unconditionally, without consulting the store or the policy.
2. **Session rule** — an existing `store.lookup(tool)` entry short-circuits:
   `Allow` allows, `Deny` denies with `"denied by session rule"` (a
   non-interrupt deny; `PermissionDecision::deny`).
3. **`DontAsk`** — with no session rule and `mode == PermissionMode::DontAsk`,
   denies with `"tool not pre-approved under dontAsk"`, never consulting
   the policy — the mode's contract is "deny anything not pre-approved,
   without prompting."
4. **Policy or default-allow** — otherwise, delegates to
   `policy.can_use_tool(tool, input, ctx)` when a
   [`PermissionPolicy`](/crates/clauders/agent/permissions.md) is
   registered, folding the returned decision's `updated_permissions` into
   `store` before returning it; with no registered policy, allows
   unconditionally (`PermissionDecision::allow()`).

A policy's `Err` propagates to the caller as an `AgentError` — `evaluate`
does not swallow policy failures; it is
[`ApiRuntime::run_tools`](/crates/clauders/agent/runtime/api/runtime.md)
that turns that error into a model-visible tool-result error rather than
a session failure.

# Examples

```rust,no_run
# async fn example() -> Result<(), clauders::agent::AgentError> {
// pub(crate)-only: exercised from within the crate, e.g. ApiRuntime::run_tools.
# Ok(())
# }
```

Related: [permissions module (PermissionMode/PermissionDecision/PermissionPolicy)](/crates/clauders/agent/permissions.md),
[PermissionUpdate cluster](/crates/clauders/agent/permissions/update.md),
[ApiRuntime::run_tools/drive](/crates/clauders/agent/runtime/api/runtime.md)
(the sole current caller — owns one `RuleStore` per `drive()` invocation
and calls `evaluate` once per tool-use block),
[runtime layer overview](/crates/clauders/agent/runtime/overview.md).

# Citations

1. `crates/clauders/src/agent/runtime/permission_engine.rs`
2. `crates/clauders/src/agent/runtime/mod.rs`
