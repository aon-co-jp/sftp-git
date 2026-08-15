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
use sftp_git::ai_diff_advisor::build_prompt;
use sftp_git::aruaru_llm_client::AruaruLlmClient;
use sftp_git::cleanup_advisor::{advise, FileSignals};
use sftp_git::drift::{detect_drift, Manifest};
use sftp_git::dual_database::DualDatabaseState;
use std::collections::BTreeMap;
use std::sync::Mutex;

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

#[derive(Debug, Deserialize)]
struct DetectDriftParams {
    git_manifest: BTreeMap<String, String>,
    server_manifest: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct DriftEntryDto {
    path: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct BuildPromptParams {
    file_path: String,
    diff_text: String,
}

#[derive(Debug, Serialize)]
struct BuildPromptResult {
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct AnalyzeDiffParams {
    aruaru_llm_base_url: String,
    file_path: String,
    diff_text: String,
}

#[derive(Debug, Serialize)]
struct AnalyzeDiffResult {
    japanese: String,
    english: String,
}

#[derive(Debug, Serialize)]
struct DualDatabaseStateDto {
    primary: String,
    mode: String,
}

impl From<&DualDatabaseState> for DualDatabaseStateDto {
    fn from(s: &DualDatabaseState) -> Self {
        Self {
            primary: format!("{:?}", s.primary),
            mode: format!("{:?}", s.mode),
        }
    }
}

#[derive(Debug, Deserialize)]
struct VersionlessResolveParams {
    current: serde_json::Value,
    requested_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct VersionlessResolveResult {
    resolved: serde_json::Value,
}

/// versionless_api::VersionRegistry<T>は型TをRustのジェネリクスで持つため
/// LSP(JSON-RPC)越しにそのまま公開できない。ここではJSON値を対象にした
/// デモ変換(バージョン"2020-01-01"向けに`full_name`から`first_name`へ
/// 簡略化する例)のみを配線し、疎通確認とVS Code拡張からの呼び出し方の
/// 実例を示す。実運用でのバージョンごとの変換ルールは利用者側で拡張する
/// 想定(次回、設定ファイル等からの動的登録に対応させる)。
fn versionless_resolve_demo(current: &serde_json::Value, requested_version: Option<&str>) -> serde_json::Value {
    match requested_version {
        Some("2020-01-01") => {
            if let Some(full_name) = current.get("full_name").and_then(|v| v.as_str()) {
                let first = full_name.split_whitespace().next().unwrap_or("").to_string();
                serde_json::json!({ "first_name": first })
            } else {
                current.clone()
            }
        }
        _ => current.clone(),
    }
}

/// LSPサーバープロセス内で1つだけ保持するDUAL DATABASE状態。
/// VS Code拡張は状態そのものを持たず、都度カスタムリクエストで
/// 問い合わせる/遷移させる(業務ロジックをRust側に集約する方針通り)。
static DUAL_DB_STATE: Mutex<Option<DualDatabaseState>> = Mutex::new(None);

fn dual_db_state_mut() -> std::sync::MutexGuard<'static, Option<DualDatabaseState>> {
    let mut guard = DUAL_DB_STATE.lock().expect("DUAL_DB_STATEのロックに失敗");
    if guard.is_none() {
        *guard = Some(DualDatabaseState::initial());
    }
    guard
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("sftp-git LSPサーバーを起動します(標準入出力でVS Code拡張と通信)");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities::default())?;
    let initialize_params = connection.initialize(server_capabilities)?;
    let _initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;

    let runtime = tokio::runtime::Runtime::new()?;
    main_loop(&connection, &runtime)?;
    io_threads.join()?;

    eprintln!("sftp-git LSPサーバーを終了します");
    Ok(())
}

fn main_loop(
    connection: &Connection,
    runtime: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(connection, req, runtime)?;
            }
            Message::Response(_) | Message::Notification(_) => {
                // 現状は通知・レスポンスの処理は無し(カスタムリクエストのみ対応)。
            }
        }
    }
    Ok(())
}

fn send_invalid_params(
    connection: &Connection,
    id: RequestId,
    msg: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = Response::new_err(id, lsp_server::ErrorCode::InvalidParams as i32, msg);
    connection.sender.send(Message::Response(response))?;
    Ok(())
}

