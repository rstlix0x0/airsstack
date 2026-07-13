//! A model-backed [`PermissionJudge`]: asks a cheap model to allow or deny a
//! tool call against a built-in safety + task-fit rulebook.

use std::future::poll_fn;

use async_trait::async_trait;

use crate::agent::error::AgentError;
use crate::agent::message::Message;
use crate::agent::permissions::{JudgeRequest, JudgeRubric, PermissionDecision, PermissionJudge};
use crate::agent::runtime::Runtime;
use crate::agent::types::Prompt;

/// The built-in rulebook: allow only when a call is BOTH safe AND task-fitting;
/// deny otherwise, including when unsure.
const BUILTIN_RUBRIC: &str = "\
You gate an AI agent's tool calls. Reply with exactly ALLOW or DENY on line 1, \
and a one-line reason on line 2.\n\n\
Allow ONLY if BOTH of these hold:\n\
1. SAFETY  — the call is not destructive or irreversible, exposes no secrets, \
and performs no unexpected network egress.\n\
2. TASK-FIT — the call plainly serves the stated task.\n\
If either fails, or you are unsure, reply DENY.";

/// A [`PermissionJudge`] that asks a cheap model — driven through any
/// [`Runtime`] — to allow or deny each reviewed tool call.
///
/// The runtime handed here must be a plain completion runtime with **no
/// tools**: a tooled judge runtime would recurse back into permission gating.
pub struct RuntimeJudge<R: Runtime> {
    runtime: R,
    rubric: Option<JudgeRubric>,
}

impl<R: Runtime> RuntimeJudge<R> {
    /// Judge with the built-in safety + task-fit rulebook only.
    pub const fn new(runtime: R) -> Self {
        Self {
            runtime,
            rubric: None,
        }
    }

    /// Judge with the built-in rulebook augmented by caller guidance; the
    /// caller's rules take precedence on conflict.
    pub const fn with_rubric(runtime: R, rubric: JudgeRubric) -> Self {
        Self {
            runtime,
            rubric: Some(rubric),
        }
    }
}

/// Render the fixed judging prompt from the rulebook and the request.
fn build_prompt(rubric: Option<&JudgeRubric>, req: &JudgeRequest<'_>) -> String {
    let mut body = String::from(BUILTIN_RUBRIC);
    if let Some(extra) = rubric {
        body.push_str("\n\nAdditional caller rules (these take precedence on conflict):\n");
        body.push_str(extra.as_str());
    }
    let input = serde_json::to_string(req.input).unwrap_or_else(|_| "{}".to_string());
    body.push_str("\n\nTask:      ");
    body.push_str(req.task.unwrap_or("(unknown)"));
    body.push_str("\nRationale: ");
    body.push_str(req.rationale.unwrap_or("(none)"));
    body.push_str("\nTool:      ");
    body.push_str(req.tool);
    body.push_str("\nInput:     ");
    body.push_str(&input);
    body
}

/// Parse a model reply into a decision. First line `ALLOW`/`DENY` (leading
/// whitespace tolerated); anything else fails closed to a deny.
fn parse_verdict(reply: &str) -> PermissionDecision {
    let mut lines = reply.trim_start().lines();
    let first = lines.next().unwrap_or("").trim();
    let reason = lines.next().unwrap_or("").trim();
    if first.eq_ignore_ascii_case("ALLOW") {
        PermissionDecision::allow()
    } else if first.eq_ignore_ascii_case("DENY") {
        let message = if reason.is_empty() {
            "denied by judge"
        } else {
            reason
        };
        PermissionDecision::deny(message)
    } else {
        PermissionDecision::deny("judge returned an unparseable verdict")
    }
}

#[async_trait]
impl<R: Runtime> PermissionJudge for RuntimeJudge<R> {
    async fn judge(&self, req: &JudgeRequest<'_>) -> Result<PermissionDecision, AgentError> {
        let prompt = build_prompt(self.rubric.as_ref(), req);
        let mut stream = self.runtime.run(Prompt::new(prompt)).await?;

        // Drain the stream to its terminal result. `futures_util` is a
        // test-only dependency, so poll the `Stream` directly here.
        let mut reply: Option<String> = None;
        while let Some(item) = poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
            match item {
                Ok(Message::Result(r)) => reply = Some(r.result),
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        reply.map_or_else(
            || Ok(PermissionDecision::deny("judge produced no verdict")),
            |text| Ok(parse_verdict(&text)),
        )
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{RuntimeJudge, build_prompt, parse_verdict};
    use crate::agent::message::{Message, ResultMessage};
    use crate::agent::permissions::{
        JudgeRequest, JudgeRubric, PermissionContext, PermissionDecision, PermissionJudge,
    };
    use crate::agent::runtime::mock::MockRuntime;
    use crate::agent::types::SessionId;

    fn request<'a>(
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

    fn result(text: &str) -> Message {
        Message::Result(ResultMessage {
            result: text.into(),
            structured_output: None,
            is_error: false,
            total_cost_usd: None,
            stop_reason: None,
            usage: None,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    #[test]
    fn parses_allow() {
        assert!(matches!(
            parse_verdict("ALLOW\nlooks safe"),
            PermissionDecision::Allow { .. }
        ));
    }

    #[test]
    fn parses_deny_with_reason() {
        match parse_verdict("DENY\ndestructive") {
            PermissionDecision::Deny { message, .. } => assert_eq!(message, "destructive"),
            allow @ PermissionDecision::Allow { .. } => panic!("expected Deny, got {allow:?}"),
        }
    }

    #[test]
    fn unparseable_fails_closed_to_deny() {
        match parse_verdict("maybe?") {
            PermissionDecision::Deny { message, .. } => {
                assert!(message.contains("unparseable"));
            }
            allow @ PermissionDecision::Allow { .. } => panic!("expected Deny, got {allow:?}"),
        }
    }

    #[test]
    fn prompt_includes_task_rationale_and_user_rubric() {
        let input = serde_json::json!({ "cmd": "ls" });
        let ctx = PermissionContext::default();
        let req = JudgeRequest {
            tool: "Bash",
            input: &input,
            task: Some("clean the build dir"),
            rationale: Some("removing stale artifacts"),
            ctx: &ctx,
        };
        let rubric = JudgeRubric::new("never touch prod").expect("rubric");
        let body = build_prompt(Some(&rubric), &req);
        assert!(body.contains("clean the build dir"));
        assert!(body.contains("removing stale artifacts"));
        assert!(body.contains("never touch prod"));
        assert!(body.contains("Bash"));
    }

    #[tokio::test]
    async fn drives_runtime_and_denies_on_deny_reply() {
        let judge = RuntimeJudge::new(MockRuntime::new(vec![vec![result(
            "DENY\nrm is destructive",
        )]]));
        let input = serde_json::json!({ "cmd": "rm -rf /" });
        let ctx = PermissionContext::default();
        let decision = judge
            .judge(&request("Bash", &input, &ctx))
            .await
            .expect("judge");
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn empty_stream_denies_no_verdict() {
        let judge = RuntimeJudge::new(MockRuntime::new(vec![])); // empty stream
        let input = serde_json::json!({});
        let ctx = PermissionContext::default();
        match judge
            .judge(&request("Bash", &input, &ctx))
            .await
            .expect("judge")
        {
            PermissionDecision::Deny { message, .. } => assert!(message.contains("no verdict")),
            allow @ PermissionDecision::Allow { .. } => panic!("expected Deny, got {allow:?}"),
        }
    }
}
