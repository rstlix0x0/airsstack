//! Prompt-cache breakpoint policy and placement for the Messages API runtime.
//!
//! Cached request tokens bill at a fraction of fresh tokens on a cache hit, at
//! the cost of a one-time write. Because the runtime re-sends the system prompt
//! and tool catalog on every turn of the loop, marking that stable prefix — and
//! optionally the running conversation — as cacheable turns repeat sends into
//! cache reads.

use crate::messages::content::ContentBlock;
use crate::messages::request::{InputMessage, MessageContent};
use crate::messages::tools::Tool;
use crate::types::{CacheControl, SystemPrompt, SystemSegment};

/// How the runtime places prompt-cache breakpoints on outbound requests.
///
/// Cached request tokens bill at a fraction of fresh tokens on a cache hit, at
/// the cost of a one-time write on the turn that establishes the cache.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CachePolicy {
    /// Place no breakpoints. Requests are byte-identical to an uncached run.
    Off,
    /// Cache the stable prefix only: the system prompt and the full tool catalog.
    Prefix,
    /// Cache the stable prefix and, additionally, the conversation accumulated
    /// so far — a rolling breakpoint refreshed on each turn of the loop.
    #[default]
    PrefixAndConversation,
}

/// Mark the stable prefix (system + tools) as a cache boundary per `policy`.
///
/// The breakpoint goes on the last tool when any tool is declared — the API
/// caches everything up to and including it, i.e. the system prompt and every
/// tool. With no tools, the system prompt is rebuilt as a single cached
/// segment. With neither, there is nothing stable to cache and this is a no-op.
pub(super) fn apply_prefix(
    policy: CachePolicy,
    system: &mut Option<SystemPrompt>,
    tools: &mut [Tool],
) {
    if matches!(policy, CachePolicy::Off) {
        return;
    }
    if let Some(last) = tools.last_mut() {
        last.cache_control = Some(CacheControl::ephemeral());
    } else if let Some(prompt) = system {
        *prompt = cache_system_prompt(prompt);
    }
}

/// Rebuild a system prompt with a cache breakpoint on its final segment.
fn cache_system_prompt(prompt: &SystemPrompt) -> SystemPrompt {
    let cc = CacheControl::ephemeral();
    match prompt {
        SystemPrompt::Text(text) => {
            SystemPrompt::segments(vec![SystemSegment::text(text.clone()).with_cache(cc)])
        }
        SystemPrompt::Segments(segments) => {
            let mut segments = segments.clone();
            if let Some(last) = segments.last_mut() {
                last.cache_control = Some(cc);
            }
            SystemPrompt::segments(segments)
        }
    }
}

/// Under `PrefixAndConversation`, mark the last cacheable block of the most
/// recent block-form history turn as a rolling cache boundary. The initial
/// plain-text user turn carries no per-block breakpoint and is left untouched.
pub(super) fn apply_conversation(policy: CachePolicy, history: &mut [InputMessage]) {
    if !matches!(policy, CachePolicy::PrefixAndConversation) {
        return;
    }
    for message in history.iter_mut().rev() {
        if let MessageContent::Blocks(blocks) = &mut message.content {
            if mark_last_cacheable_block(blocks) {
                return;
            }
        }
    }
}

