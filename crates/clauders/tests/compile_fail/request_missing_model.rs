fn main() {
    let _req = clauders::messages::MessageRequest::builder()
        .max_tokens(clauders::types::MaxTokens::new(1).unwrap())
        .build();
}
