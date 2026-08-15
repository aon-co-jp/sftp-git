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
struct GenerateRequest {
    prompt: String,
    max_new_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    completion: String,
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

    /// `/v1/generate`へプロンプトを送り、生成テキスト(`completion`)を返す。
    /// `/v1/chat`は意図分類ベースのFAQ応答用であり、差分解析のような
    /// 自由記述の生成には`/v1/generate`が適する(aruaru-llm側の設計方針、
    /// `src/main.rs`のコメント参照)。
    pub async fn chat(&self, prompt: &str) -> Result<String, AruaruLlmError> {
        let url = format!("{}/v1/generate", self.base_url.trim_end_matches('/'));
        let res = self
            .http
            .post(&url)
            .json(&GenerateRequest {
                prompt: prompt.to_string(),
                max_new_tokens: 128,
            })
            .send()
            .await
            .map_err(AruaruLlmError::Request)?;

        if !res.status().is_success() {
            return Err(AruaruLlmError::UnexpectedStatus(res.status()));
        }

        let body: GenerateResponse = res.json().await.map_err(AruaruLlmError::Request)?;
        Ok(body.completion)
    }

    /// 差分をaruaru-llmに解析させ、日英アドバイスを取得する。
    /// 2026-08-15実機検証で判明した通り、1回の呼び出しで「見出し付き
    /// 日英両方」の形式指定に従わせる方式(旧`analyze_diff_single_call`)
    /// は指示追従非対応の小型GPT-2系モデルでは実用に耐えないため、
    /// **日本語用・英語用に別プロンプトで2回呼び出す**方式を既定とする。
    pub async fn analyze_diff(
        &self,
        file_path: &str,
        diff_text: &str,
    ) -> Result<crate::ai_diff_advisor::BilingualAdvice, AruaruLlmError> {
        use crate::ai_diff_advisor::{build_prompt_en, build_prompt_ja, BilingualAdvice};

        let ja_prompt = build_prompt_ja(file_path, diff_text);
        let en_prompt = build_prompt_en(file_path, diff_text);

        let (japanese, english) = tokio::try_join!(self.chat(&ja_prompt), self.chat(&en_prompt))?;

        Ok(BilingualAdvice {
            japanese: japanese.trim().to_string(),
            english: english.trim().to_string(),
        })
    }

    /// 旧方式(1回呼び出し+見出しパース)。後方互換のため残すが、
    /// 指示追従非対応モデルでは失敗しやすいため`analyze_diff`を推奨する。
    pub async fn analyze_diff_single_call(
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
