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
    JudgeRequest, PermissionBehavior, PermissionDecision, PermissionJudge, PermissionMode,
    PermissionPolicy, PermissionUpdate,
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

/// Decide whether the reviewed tool call may run, honoring bypass, session
/// rules, `DontAsk`, and (otherwise) the caller's policy. Returns the
/// canonical [`PermissionDecision`]; any policy-returned `updated_permissions`
/// are folded into `store` before returning. First match wins.
///
/// # Errors
/// Propagates an [`AgentError`] from the policy's `can_use_tool`.
pub(crate) async fn evaluate(
    mode: PermissionMode,
    store: &mut RuleStore,
    judge: Option<&Arc<dyn PermissionJudge>>,
    policy: Option<&Arc<dyn PermissionPolicy>>,
    req: &JudgeRequest<'_>,
) -> Result<PermissionDecision, AgentError> {
    if mode == PermissionMode::BypassPermissions {
        return Ok(PermissionDecision::allow());
    }
    match store.lookup(req.tool) {
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
    if mode == PermissionMode::Auto {
        let Some(judge) = judge else {
            return Ok(PermissionDecision::deny(
                "Auto mode has no judge configured",
            ));
        };
        let verdict = judge.judge(req).await?;
        if matches!(verdict, PermissionDecision::Deny { .. }) {
            return Ok(verdict);
        }
        // Judge allowed; a registered policy may still veto (tighten only).
        return match policy {
            Some(policy) => {
                let decision = policy
                    .can_use_tool(req.tool, req.input, req.ctx.clone())
                    .await?;
                store.apply(decision.updated_permissions());
                Ok(decision)
            }
            None => Ok(verdict),
        };
    }
    match policy {
        Some(policy) => {
            let decision = policy
                .can_use_tool(req.tool, req.input, req.ctx.clone())
                .await?;
            store.apply(decision.updated_permissions());
            Ok(decision)
        }
        None => Ok(PermissionDecision::allow()),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::{RuleStore, evaluate};
    use crate::agent::error::AgentError;
    use crate::agent::permissions::{
        JudgeRequest, PermissionBehavior, PermissionContext, PermissionDecision, PermissionMode,
        PermissionPolicy, PermissionScope, PermissionUpdate,
    };

    fn req<'a>(
        tool: &'a str,
        input: &'a serde_json::Value,
        ctx: &'a PermissionContext,
    ) -> JudgeRequest<'a> {
        JudgeRequest {
            tool,
            input,
            task: None,
            rationale: None,
            ctx,
        }
    }

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
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::BypassPermissions,
            &mut store,
            None,
            Some(&policy),
            &req("Bash", &input, &ctx),
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
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::DontAsk,
            &mut store,
            None,
            None,
            &req("Bash", &input, &ctx),
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
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::DontAsk,
            &mut store,
            None,
            Some(&policy),
            &req("Bash", &input, &ctx),
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
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Default,
            &mut store,
            None,
            Some(&policy),
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(store.lookup("Bash"), Some(PermissionBehavior::Deny));
    }

    #[tokio::test]
    async fn default_without_policy_allows() {
        let mut store = RuleStore::new(&[]);
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Default,
            &mut store,
            None,
            None,
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
    }

    use crate::agent::permissions::{JudgeRequest as _JR, PermissionJudge};

    struct MockJudge {
        called: Arc<AtomicBool>,
        decision: fn() -> PermissionDecision,
    }

    #[async_trait::async_trait]
    impl PermissionJudge for MockJudge {
        async fn judge(&self, _req: &_JR<'_>) -> Result<PermissionDecision, AgentError> {
            self.called.store(true, Ordering::SeqCst);
            Ok((self.decision)())
        }
    }

    fn deny_decision() -> PermissionDecision {
        PermissionDecision::deny("nope")
    }

    #[tokio::test]
    async fn auto_no_judge_denies_fail_closed() {
        let mut store = RuleStore::new(&[]);
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Auto,
            &mut store,
            None,
            None,
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        match decision {
            PermissionDecision::Deny { message, .. } => {
                assert!(message.contains("no judge configured"), "got: {message}");
            }
            other @ PermissionDecision::Allow { .. } => {
                panic!("expected Deny, got {other:?}")
            }
        }
    }

    #[tokio::test]
    async fn auto_judge_allow_no_policy_allows() {
        let jcalled = Arc::new(AtomicBool::new(false));
        let judge: Arc<dyn PermissionJudge> = Arc::new(MockJudge {
            called: jcalled.clone(),
            decision: allow_decision,
        });
        let mut store = RuleStore::new(&[]);
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Auto,
            &mut store,
            Some(&judge),
            None,
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
        assert!(jcalled.load(Ordering::SeqCst), "judge must be consulted");
    }

    #[tokio::test]
    async fn auto_judge_allow_then_policy_veto_denies() {
        let judge: Arc<dyn PermissionJudge> = Arc::new(MockJudge {
            called: Arc::new(AtomicBool::new(false)),
            decision: allow_decision,
        });
        let pcalled = Arc::new(AtomicBool::new(false));
        let policy: Arc<dyn PermissionPolicy> = Arc::new(SpyPolicy {
            called: pcalled.clone(),
            decision: deny_decision,
        });
        let mut store = RuleStore::new(&[]);
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Auto,
            &mut store,
            Some(&judge),
            Some(&policy),
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        assert!(
            matches!(decision, PermissionDecision::Deny { .. }),
            "policy vetoes the judge-allow"
        );
        assert!(
            pcalled.load(Ordering::SeqCst),
            "policy must be consulted on a judge-allow"
        );
    }

    #[tokio::test]
    async fn auto_judge_deny_is_final_policy_not_consulted() {
        let judge: Arc<dyn PermissionJudge> = Arc::new(MockJudge {
            called: Arc::new(AtomicBool::new(false)),
            decision: deny_decision,
        });
        let pcalled = Arc::new(AtomicBool::new(false));
        let policy: Arc<dyn PermissionPolicy> = Arc::new(SpyPolicy {
            called: pcalled.clone(),
            decision: allow_decision,
        });
        let mut store = RuleStore::new(&[]);
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Auto,
            &mut store,
            Some(&judge),
            Some(&policy),
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
        assert!(
            !pcalled.load(Ordering::SeqCst),
            "judge-deny must not consult the policy"
        );
    }

    #[tokio::test]
    async fn auto_pre_ruled_tool_skips_the_judge() {
        let jcalled = Arc::new(AtomicBool::new(false));
        let judge: Arc<dyn PermissionJudge> = Arc::new(MockJudge {
            called: jcalled.clone(),
            decision: deny_decision,
        });
        let mut store = RuleStore::new(&["Bash".to_string()]); // pre-approved
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        let decision = evaluate(
            PermissionMode::Auto,
            &mut store,
            Some(&judge),
            None,
            &req("Bash", &input, &ctx),
        )
        .await
        .expect("evaluate");
        assert!(matches!(decision, PermissionDecision::Allow { .. }));
        assert!(
            !jcalled.load(Ordering::SeqCst),
            "a pre-ruled tool must skip the judge"
        );
    }
}
