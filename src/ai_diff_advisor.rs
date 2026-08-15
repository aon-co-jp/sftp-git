//! テスト/本番差分のAIチェック+日英アドバイス。DESIGN.md「5.」「9.」参照。
//!
//! **2026-08-15実機検証での発見**: `build_prompt`+`parse_response`
//! (1回の呼び出しで日英2見出し付き出力を要求する方式)は、
//! `distilgpt2`(82M)でも`gpt2-medium`(355M、330秒かけて実行)でも
//! 失敗した。GPT-2ファミリーは指示追従(instruction-tuned)されて
//! いないため、モデルを大きくするだけでは形式指定への追従は改善しない。
//! この教訓を受け、`build_prompt_ja`/`build_prompt_en`で**言語ごとに
//! 別々のプロンプトを組み立て2回呼び出す**方式を追加した(こちらが
//! 現在の推奨方式)。旧方式(`build_prompt`/`parse_response`)は後方互換
//! として残すが、実運用では新方式を使うこと。

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

/// 日本語アドバイス用のプロンプト(2回呼び出し方式、推奨)。
/// 形式指定(見出し等)を要求せず、日本語で書き始めさせるプライミングの
/// みに頼ることで、指示追従非対応の小型モデルでも壊れにくくする。
pub fn build_prompt_ja(file_path: &str, diff_text: &str) -> String {
    format!(
        "ファイル`{path}`のテスト/本番差分について、問題点や改善案を\
         日本語で簡潔に述べます。\n\n--- diff ---\n{diff}\n\n差分についての所見: ",
        path = file_path,
        diff = diff_text,
    )
}

/// 英語アドバイス用のプロンプト(2回呼び出し方式、推奨)。
pub fn build_prompt_en(file_path: &str, diff_text: &str) -> String {
    format!(
        "Here is a brief review of the test/production diff for file `{path}`.\n\n\
         --- diff ---\n{diff}\n\nObservation: ",
        path = file_path,
        diff = diff_text,
    )
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

    #[test]
    fn build_prompt_ja_includes_path_and_diff_and_is_japanese_primed() {
        let prompt = build_prompt_ja("index.html", "-old\n+new");
        assert!(prompt.contains("index.html"));
        assert!(prompt.contains("-old\n+new"));
        assert!(prompt.contains("日本語"));
    }

    #[test]
    fn build_prompt_en_includes_path_and_diff_and_is_english_primed() {
        let prompt = build_prompt_en("index.html", "-old\n+new");
        assert!(prompt.contains("index.html"));
        assert!(prompt.contains("-old\n+new"));
        assert!(prompt.contains("Observation"));
    }
}
