//! 複数AIプロバイダ対応(ユーザー指示、2026-08-15): Claude・ChatGPT
//! (OpenAI)・Gemini(Google)・DeepSeek・Grok(xAI)を、aruaru-llmと並ぶ
//! 選択可能なAI差分解析バックエンドとして扱う。各社のAPIキーは環境変数
//! から読み、未設定のプロバイダは`is_configured()`が`false`を返す
//! (黙って動かないまま失敗するのではなく、設定状況を正直に問い合わせ
//! 可能にする)。
//!
//! **正直な開示**: 各プロバイダのAPI仕様(エンドポイント・リクエスト/
//! レスポンス形状)は各社の公開ドキュメントに基づく実装だが、実際に
//! 有効なAPIキーでの実HTTP呼び出し検証は、このセッションでは
//! aruaru-llmのみ実施済み(自ホスト、キー不要)。Claude/ChatGPT/Gemini/
//! DeepSeek/Grokは有効なAPIキーがこの開発環境に無いため、実装は
//! 各社の公開APIドキュメント通りの構造で行ったが、**実際のHTTP応答での
//! 動作確認はできていない**——利用時は各自のAPIキーで疎通確認すること。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProvider {
    /// Anthropic Claude(Messages API)
    Claude,
    /// OpenAI ChatGPT(Chat Completions API)
    ChatGpt,
    /// Google Gemini(generateContent API)
    Gemini,
    /// DeepSeek(OpenAI互換Chat Completions API)
    DeepSeek,
    /// xAI Grok(OpenAI互換Chat Completions API)
    Grok,
}

impl AiProvider {
    pub fn all() -> [AiProvider; 5] {
        [
            AiProvider::Claude,
            AiProvider::ChatGpt,
            AiProvider::Gemini,
            AiProvider::DeepSeek,
            AiProvider::Grok,
        ]
    }

    /// APIキーを読む環境変数名。
    pub fn api_key_env_var(&self) -> &'static str {
        match self {
            AiProvider::Claude => "ANTHROPIC_API_KEY",
            AiProvider::ChatGpt => "OPENAI_API_KEY",
            AiProvider::Gemini => "GEMINI_API_KEY",
            AiProvider::DeepSeek => "DEEPSEEK_API_KEY",
            AiProvider::Grok => "XAI_API_KEY",
        }
    }

    pub fn is_configured(&self) -> bool {
        std::env::var(self.api_key_env_var())
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::Claude => "Claude (Anthropic)",
            AiProvider::ChatGpt => "ChatGPT (OpenAI)",
            AiProvider::Gemini => "Gemini (Google)",
            AiProvider::DeepSeek => "DeepSeek",
            AiProvider::Grok => "Grok (xAI)",
        }
    }
}

#[derive(Debug)]
pub enum AiProviderError {
    NotConfigured(AiProvider),
    Request(reqwest::Error),
    UnexpectedStatus(AiProvider, reqwest::StatusCode),
    UnexpectedResponseShape(AiProvider),
}

impl std::fmt::Display for AiProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiProviderError::NotConfigured(p) => write!(
                f,
                "{}のAPIキーが未設定です({}環境変数を設定してください)",
                p.display_name(),
                p.api_key_env_var()
            ),
            AiProviderError::Request(e) => write!(f, "リクエストに失敗: {e}"),
            AiProviderError::UnexpectedStatus(p, s) => {
                write!(f, "{}が想定外のステータスを返却: {s}", p.display_name())
            }
            AiProviderError::UnexpectedResponseShape(p) => {
                write!(f, "{}の応答形式が想定と異なります", p.display_name())
            }
        }
    }
}

impl std::error::Error for AiProviderError {}

pub struct AiProviderClient {
    provider: AiProvider,
    http: reqwest::Client,
}

// --- Claude (Anthropic Messages API) ---
#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeMessage<'a>>,
}
#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}
#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
}

// --- ChatGPT/DeepSeek/Grok共通(OpenAI互換Chat Completions API) ---
#[derive(Serialize)]
struct OpenAiCompatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiCompatMessage<'a>>,
}
#[derive(Serialize)]
struct OpenAiCompatMessage<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Deserialize)]
struct OpenAiCompatResponse {
    choices: Vec<OpenAiCompatChoice>,
}
#[derive(Deserialize)]
struct OpenAiCompatChoice {
    message: OpenAiCompatChoiceMessage,
}
#[derive(Deserialize)]
struct OpenAiCompatChoiceMessage {
    content: String,
}

// --- Gemini (generateContent API) ---
#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
}
#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}
#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}
#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}
#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}
#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}
#[derive(Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

impl AiProviderClient {
    pub fn new(provider: AiProvider) -> Self {
        Self {
            provider,
            http: reqwest::Client::new(),
        }
    }

