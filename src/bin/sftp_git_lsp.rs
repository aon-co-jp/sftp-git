//! sftp-git LSPサーバー(rust-analyzer方式)。DESIGN.md「7.」参照。
//!
//! VS Code拡張機能側(Node製、`vscode-extension/`)はこのバイナリを
//! 子プロセスとして起動し、標準入出力でLSPメッセージを中継するだけの
//! 薄い接続層に留める。業務ロジックは全てこちら側(Rust)が持つ。
//!
//! 標準のLSPリクエスト(初期化等)に加え、sftp-git独自のカスタム
//! リクエスト`sftpGit/cleanupAdvice`を実装し、`cleanup_advisor`の
//! 判定結果をVS Code側へ返す(他の4機能も同様のカスタムリクエストとして
//! 順次追加していく想定、現状はまず1つを配線して疎通を確認する)。

use lsp_server::{Connection, Message, Request, RequestId, Response};
use lsp_types::{InitializeParams, ServerCapabilities};
use serde::{Deserialize, Serialize};
use sftp_git::cleanup_advisor::{advise, FileSignals};

#[derive(Debug, Deserialize)]
struct CleanupAdviceParams {
    path: String,
    days_since_last_commit: u32,
    is_referenced: bool,
    was_accessed_in_production: Option<bool>,
}

#[derive(Debug, Serialize)]
struct CleanupAdviceResult {
    recommendation: String,
    reasons: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("sftp-git LSPサーバーを起動します(標準入出力でVS Code拡張と通信)");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities::default())?;
    let initialize_params = connection.initialize(server_capabilities)?;
    let _initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;

    main_loop(&connection)?;
    io_threads.join()?;

    eprintln!("sftp-git LSPサーバーを終了します");
    Ok(())
}

fn main_loop(connection: &Connection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(connection, req)?;
            }
            Message::Response(_) | Message::Notification(_) => {
                // 現状は通知・レスポンスの処理は無し(カスタムリクエストのみ対応)。
            }
        }
    }
    Ok(())
}

fn handle_request(
    connection: &Connection,
    req: Request,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match req.method.as_str() {
        "sftpGit/cleanupAdvice" => {
            let (id, params) = match cast_request::<CleanupAdviceParams>(req) {
                Ok(pair) => pair,
                Err((id, msg)) => {
                    let response = Response::new_err(id, lsp_server::ErrorCode::InvalidParams as i32, msg);
                    connection.sender.send(Message::Response(response))?;
                    return Ok(());
                }
            };
            let signals = FileSignals {
                path: params.path,
                days_since_last_commit: params.days_since_last_commit,
                is_referenced: params.is_referenced,
                was_accessed_in_production: params.was_accessed_in_production,
            };
            let advice = advise(&signals);
            let result = CleanupAdviceResult {
                recommendation: format!("{:?}", advice.recommendation),
                reasons: advice.reasons,
            };
            let response = Response::new_ok(id, serde_json::to_value(result)?);
            connection.sender.send(Message::Response(response))?;
        }
        _ => {
            let response = Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("未対応のメソッド: {}", req.method),
            );
            connection.sender.send(Message::Response(response))?;
        }
    }
    Ok(())
}

fn cast_request<P: for<'de> Deserialize<'de>>(req: Request) -> Result<(RequestId, P), (RequestId, String)> {
    let id = req.id.clone();
    serde_json::from_value(req.params)
        .map(|params| (id.clone(), params))
        .map_err(|e| (id, format!("パラメータのデシリアライズに失敗: {e}")))
}
