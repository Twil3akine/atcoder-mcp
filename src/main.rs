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
async fn fetch_problem(contest_id: &str, problem_id: &str) -> anyhow::Result<String> {
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

/// 解説ページ(一覧)を取得
async fn fetch_editorial(contest_id: &str, problem_id: &str) -> anyhow::Result<String> {
    let url = format!(
        "https://atcoder.jp/contests/{}/tasks/{}/editorial",
        contest_id, problem_id
    );

    let client = reqwest::Client::builder()
        .user_agent("atcoder-mcp/0.1.0")
        .build()?;

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Ok(format!(
            "Error: Failed to fetch editorial page. Status: {}",
            resp.status()
        ));
    }

    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // 解説ページのメインコンテンツを取得
    // 問題文とは違い、特定のIDがない場合が多いので、メインカラム全体(.col-sm-12)などを狙う
    // あるいは #main-container 内のテキストをざっくり取る
    let selector = Selector::parse("#main-container").map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if let Some(element) = document.select(&selector).next() {
        // テキストを抽出
        let text = element.text().collect::<Vec<_>>().join(" ");
        // 空白整理（改行などをきれいに）
        let cleaned_text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(cleaned_text)
    } else {
        Ok("Error: Could not find content in editorial page.".to_string())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        // JSON-RPCリクエストをパース
        let req: JsonRcpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                continue;
            }
        };

        // メソッドごとの分岐
        match req.method.as_str() {
            // 1. 初期化リクエスト (Zedが最初に送ってくる)
            "initialize" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "atcoder-hint-mcp",
                            "version": "0.1.0"
                        }
                    }
                });
                println!("{}", response);
            }

            // 2. 初期化完了通知 (返信不要)
            "notifications/initialized" => {
                // 何もしなくてOK
            }

            // 3. ツール一覧の要求 (Zed「どんな機能があるの？」)
            "tools/list" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": {
                        "tools": [{
                            "name": "fetch_problem",
                            "description": "AtCoderの問題文を取得します。contest_id (例: abc335) と problem_id (例: abc335_a) が必要です。",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "contest_id": { "type": "string" },
                                    "problem_id": { "type": "string" }
                                },
                                "required": ["contest_id", "problem_id"]
                            }
                        },
                        {
                            "name": "fetch_editorial",
                            "description": "AtCoderの解説ページを取得します。公式解説やユーザー解説の一覧とリンクが取得できます。contest_id と problem_id が必要です。",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "contest_id": { "type": "string" },
                                    "problem_id": { "type": "string" }
                                },
                                "required": ["contest_id", "problem_id"]
                            }
                        }]
                    }
                });
                println!("{}", response);
            }

            // 4. ツールの実行
            "tools/call" => {
                if let Some(params) = req.params {
                    // params["name"] は Value 型なので、文字列スライス (&str) に変換してから match する
                    if let Some(tool_name) = params["name"].as_str() {
                        match tool_name {
                            "fetch_problem" => {
                                let args = &params["arguments"];
                                let contest_id = args["contest_id"].as_str().unwrap_or("");
                                let problem_id = args["problem_id"].as_str().unwrap_or("");

                                let result_text = fetch_problem(contest_id, problem_id)
                                    .await
                                    .unwrap_or_else(|e| e.to_string());

                                // レスポンス作成関数を共通化してもいいかもですが、一旦ベタ書きで
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

                            "fetch_editorial" => {
                                let args = &params["arguments"];
                                let contest_id = args["contest_id"].as_str().unwrap_or("");
                                let problem_id = args["problem_id"].as_str().unwrap_or(""); // 解説では使わないこともあるけど引数には含めておく

                                let result_text = fetch_editorial(contest_id, problem_id)
                                    .await
                                    .unwrap_or_else(|e| format!("Error: {}", e));

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

                            // 定義されていないツールが呼ばれた場合
                            unknown_tool => {
                                eprintln!("Unknown tool called: {}", unknown_tool);
                            }
                        }
                    }
                }
            }

            // 未知のメソッドは無視
            _ => {}
        }
    }

    Ok(())
}
