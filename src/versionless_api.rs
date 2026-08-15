//! VersionlessAPI×バージョン管理のハイブリッド。DESIGN.md「2.」参照。
//!
//! 2026-08-15、RPoem(https://github.com/aon-co-jp/RPoem)の
//! `open-runo-versionless-api`crateへ実依存を追加した(path依存)。
//! RPoem側はフィールド単位の互換性ルール
//! (`CompatibilityRule::RenamedField`/`RemovedFieldDefault`/
//! `Deprecated`)を`serde_json::Value`に適用する`apply_compatibility`を
//! 提供する——これがStripe方式(現行の正規実装+過去バージョンへの
//! 変換)の実体。本モジュールの`VersionRegistry<T>`(型付きバージョン別
//! 変換関数の登録)は、内部でRPoemの`apply_compatibility`を呼ぶ
//! `json_registry`サブモジュールと併用する想定。

use std::collections::BTreeMap;

/// RPoemの`open-runo-versionless-api`を使った、JSON値ベースの
/// バージョン変換レジストリ。フィールドのリネーム・削除時デフォルト
/// 値補完・非推奨マーキングという3種類の互換性ルールを、バージョン
/// ごとに登録する。
///
/// **RustJSON(`open-runo-rustjson`/`RS-JSON`)との関係**: RustJSON自体は
/// 「値モデルとして`serde_json::Value`をそのまま採用する寛容パーサー」
/// という設計(RustJSON側のdocコメントに明記)であり、`serde_json::Value`
/// を置き換える別の型ではない。本モジュールは外部文字列を直接パースする
/// 箇所を持たないため`serde_json::json!`で値を組み立てているが、
/// **将来HTTPリクエストボディ等、外部から受け取ったJSON文字列を
/// パースする箇所を追加する場合は、`serde_json::from_str`ではなく
/// `open_runo_rustjson::parse`を使うこと**(このエコシステム全体の
/// 既定ルール、RPoem CLAUDE.md参照)。
pub mod json_registry {
    use open_runo_versionless_api::{apply_compatibility, CompatibilityRule};
    use std::collections::BTreeMap;

    pub struct JsonVersionRegistry {
        rules: BTreeMap<String, Vec<CompatibilityRule>>,
    }

    impl JsonVersionRegistry {
        pub fn new() -> Self {
            Self {
                rules: BTreeMap::new(),
            }
        }

        /// 指定バージョン向けの互換性ルール群を登録する。
        pub fn register(&mut self, version: impl Into<String>, rules: Vec<CompatibilityRule>) {
            self.rules.insert(version.into(), rules);
        }

        /// 現行スキーマのJSON値を、リクエストされたバージョン向けへ
        /// 変換する。該当バージョンの登録が無ければ無変換のまま返す
        /// (versionless、変換不要という意味)。
        pub fn resolve(
            &self,
            current: serde_json::Value,
            requested_version: Option<&str>,
        ) -> serde_json::Value {
            match requested_version.and_then(|v| self.rules.get(v)) {
                Some(rules) => apply_compatibility(current, rules),
                None => current,
            }
        }
    }

    impl Default for JsonVersionRegistry {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn no_version_requested_returns_current_shape_unchanged() {
            let registry = JsonVersionRegistry::new();
            let current = json!({ "user_id": "u_1" });
            assert_eq!(registry.resolve(current.clone(), None), current);
        }

        #[test]
        fn renames_field_back_for_a_registered_legacy_version() {
            let mut registry = JsonVersionRegistry::new();
            registry.register(
                "2020-01-01",
                vec![CompatibilityRule::RenamedField {
                    old_name: "userId".to_string(),
                    new_name: "user_id".to_string(),
                }],
            );
            let current = json!({ "user_id": "u_1" });
            let legacy = registry.resolve(current, Some("2020-01-01"));
            assert_eq!(legacy["userId"], "u_1");
        }

        #[test]
        fn unregistered_version_falls_back_to_current_shape() {
            let registry = JsonVersionRegistry::new();
            let current = json!({ "user_id": "u_1" });
            assert_eq!(
                registry.resolve(current.clone(), Some("1999-01-01")),
                current
            );
        }
    }
}

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
