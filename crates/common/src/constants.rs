pub const DEFAULT_PORT: u16 = 3000;
pub const API_PREFIX: &str = "/console/api";
pub const INTERNAL_PREFIX: &str = "/console/internal";
pub const WS_PATH: &str = "/ws";

// Helper to get base URL
pub fn get_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{}", port)
}

// Helper to get API URL
pub fn get_api_url(port: u16, path: &str) -> String {
    let path = path.trim_start_matches('/');
    format!("{}{}/{}", get_base_url(port), API_PREFIX, path)
}
