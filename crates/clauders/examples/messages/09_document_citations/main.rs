//! Document input with citations — ground an answer in a supplied document.
//!
//! Sends a plain-text document with `citations` enabled, then asks a question
//! answerable only from it. The reply's text blocks carry `citations` that
//! point back at the exact spans used, so the answer is traceable to source.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 09_document_citations
//! ```

use clauders::messages::{
    CitationsConfig, DocumentBlock, DocumentSource, MessageContent, PlainTextMediaType, TextBlock,
    TextCitation,
};
use clauders::prelude::*;

const DOCUMENT: &str = "\
Acme Corp was founded in 1997 in Portland, Oregon. \
Its first product was a line of industrial label printers. \
The company went public in 2012 and employs about 4,500 people.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024);

    let document = DocumentBlock {
        source: DocumentSource::Text {
            media_type: PlainTextMediaType::TextPlain,
            data: DOCUMENT.into(),
        },
        cache_control: None,
        citations: Some(CitationsConfig { enabled: true }),
        context: None,
        title: Some("acme-facts.txt".into()),
    };

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .add_message(
            Role::User,
            MessageContent::Blocks(vec![
                ContentBlockParam::Document(document),
                ContentBlockParam::Text(TextBlock::new(
                    "When was Acme Corp founded and when did it go public? Cite the document.",
                )),
            ]),
        )
        .build();

    let msg = client.messages().create(req).await?;

    for block in &msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
            for citation in t.citations.iter().flatten() {
                print_citation(citation);
            }
        }
    }

    Ok(())
}

/// Print the span a citation points at. Plain-text documents yield
/// `CharLocation`; the other kinds are shown generically.
fn print_citation(citation: &TextCitation) {
    match citation {
        TextCitation::CharLocation {
            cited_text,
            start_char_index,
            end_char_index,
            ..
        } => {
            println!("  ↳ cited [{start_char_index}..{end_char_index}]: {cited_text:?}");
        }
        other => {
            println!("  ↳ cited: {other:?}");
        }
    }
}
