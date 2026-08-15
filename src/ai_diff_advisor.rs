//! テスト/本番差分のAIチェック+日英アドバイス。DESIGN.md「5.」参照。
//!
//! aruaru-llmの`/v1/chat`または`/v1/generate`へ渡すプロンプトの組み立てと、
//! レスポンスのパース(日本語アドバイス・英語アドバイスへの分離)を行う。
//! 実際のHTTP呼び出しは未接続(次回、aruaru-llmクライアント実装と合わせて
//! 行う)。ここでは呼び出し前後のロジックのみを実装・テストする。

const JA_MARKER: &str = "### 日本語アドバイス";
const EN_MARKER: &str = "### English Advice";

/// aruaru-llmへ渡すプロンプトを組み立てる。差分本文と対象パスを受け取り、
/// 日本語・英語両方のアドバイスを、決まった見出しで区切って出力するよう
/// 指示するプロンプトを返す。
pub fn build_prompt(file_path: &str, diff_text: &str) -> String {
    format!(
        "以下はファイル`{path}`のテスト/本番差分です。\n\
         この差分についてレビューし、問題点や改善案を指摘してください。\n\
         \n\
         出力は必ず次の2つの見出しで構成し、それぞれの言語で\
         同じ内容のアドバイスを書いてください:\n\
         {ja_marker}\n\
         (ここに日本語で)\n\
         {en_marker}\n\
         (Write the same advice in English here)\n\
         \n\
         --- diff ---\n\
         {diff}",
        path = file_path,
        ja_marker = JA_MARKER,
        en_marker = EN_MARKER,
        diff = diff_text,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilingualAdvice {
    pub japanese: String,
    pub english: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    MissingJapaneseSection,
    MissingEnglishSection,
}

/// aruaru-llmのレスポンス本文(生成テキスト)から、日英アドバイスを
/// 見出しをもとに分離する。
pub fn parse_response(raw: &str) -> Result<BilingualAdvice, ParseError> {
    let ja_start = raw.find(JA_MARKER).ok_or(ParseError::MissingJapaneseSection)?;
    let en_start = raw.find(EN_MARKER).ok_or(ParseError::MissingEnglishSection)?;

    if en_start < ja_start {
        // 英語見出しが先に来る形式は想定しない(プロンプトで順序を固定しているため)
        return Err(ParseError::MissingJapaneseSection);
    }

    let japanese = raw[ja_start + JA_MARKER.len()..en_start].trim().to_string();
    let english = raw[en_start + EN_MARKER.len()..].trim().to_string();

    Ok(BilingualAdvice { japanese, english })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_includes_path_diff_and_both_markers() {
        let prompt = build_prompt("index.html", "-old\n+new");
        assert!(prompt.contains("index.html"));
        assert!(prompt.contains("-old\n+new"));
        assert!(prompt.contains(JA_MARKER));
        assert!(prompt.contains(EN_MARKER));
    }

    #[test]
    fn parse_response_splits_japanese_and_english_sections() {
        let raw = format!(
            "{ja}\n本番のみ変更された内容が検出されました。\n{en}\nA change was detected only in production.",
            ja = JA_MARKER,
            en = EN_MARKER,
        );
        let advice = parse_response(&raw).unwrap();
        assert_eq!(advice.japanese, "本番のみ変更された内容が検出されました。");
        assert_eq!(advice.english, "A change was detected only in production.");
    }

    #[test]
    fn parse_response_fails_when_japanese_marker_missing() {
        let raw = format!("{en}\nSome advice.", en = EN_MARKER);
        assert_eq!(
            parse_response(&raw).unwrap_err(),
            ParseError::MissingJapaneseSection
        );
    }

    #[test]
    fn parse_response_fails_when_english_marker_missing() {
        let raw = format!("{ja}\n何かのアドバイス。", ja = JA_MARKER);
        assert_eq!(
            parse_response(&raw).unwrap_err(),
            ParseError::MissingEnglishSection
        );
    }
}