fn handle_request(
    connection: &Connection,
    req: Request,
    runtime: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match req.method.as_str() {
        "sftpGit/cleanupAdvice" => {
            let (id, params) = match cast_request::<CleanupAdviceParams>(req) {
                Ok(pair) => pair,
                Err((id, msg)) => return send_invalid_params(connection, id, msg),
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
        "sftpGit/detectDrift" => {
            let (id, params) = match cast_request::<DetectDriftParams>(req) {
                Ok(pair) => pair,
                Err((id, msg)) => return send_invalid_params(connection, id, msg),
            };
            let git_manifest: Manifest = params.git_manifest;
            let server_manifest: Manifest = params.server_manifest;
            let drifts = detect_drift(&git_manifest, &server_manifest);
            let result: Vec<DriftEntryDto> = drifts
                .into_iter()
                .map(|d| DriftEntryDto {
                    path: d.path,
                    kind: format!("{:?}", d.kind),
                })
                .collect();
            let response = Response::new_ok(id, serde_json::to_value(result)?);
            connection.sender.send(Message::Response(response))?;
        }
        "sftpGit/buildDiffPrompt" => {
            let (id, params) = match cast_request::<BuildPromptParams>(req) {
                Ok(pair) => pair,
                Err((id, msg)) => return send_invalid_params(connection, id, msg),
            };
            let prompt = build_prompt(&params.file_path, &params.diff_text);
            let response =
                Response::new_ok(id, serde_json::to_value(BuildPromptResult { prompt })?);
            connection.sender.send(Message::Response(response))?;
        }
        "sftpGit/analyzeDiff" => {
            let (id, params) = match cast_request::<AnalyzeDiffParams>(req) {
                Ok(pair) => pair,
                Err((id, msg)) => return send_invalid_params(connection, id, msg),
            };
            let client = AruaruLlmClient::new(params.aruaru_llm_base_url);
            // 2026-08-15実機検証の結果、日英を1回で見出し分割させる方式は
            // 指示追従非対応の小型GPT-2系モデルでは失敗するため、
            // 言語ごとに別プロンプトで2回呼び出す`analyze_diff`を使う。
            let outcome =
                runtime.block_on(client.analyze_diff(&params.file_path, &params.diff_text));
            match outcome {
                Ok(advice) => {
                    let result = AnalyzeDiffResult {
                        japanese: advice.japanese,
                        english: advice.english,
                    };
                    let response = Response::new_ok(id, serde_json::to_value(result)?);
                    connection.sender.send(Message::Response(response))?;
                }
                Err(client_err) => {
                    let response = Response::new_err(
                        id,
                        lsp_server::ErrorCode::InternalError as i32,
                        format!("aruaru-llmへの接続に失敗: {client_err}"),
                    );
                    connection.sender.send(Message::Response(response))?;
                }
            }
        }
        "sftpGit/versionlessResolve" => {
            let (id, params) = match cast_request::<VersionlessResolveParams>(req) {
                Ok(pair) => pair,
                Err((id, msg)) => return send_invalid_params(connection, id, msg),
            };
            let resolved =
                versionless_resolve_demo(&params.current, params.requested_version.as_deref());
            let response =
                Response::new_ok(id, serde_json::to_value(VersionlessResolveResult { resolved })?);
            connection.sender.send(Message::Response(response))?;
        }
        "sftpGit/dualDatabaseState" => {
            let state = dual_db_state_mut();
            let dto = DualDatabaseStateDto::from(state.as_ref().expect("初期化済みのはず"));
            let response = Response::new_ok(req.id, serde_json::to_value(dto)?);
            connection.sender.send(Message::Response(response))?;
        }
        "sftpGit/dualDatabaseOnPrimaryFailureDetected" => {
            let mut state = dual_db_state_mut();
            state
                .as_mut()
                .expect("初期化済みのはず")
                .on_primary_failure_detected();
            let dto = DualDatabaseStateDto::from(state.as_ref().expect("初期化済みのはず"));
            let response = Response::new_ok(req.id, serde_json::to_value(dto)?);
            connection.sender.send(Message::Response(response))?;
        }
        "sftpGit/dualDatabaseOnRecoveredAndResynced" => {
            let mut state = dual_db_state_mut();
            state
                .as_mut()
                .expect("初期化済みのはず")
                .on_recovered_and_resynced();
            let dto = DualDatabaseStateDto::from(state.as_ref().expect("初期化済みのはず"));
            let response = Response::new_ok(req.id, serde_json::to_value(dto)?);
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
