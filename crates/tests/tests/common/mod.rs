use dotenvy::dotenv;
use std::env;

pub fn get_base_url() -> String {
    dotenv().ok();
    env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
}

pub fn get_root_token() -> String {
    // Matches schema.rs initialization
    "sk-root-token-123456".to_string()
}

pub fn get_demo_token() -> String {
    // Matches schema.rs initialization
    "sk-burncloud-demo".to_string()
}

pub fn get_openai_config() -> Option<(String, String)> {
    dotenv().ok();
    let key = env::var("TEST_OPENAI_KEY").ok().filter(|k| !k.is_empty())?;
    let url = env::var("TEST_OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".to_string());
    Some((key, url))
}