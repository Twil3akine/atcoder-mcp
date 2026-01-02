use serde::Deserialize;

/// MCPのリクエスト構造
#[derive(Deserialize)]
struct JsonRcpRequest {
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

fn main() {}
