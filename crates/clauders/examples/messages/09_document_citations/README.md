# 09 — Document with citations

Ground an answer in a supplied document and read back the exact spans the model
cited.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 09_document_citations
```

## What it shows

**Send a document block with citations enabled.** The example uses a plain-text
source so no PDF is needed:

```rust
use clauders::messages::{CitationsConfig, DocumentBlock, DocumentSource, PlainTextMediaType};

let document = DocumentBlock {
    source: DocumentSource::Text {
        media_type: PlainTextMediaType::TextPlain,
        data: DOCUMENT.into(),
    },
    cache_control: None,
    citations: Some(CitationsConfig { enabled: true }),   // the toggle
    context: None,
    title: Some("acme-facts.txt".into()),
};

let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024))
    .add_message(Role::User, MessageContent::Blocks(vec![
        ContentBlockParam::Document(document),
        ContentBlockParam::Text(TextBlock::new(
            "When was Acme Corp founded and when did it go public? Cite the document.")),
    ]))
    .build();
```

**Read the citations** off each text block. `TextBlock.citations` is
`Option<Vec<TextCitation>>`; a plain-text document yields `CharLocation` spans:

```rust
if let ContentBlock::Text(t) = block {
    println!("{}", t.text);
    for citation in t.citations.iter().flatten() {
        if let TextCitation::CharLocation { cited_text, start_char_index, end_char_index, .. } = citation {
            println!("  cited [{start_char_index}..{end_char_index}]: {cited_text:?}");
        }
    }
}
```

## Notes

- `TextCitation` is `#[non_exhaustive]` with five location kinds (`CharLocation`,
  `PageLocation`, `ContentBlockLocation`, `WebSearchResultLocation`,
  `SearchResultLocation`) plus an `Unknown` fallback for kinds a future API adds.
  Which kind you get depends on the document source — plain text and PDF-text
  give char ranges, paged PDFs give page ranges.
- `DocumentSource` also has `Base64` (PDF), `Url` (PDF), and `Content` (embedded)
  forms.
