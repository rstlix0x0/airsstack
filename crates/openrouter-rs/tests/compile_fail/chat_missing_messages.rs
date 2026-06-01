use openrouter_rs::chat::ChatRequest;
use openrouter_rs::types::ModelId;

fn main() {
    // model set, messages missing — build() must not exist.
    let _ = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o").unwrap())
        .build();
}
