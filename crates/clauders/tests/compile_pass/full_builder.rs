fn main() -> Result<(), clauders::error::BuildError> {
    let key = clauders::types::ApiKey::new("sk-test-key").unwrap();
    let _client = clauders::Client::builder()?
        .api_key(key)
        .build()?;
    Ok(())
}
