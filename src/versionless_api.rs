//! VersionlessAPI×バージョン管理のハイブリッド。DESIGN.md「2.」参照。
//!
//! ゼロから実装せず、RPoem(https://github.com/aon-co-jp/RPoem)が
//! 既に持つVersionlessAPI実装(Stripe方式: 現行の正規スキーマ+過去
//! バージョンへの変換トランスフォーマー)の考え方を土台にする。
//! ここでは、sftp-git側で必要な「バージョン別レスポンス変換」の
//! 最小限のフレームワーク部分のみを実装する(RPoem本体への実依存は
//! 未接続、次回精査)。

use std::collections::BTreeMap;

/// 日付ベースのAPIバージョン(例: "2026-08-15")。
pub type ApiVersion = String;

/// 現行(最新)スキーマの値から、指定バージョン向けの値へ変換する関数。
pub type Transformer<T> = fn(&T) -> T;

/// バージョンごとの変換関数を保持するレジストリ。
/// 破壊的変更をした回数分だけ変換関数を登録すればよく、
/// バージョンの組み合わせ数だけ増える設計を避ける。
pub struct VersionRegistry<T> {
    transformers: BTreeMap<ApiVersion, Transformer<T>>,
}

impl<T: Clone> VersionRegistry<T> {
    pub fn new() -> Self {
        Self {
            transformers: BTreeMap::new(),
        }
    }

    /// 指定バージョン向けの変換関数を登録する。
    pub fn register(&mut self, version: impl Into<ApiVersion>, transformer: Transformer<T>) {
        self.transformers.insert(version.into(), transformer);
    }

    /// 現行スキーマの値を、リクエストされたバージョン向けに変換する。
    /// 該当バージョンの変換関数が無ければ現行スキーマのまま返す
    /// (=versionless、変換不要という意味)。
    pub fn resolve(&self, current: &T, requested_version: Option<&str>) -> T {
        match requested_version {
            Some(v) => match self.transformers.get(v) {
                Some(transform) => transform(current),
                None => current.clone(),
            },
            None => current.clone(),
        }
    }
}

impl<T: Clone> Default for VersionRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct UserV2026 {
        full_name: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct LegacyUser {
        first_name: String,
    }

    #[test]
    fn returns_current_schema_when_no_version_requested() {
        let registry: VersionRegistry<UserV2026> = VersionRegistry::new();
        let current = UserV2026 {
            full_name: "Taro Yamada".to_string(),
        };
        assert_eq!(registry.resolve(&current, None), current);
    }

    #[test]
    fn returns_current_schema_when_no_transformer_registered_for_version() {
        let registry: VersionRegistry<UserV2026> = VersionRegistry::new();
        let current = UserV2026 {
            full_name: "Taro Yamada".to_string(),
        };
        assert_eq!(registry.resolve(&current, Some("2020-01-01")), current);
    }

    #[test]
    fn applies_transformer_for_registered_legacy_version() {
        fn to_legacy(current: &UserV2026) -> UserV2026 {
            let first = current
                .full_name
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            UserV2026 { full_name: first }
        }

        let mut registry: VersionRegistry<UserV2026> = VersionRegistry::new();
        registry.register("2020-01-01", to_legacy);

        let current = UserV2026 {
            full_name: "Taro Yamada".to_string(),
        };
        let legacy = registry.resolve(&current, Some("2020-01-01"));
        assert_eq!(
            legacy,
            UserV2026 {
                full_name: "Taro".to_string()
            }
        );
    }

    #[test]
    fn legacy_shape_example_shows_intended_usage() {
        // 実運用ではLegacyUserのような別型へ変換するのが本来の姿だが、
        // Transformer<T>はT->Tのため、ここではドキュメント的な例のみ示す。
        let legacy = LegacyUser {
            first_name: "Taro".to_string(),
        };
        assert_eq!(legacy.first_name, "Taro");
    }
}
