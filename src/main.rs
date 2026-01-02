use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::json;
use std::io::{self, BufRead};

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

#[tokio::main]
async fn main() -> Result<()> {
    // なんか本来はツールの定義？が必要らしい
    // あと、MCPに基づいた handshake が必要らしい

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        // JSON-RPCリクエストをパース
        let req: JsonRcpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue, // パースエラーは無視
        };

        // ツールの実行レクかチェック
        if req.method == "tools/call" {
            if let Some(params) = req.params {
                if params["name"] == "fetch_problem" {
                    let args = &params["argument"];
                    let contest_id = args["contest_id"].as_str().unwrap_or("");
                    let problem_id = args["problem_id"].as_str().unwrap_or("");

                    // ロジックの実行
                    let result_text = fetch_problem(contest_id, problem_id)
                        .await
                        .unwrap_or_else(|e| e.to_string());

                    // 結果をJSON-RPC形式で返す
                    let response = json!({
                        "jsonrpc": "2.0",
                         "id": req.id,
                      "result": {
                          "content": [{
                           "type": "text",
                        "text": result_text
                       }]
                      }
                    });
                    println!("{}", response);
                }
            }
        }
    }

    Ok(())
}
