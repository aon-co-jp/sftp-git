//! aruaru-llmへの実HTTPクライアント。`ai_diff_advisor`のプロンプト組み立て/
//! レスポンス解析ロジックと、`cleanup_advisor`の判断材料を実際に
//! aruaru-llmサーバー(`/v1/chat`)へ送るための配線。
//!
//! ネットワークが必要なテスト(実サーバー呼び出し)は`#[ignore]`とし、
//! `cargo test -- --ignored`かつ`ARUARU_LLM_BASE_URL`環境変数を
//! 設定した場合のみ手動実行する想定(CI等での既定実行では走らない)。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AruaruLlmClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(alias = "response", alias = "text", alias = "output")]
    text: String,
}

#[derive(Debug)]
pub enum AruaruLlmError {
    Request(reqwest::Error),
    UnexpectedStatus(reqwest::StatusCode),
}

impl std::fmt::Display for AruaruLlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AruaruLlmError::Request(e) => write!(f, "aruaru-llmへのリクエストに失敗: {e}"),
            AruaruLlmError::UnexpectedStatus(s) => write!(f, "aruaru-llmが想定外のステータスを返却: {s}"),
        }
    }
}

impl std::error::Error for AruaruLlmError {}

impl AruaruLlmClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// `/v1/chat`へプロンプトを送り、生成テキストを返す。
    pub async fn chat(&self, prompt: &str) -> Result<String, AruaruLlmError> {
        let url = format!("{}/v1/chat", self.base_url.trim_end_matches('/'));
        let res = self
            .http
            .post(&url)
            .json(&ChatRequest {
                prompt: prompt.to_string(),
            })
            .send()
            .await
            .map_err(AruaruLlmError::Request)?;

        if !res.status().is_success() {
            return Err(AruaruLlmError::UnexpectedStatus(res.status()));
        }

        let body: ChatResponse = res.json().await.map_err(AruaruLlmError::Request)?;
        Ok(body.text)
    }

    /// 差分をaruaru-llmに解析させ、日英アドバイスを取得する。
    /// `ai_diff_advisor`のプロンプト組み立て・レスポンス解析ロジックを
    /// 実HTTP呼び出しに配線する。
    pub async fn analyze_diff(
        &self,
        file_path: &str,
        diff_text: &str,
    ) -> Result<crate::ai_diff_advisor::BilingualAdvice, AnalyzeDiffError> {
        let prompt = crate::ai_diff_advisor::build_prompt(file_path, diff_text);
        let raw = self.chat(&prompt).await.map_err(AnalyzeDiffError::Client)?;
        crate::ai_diff_advisor::parse_response(&raw).map_err(AnalyzeDiffError::Parse)
    }
}

#[derive(Debug)]
pub enum AnalyzeDiffError {
    Client(AruaruLlmError),
    Parse(crate::ai_diff_advisor::ParseError),
}

impl std::fmt::Display for AnalyzeDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzeDiffError::Client(e) => write!(f, "{e}"),
            AnalyzeDiffError::Parse(e) => write!(f, "aruaru-llmの応答解析に失敗: {e:?}"),
        }
    }
}

impl std::error::Error for AnalyzeDiffError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_stores_base_url_without_trailing_slash_issues() {
        let client = AruaruLlmClient::new("http://localhost:8080/");
        assert_eq!(client.base_url, "http://localhost:8080/");
    }

    /// 実サーバーへの接続が必要なため既定では実行しない。
    /// `ARUARU_LLM_BASE_URL=http://localhost:8080 cargo test -- --ignored`
    /// のように、実際にaruaru-llmを起動した環境でのみ手動実行する。
    #[tokio::test]
    #[ignore]
    async fn analyze_diff_against_real_server() {
        let base_url = std::env::var("ARUARU_LLM_BASE_URL")
            .expect("ARUARU_LLM_BASE_URLを設定してください");
        let client = AruaruLlmClient::new(base_url);
        let advice = client
            .analyze_diff("index.html", "-<h1>old</h1>\n+<h1>new</h1>")
            .await
            .expect("aruaru-llmへの実接続に失敗");
        assert!(!advice.japanese.is_empty());
        assert!(!advice.english.is_empty());
    }
}
