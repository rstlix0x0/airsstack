//! Native permission enforcement for the HTTP-API runtimes.
//!
//! A session-scoped [`RuleStore`] (seeded from the caller's allowlist) and an
//! [`evaluate`] step that returns the canonical
//! [`crate::agent::permissions::PermissionDecision`] — the same verdict type a
//! policy returns, with no parallel gate enum. Runtime-agnostic: it lives beside
//! the runtime consumers, not under any single adapter.
#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::error::AgentError;
use crate::agent::permissions::{
    PermissionBehavior, PermissionContext, PermissionDecision, PermissionMode, PermissionPolicy,
    PermissionUpdate,
};

/// Session-scoped, tool-name-keyed permission rules for a native runtime loop.
///
/// Seeded from the caller's allowlist; folds in any rule updates a decision
/// returns. In-memory and session-lived — no glob matching, no disk persistence.
pub(crate) struct RuleStore {
    rules: HashMap<String, PermissionBehavior>,
}

impl RuleStore {
    /// Seed the store from `Options.allowed_tools` — each named tool starts
    /// under an `Allow` rule.
    pub(crate) fn new(seed_allow: &[String]) -> Self {
        let rules = seed_allow
            .iter()
            .map(|tool| (tool.clone(), PermissionBehavior::Allow))
            .collect();
        Self { rules }
    }

    /// Fold decision-returned updates into the store (last write wins per tool).
    pub(crate) fn apply(&mut self, updates: &[PermissionUpdate]) {
        for update in updates {
            self.rules.insert(update.tool.clone(), update.behavior);
        }
    }

    /// The standing rule for `tool`, if any.
    pub(crate) fn lookup(&self, tool: &str) -> Option<PermissionBehavior> {
        self.rules.get(tool).copied()
    }
}

