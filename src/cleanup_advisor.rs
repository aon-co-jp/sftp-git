//! 不要ファイルのAI削除判定支援。DESIGN.md「3.」参照。
//!
//! ここでは、aruaru-llmへ渡す判断材料を整理するロジックと、
//! ヒューリスティックによる暫定スコアリングを実装する。
//! **削除の実行権限は持たない**——このモジュールはあくまで
//! 「削除候補として妥当そうか」の判断材料・所見を返すのみで、
//! 実際の削除は常に作業者の承認を経て別途行う(誤削除防止方針)。

#[derive(Debug, Clone)]
pub struct FileSignals {
    pub path: String,
    /// Git上の最終更新からの経過日数
    pub days_since_last_commit: u32,
    /// リポジトリ内の他ファイルから参照されているか(import/require/リンク等)
    pub is_referenced: bool,
    /// 本番サーバーのアクセスログ上でアクセスされた記録があるか(無い場合はNone=未計測)
    pub was_accessed_in_production: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletionRecommendation {
    /// 削除候補として妥当そう(参照無し・長期未更新等)
    LikelySafeToDelete,
    /// 判断材料が割れている、または情報不足(要作業者確認)
    NeedsHumanReview,
    /// 参照されている・最近更新されている等、削除すべきでない
    KeepFile,
}

#[derive(Debug, Clone)]
pub struct Advice {
    pub recommendation: DeletionRecommendation,
    /// 判断根拠(作業者向けの説明、日本語)
    pub reasons: Vec<String>,
}

const STALE_THRESHOLD_DAYS: u32 = 180;

/// ヒューリスティックによる暫定判定。実運用ではこの結果をプロンプトの
/// 一部としてaruaru-llmに渡し、自然言語の所見を生成させる想定
/// (aruaru-llm呼び出し自体は未接続、次回実装)。
pub fn advise(signals: &FileSignals) -> Advice {
    let mut reasons = Vec::new();

    if signals.is_referenced {
        reasons.push(format!(
            "{} は他ファイルから参照されているため保持を推奨",
            signals.path
        ));
        return Advice {
            recommendation: DeletionRecommendation::KeepFile,
            reasons,
        };
    }

    if signals.was_accessed_in_production == Some(true) {
        reasons.push(format!(
            "{} は本番サーバーで実際にアクセスされた記録があるため保持を推奨",
            signals.path
        ));
        return Advice {
            recommendation: DeletionRecommendation::KeepFile,
            reasons,
        };
    }

    let is_stale = signals.days_since_last_commit >= STALE_THRESHOLD_DAYS;
    let access_unknown = signals.was_accessed_in_production.is_none();

    if is_stale && !access_unknown {
        reasons.push(format!(
            "{} は参照なし・{}日間未更新・本番アクセス記録なしのため削除候補",
            signals.path, signals.days_since_last_commit
        ));
        return Advice {
            recommendation: DeletionRecommendation::LikelySafeToDelete,
            reasons,
        };
    }

    reasons.push(format!(
        "{} は参照は無いが、{}",
        signals.path,
        if access_unknown {
            "本番アクセスログが未計測のため判断材料不足".to_string()
        } else {
            format!(
                "最終更新から{}日と閾値({}日)未満のため様子見",
                signals.days_since_last_commit, STALE_THRESHOLD_DAYS
            )
        }
    ));
    Advice {
        recommendation: DeletionRecommendation::NeedsHumanReview,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referenced_file_is_kept() {
        let signals = FileSignals {
            path: "src/utils.rs".to_string(),
            days_since_last_commit: 400,
            is_referenced: true,
            was_accessed_in_production: None,
        };
        let advice = advise(&signals);
        assert_eq!(advice.recommendation, DeletionRecommendation::KeepFile);
    }

    #[test]
    fn accessed_in_production_is_kept_even_if_unreferenced() {
        let signals = FileSignals {
            path: "legacy_endpoint.php".to_string(),
            days_since_last_commit: 900,
            is_referenced: false,
            was_accessed_in_production: Some(true),
        };
        let advice = advise(&signals);
        assert_eq!(advice.recommendation, DeletionRecommendation::KeepFile);
    }

    #[test]
    fn unreferenced_stale_and_unaccessed_is_likely_safe_to_delete() {
        let signals = FileSignals {
            path: "old_backup.bak".to_string(),
            days_since_last_commit: 400,
            is_referenced: false,
            was_accessed_in_production: Some(false),
        };
        let advice = advise(&signals);
        assert_eq!(
            advice.recommendation,
            DeletionRecommendation::LikelySafeToDelete
        );
    }

    #[test]
    fn unreferenced_but_recently_modified_needs_human_review() {
        let signals = FileSignals {
            path: "wip_feature.rs".to_string(),
            days_since_last_commit: 5,
            is_referenced: false,
            was_accessed_in_production: Some(false),
        };
        let advice = advise(&signals);
        assert_eq!(
            advice.recommendation,
            DeletionRecommendation::NeedsHumanReview
        );
    }

    #[test]
    fn unreferenced_stale_with_unknown_access_needs_human_review() {
        let signals = FileSignals {
            path: "mystery_file.dat".to_string(),
            days_since_last_commit: 500,
            is_referenced: false,
            was_accessed_in_production: None,
        };
        let advice = advise(&signals);
        assert_eq!(
            advice.recommendation,
            DeletionRecommendation::NeedsHumanReview
        );
    }
}