/// Stamp a cache breakpoint on the last block that can carry one, scanning from
/// the end. Thinking blocks carry no `cache_control`, so they are skipped.
/// Returns whether a block was marked.
fn mark_last_cacheable_block(blocks: &mut [ContentBlock]) -> bool {
    for block in blocks.iter_mut().rev() {
        let slot = match block {
            ContentBlock::Text(b) => &mut b.cache_control,
            ContentBlock::ToolUse(b) => &mut b.cache_control,
            ContentBlock::ToolResult(b) => &mut b.cache_control,
            ContentBlock::Thinking(_) => continue,
        };
        *slot = Some(CacheControl::ephemeral());
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{CachePolicy, apply_conversation, apply_prefix};
    use crate::messages::content::{ContentBlock, TextBlock, ThinkingBlock};
    use crate::messages::request::{InputMessage, MessageContent, Role};
    use crate::messages::tools::Tool;
    use crate::types::{SystemPrompt, ToolName};

    fn wire_tool(name: &str) -> Tool {
        Tool {
            name: ToolName::new(name).expect("name"),
            description: "d".into(),
            input_schema: serde_json::json!({"type": "object"}),
            cache_control: None,
            strict: None,
        }
    }

    #[test]
    fn default_policy_is_prefix_and_conversation() {
        assert_eq!(CachePolicy::default(), CachePolicy::PrefixAndConversation);
    }

    #[test]
    fn off_marks_no_tool() {
        let mut tools = vec![wire_tool("mcp__a__x"), wire_tool("mcp__a__y")];
        let mut system = None;
        apply_prefix(CachePolicy::Off, &mut system, &mut tools);
        assert!(tools.iter().all(|t| t.cache_control.is_none()));
    }

    #[test]
    fn prefix_marks_the_last_tool_only() {
        let mut tools = vec![wire_tool("mcp__a__x"), wire_tool("mcp__a__y")];
        let mut system = None;
        apply_prefix(CachePolicy::Prefix, &mut system, &mut tools);
        assert!(tools[0].cache_control.is_none());
        assert!(tools[1].cache_control.is_some());
    }

    #[test]
    fn prefix_marks_system_segment_when_no_tools() {
        let mut tools: Vec<Tool> = vec![];
        let mut system = Some(SystemPrompt::text("be terse"));
        apply_prefix(CachePolicy::Prefix, &mut system, &mut tools);
        match system.expect("system") {
            SystemPrompt::Segments(segs) => {
                assert_eq!(segs.len(), 1);
                assert!(segs[0].cache_control.is_some());
            }
            SystemPrompt::Text(_) => panic!("expected segmented system prompt"),
        }
    }

    #[test]
    fn prefix_no_tools_no_system_is_noop() {
        let mut tools: Vec<Tool> = vec![];
        let mut system: Option<SystemPrompt> = None;
        apply_prefix(CachePolicy::Prefix, &mut system, &mut tools);
        assert!(system.is_none());
    }

    fn blocks_turn(blocks: Vec<ContentBlock>) -> InputMessage {
        InputMessage {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    #[test]
    fn conversation_marks_last_block_of_recent_blocks_turn() {
        let mut history = vec![
            InputMessage {
                role: Role::User,
                content: MessageContent::Text("hi".into()),
            },
            blocks_turn(vec![ContentBlock::Text(TextBlock::new("answer"))]),
        ];
        apply_conversation(CachePolicy::PrefixAndConversation, &mut history);
        match &history[1].content {
            MessageContent::Blocks(b) => match &b[0] {
                ContentBlock::Text(t) => assert!(t.cache_control.is_some()),
                other => panic!("expected text, got {other:?}"),
            },
            MessageContent::Text(_) => panic!("expected blocks"),
        }
    }

    #[test]
    fn conversation_skips_trailing_thinking_block() {
        let mut history = vec![blocks_turn(vec![
            ContentBlock::Text(TextBlock::new("answer")),
            ContentBlock::Thinking(ThinkingBlock {
                thinking: "…".into(),
                signature: None,
            }),
        ])];
        apply_conversation(CachePolicy::PrefixAndConversation, &mut history);
        match &history[0].content {
            MessageContent::Blocks(b) => match &b[0] {
                ContentBlock::Text(t) => assert!(t.cache_control.is_some()),
                other => panic!("expected text, got {other:?}"),
            },
            MessageContent::Text(_) => panic!("expected blocks"),
        }
    }

    #[test]
    fn conversation_text_only_history_is_noop() {
        let mut history = vec![InputMessage {
            role: Role::User,
            content: MessageContent::Text("hi".into()),
        }];
        apply_conversation(CachePolicy::PrefixAndConversation, &mut history);
        assert!(matches!(history[0].content, MessageContent::Text(_)));
    }

    #[test]
    fn conversation_prefix_policy_leaves_history_untouched() {
        let mut history = vec![blocks_turn(vec![ContentBlock::Text(TextBlock::new(
            "answer",
        ))])];
        apply_conversation(CachePolicy::Prefix, &mut history);
        match &history[0].content {
            MessageContent::Blocks(b) => match &b[0] {
                ContentBlock::Text(t) => assert!(t.cache_control.is_none()),
                other => panic!("expected text, got {other:?}"),
            },
            MessageContent::Text(_) => panic!("expected blocks"),
        }
    }
}