/// Decide whether `tool` may run, honoring bypass, session rules, `DontAsk`,
/// and (otherwise) the caller's policy. Returns the canonical
/// [`PermissionDecision`]; any policy-returned `updated_permissions` are folded
/// into `store` before returning. First match wins.
///
/// # Errors
/// Propagates an [`AgentError`] from the policy's `can_use_tool`.
pub(crate) async fn evaluate(
    mode: PermissionMode,
    store: &mut RuleStore,
    policy: Option<&Arc<dyn PermissionPolicy>>,
    tool: &str,
    input: &serde_json::Value,
    ctx: PermissionContext,
) -> Result<PermissionDecision, AgentError> {
    if mode == PermissionMode::BypassPermissions {
        return Ok(PermissionDecision::allow());
    }
    match store.lookup(tool) {
        Some(PermissionBehavior::Allow) => return Ok(PermissionDecision::allow()),
        Some(PermissionBehavior::Deny) => {
            return Ok(PermissionDecision::deny("denied by session rule"));
        }
        None => {}
    }
    if mode == PermissionMode::DontAsk {
        return Ok(PermissionDecision::deny(
            "tool not pre-approved under dontAsk",
        ));
    }
    match policy {
        Some(policy) => {
            let decision = policy.can_use_tool(tool, input, ctx).await?;
            store.apply(decision.updated_permissions());
            Ok(decision)
        }
        None => Ok(PermissionDecision::allow()),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::{RuleStore, evaluate};
    use crate::agent::error::AgentError;
    use crate::agent::permissions::{
        PermissionBehavior, PermissionContext, PermissionDecision, PermissionMode,
        PermissionPolicy, PermissionScope, PermissionUpdate,
    };

    #[test]
    fn seed_allow_marks_named_tools_allowed() {
        let store = RuleStore::new(&["Bash".to_string(), "Read".to_string()]);
        assert_eq!(store.lookup("Bash"), Some(PermissionBehavior::Allow));
        assert_eq!(store.lookup("Read"), Some(PermissionBehavior::Allow));
        assert_eq!(store.lookup("Write"), None);
    }

    #[test]
    fn apply_folds_updates_last_write_wins() {
        let mut store = RuleStore::new(&[]);
        store.apply(&[PermissionUpdate {
            behavior: PermissionBehavior::Allow,
            tool: "Bash".to_string(),
            scope: PermissionScope::Session,
        }]);
        assert_eq!(store.lookup("Bash"), Some(PermissionBehavior::Allow));
        store.apply(&[PermissionUpdate {
            behavior: PermissionBehavior::Deny,
            tool: "Bash".to_string(),
            scope: PermissionScope::Session,
        }]);
        assert_eq!(store.lookup("Bash"), Some(PermissionBehavior::Deny));
    }

    struct SpyPolicy {
        called: Arc<AtomicBool>,
        decision: fn() -> PermissionDecision,
    }

    #[async_trait::async_trait]
    impl PermissionPolicy for SpyPolicy {
        async fn can_use_tool(
            &self,
            _tool: &str,
            _input: &serde_json::Value,
            _ctx: PermissionContext,
        ) -> Result<PermissionDecision, AgentError> {
            self.called.store(true, Ordering::SeqCst);
            Ok((self.decision)())
        }
    }

    fn allow_decision() -> PermissionDecision {
        PermissionDecision::allow()
    }

    #[tokio::test]
    async fn bypass_allows_without_consulting_policy() {
        let called = Arc::new(AtomicBool::new(false));
        let policy: Arc<dyn PermissionPolicy> = Arc::new(SpyPolicy {
            called: called.clone(),
            decision: allow_decision,
        });
        let mut store = RuleStore::new(&[]);
        let decision = evaluate(
            PermissionMode::BypassPermissions,
            &mut store,
            Some(&policy),
            "Bash",
            &serde_json::json!({}),
            PermissionContext::default(),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
        assert!(
            !called.load(Ordering::SeqCst),
            "bypass must not call the policy"
        );
    }

    #[tokio::test]
    async fn session_rule_short_circuits() {
        let mut store = RuleStore::new(&["Bash".to_string()]);
        let decision = evaluate(
            PermissionMode::DontAsk,
            &mut store,
            None,
            "Bash",
            &serde_json::json!({}),
            PermissionContext::default(),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
    }

    #[tokio::test]
    async fn dont_ask_denies_unruled_tool_without_policy() {
        let called = Arc::new(AtomicBool::new(false));
        let policy: Arc<dyn PermissionPolicy> = Arc::new(SpyPolicy {
            called: called.clone(),
            decision: allow_decision,
        });
        let mut store = RuleStore::new(&[]);
        let decision = evaluate(
            PermissionMode::DontAsk,
            &mut store,
            Some(&policy),
            "Bash",
            &serde_json::json!({}),
            PermissionContext::default(),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
        assert!(
            !called.load(Ordering::SeqCst),
            "DontAsk must not consult the policy"
        );
    }

    #[tokio::test]
    async fn default_consults_policy_and_applies_updates() {
        fn allow_then_rule() -> PermissionDecision {
            PermissionDecision::Allow {
                updated_input: None,
                updated_permissions: vec![PermissionUpdate {
                    behavior: PermissionBehavior::Deny,
                    tool: "Bash".to_string(),
                    scope: PermissionScope::Session,
                }],
            }
        }
        let called = Arc::new(AtomicBool::new(false));
        let policy: Arc<dyn PermissionPolicy> = Arc::new(SpyPolicy {
            called: called.clone(),
            decision: allow_then_rule,
        });
        let mut store = RuleStore::new(&[]);
        let decision = evaluate(
            PermissionMode::Default,
            &mut store,
            Some(&policy),
            "Bash",
            &serde_json::json!({}),
            PermissionContext::default(),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
        assert!(called.load(Ordering::SeqCst));
        // The returned update landed in the store: a later lookup reflects it.
        assert_eq!(store.lookup("Bash"), Some(PermissionBehavior::Deny));
    }

    #[tokio::test]
    async fn default_without_policy_allows() {
        let mut store = RuleStore::new(&[]);
        let decision = evaluate(
            PermissionMode::Default,
            &mut store,
            None,
            "Bash",
            &serde_json::json!({}),
            PermissionContext::default(),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
    }
}