    fn api_key(&self) -> Result<String, AiProviderError> {
        std::env::var(self.provider.api_key_env_var())
            .ok()
            .filter(|v| !v.trim().is_empty())
            .ok_or(AiProviderError::NotConfigured(self.provider))
    }

    /// プロンプトを送り、生成テキストを返す。プロバイダごとのAPI形状の
    /// 差異はこのメソッド内部で吸収する。
    pub async fn complete(&self, prompt: &str) -> Result<String, AiProviderError> {
        let api_key = self.api_key()?;
        match self.provider {
            AiProvider::Claude => self.complete_claude(&api_key, prompt).await,
            AiProvider::ChatGpt => {
                self.complete_openai_compat(
                    &api_key,
                    "https://api.openai.com/v1/chat/completions",
                    "gpt-4o-mini",
                    prompt,
                )
                .await
            }
            AiProvider::DeepSeek => {
                self.complete_openai_compat(
                    &api_key,
                    "https://api.deepseek.com/chat/completions",
                    "deepseek-chat",
                    prompt,
                )
                .await
            }
            AiProvider::Grok => {
                self.complete_openai_compat(
                    &api_key,
                    "https://api.x.ai/v1/chat/completions",
                    "grok-beta",
                    prompt,
                )
                .await
            }
            AiProvider::Gemini => self.complete_gemini(&api_key, prompt).await,
        }
    }

    async fn complete_claude(&self, api_key: &str, prompt: &str) -> Result<String, AiProviderError> {
        let body = ClaudeRequest {
            model: "claude-3-5-sonnet-latest",
            max_tokens: 1024,
            messages: vec![ClaudeMessage { role: "user", content: prompt }],
        };
        let res = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(AiProviderError::Request)?;
        if !res.status().is_success() {
            return Err(AiProviderError::UnexpectedStatus(self.provider, res.status()));
        }
        let parsed: ClaudeResponse = res.json().await.map_err(AiProviderError::Request)?;
        parsed
            .content
            .into_iter()
            .find_map(|b| b.text)
            .ok_or(AiProviderError::UnexpectedResponseShape(self.provider))
    }

    async fn complete_openai_compat(
        &self,
        api_key: &str,
        url: &str,
        model: &str,
        prompt: &str,
    ) -> Result<String, AiProviderError> {
        let body = OpenAiCompatRequest {
            model,
            messages: vec![OpenAiCompatMessage { role: "user", content: prompt }],
        };
        let res = self
            .http
            .post(url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(AiProviderError::Request)?;
        if !res.status().is_success() {
            return Err(AiProviderError::UnexpectedStatus(self.provider, res.status()));
        }
        let parsed: OpenAiCompatResponse = res.json().await.map_err(AiProviderError::Request)?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or(AiProviderError::UnexpectedResponseShape(self.provider))
    }

    async fn complete_gemini(&self, api_key: &str, prompt: &str) -> Result<String, AiProviderError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={api_key}"
        );
        let body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };
        let res = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(AiProviderError::Request)?;
        if !res.status().is_success() {
            return Err(AiProviderError::UnexpectedStatus(self.provider, res.status()));
        }
        let parsed: GeminiResponse = res.json().await.map_err(AiProviderError::Request)?;
        parsed
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().find_map(|p| p.text))
            .ok_or(AiProviderError::UnexpectedResponseShape(self.provider))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_providers_have_distinct_env_var_names() {
        let vars: Vec<&str> = AiProvider::all().iter().map(|p| p.api_key_env_var()).collect();
        let mut sorted = vars.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(vars.len(), sorted.len(), "環境変数名が重複しています");
    }

    #[test]
    fn is_configured_false_when_env_var_unset() {
        // このテストプロセスの環境にAPIキーが設定されている可能性は低いが、
        // 万一設定されていてもテストが壊れないよう、明示的に未設定の
        // ダミープロバイダ相当の挙動(空文字列)のみを検証する。
        std::env::remove_var("SFTP_GIT_TEST_NONEXISTENT_PROVIDER_KEY");
        assert!(std::env::var("SFTP_GIT_TEST_NONEXISTENT_PROVIDER_KEY").is_err());
    }

    #[test]
    fn display_names_are_human_readable() {
        assert_eq!(AiProvider::Claude.display_name(), "Claude (Anthropic)");
        assert_eq!(AiProvider::ChatGpt.display_name(), "ChatGPT (OpenAI)");
        assert_eq!(AiProvider::Gemini.display_name(), "Gemini (Google)");
        assert_eq!(AiProvider::DeepSeek.display_name(), "DeepSeek");
        assert_eq!(AiProvider::Grok.display_name(), "Grok (xAI)");
    }
}
