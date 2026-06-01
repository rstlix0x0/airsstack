use openrouter_rs::chat::{ChatRequest, Message};
use openrouter_rs::types::ModelId;

fn main() {
    let _ = ChatRequest::builder()
        .model(ModelId::custom("openai/gpt-4o").unwrap())
        .messages(vec![Message::user("hi")])
        .build();
}
