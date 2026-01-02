use reqwest::Request;
use scraper::{Html, Selector};
use serde::Deserialize;

/// MCPのリクエスト構造
#[derive(Deserialize)]
struct JsonRcpRequest {
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

/// スクレイピング機能: 指定した問題のHTMLを取得してテキストを抽出
async fn fetch_problem(contest_id: &str, problem_id: &str) -> Result<String> {
    let url = format!(
        "https://atcoder.jp/contests/{}/tasks/{}",
        contest_id, problem_id
    );

    // User-Agentを設定しないと拒否されるかもなので注意
    let client = reqwest::Client::builder()
        .user_agent("atcoder-hint-mcp/0.1.0")
        .build()?;

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Ok(format!(
            "Error: Failed to fetch page. Status: {}",
            resp.status()
        ));
    }

    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // 問題文のセクションを取得(AtCoderのHTML構造に依存)
    // #task-statement というIDの中に問題文がある。らしい。
    let selector = Selector::parse("#task-statement").unwrap();

    if let Some(element) = document.select(&selector).next() {
        // テキストだけ抽出 (MD変換はとりあえず置いとく)
        let text = element.text().collect::<Vec<_>>().join("");
        Ok(text.trim().to_string())
    } else {
        Ok("Error: Could not find problem statement in HTML.".to_string())
    }
}

fn main() {}
