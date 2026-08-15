//! SFTPドリフト検出: Gitマニフェスト(ファイルパス→ハッシュ)と、
//! 本番サーバー側から読み出した実ファイルのハッシュを突き合わせ、
//! 乖離(ドリフト)を検出する。DESIGN.md「1. SFTPドリフト検出」参照。
//!
//! 削除実行や上書きは行わない(判定のみ)。作業者が結果を見て
//! 「Gitへ取り込む」か「本番側を上書きする」かを選ぶ設計。

use std::collections::BTreeMap;

pub type Manifest = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftKind {
    /// 本番にはあるがGitマニフェストに無い(本番側で追加された)
    OnlyOnServer,
    /// Gitマニフェストにはあるが本番に無い(本番側で削除された)
    OnlyInGit,
    /// 両方にあるがハッシュが一致しない(本番側で内容が変更された)
    HashMismatch { git_hash: String, server_hash: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftEntry {
    pub path: String,
    pub kind: DriftKind,
}

/// GitマニフェストとサーバーマニフェストからDriftを検出する。
/// 両方に同じハッシュで存在するファイルはDriftに含めない。
pub fn detect_drift(git_manifest: &Manifest, server_manifest: &Manifest) -> Vec<DriftEntry> {
    let mut drifts = Vec::new();

    for (path, git_hash) in git_manifest {
        match server_manifest.get(path) {
            None => drifts.push(DriftEntry {
                path: path.clone(),
                kind: DriftKind::OnlyInGit,
            }),
            Some(server_hash) if server_hash != git_hash => drifts.push(DriftEntry {
                path: path.clone(),
                kind: DriftKind::HashMismatch {
                    git_hash: git_hash.clone(),
                    server_hash: server_hash.clone(),
                },
            }),
            Some(_) => {}
        }
    }

    for path in server_manifest.keys() {
        if !git_manifest.contains_key(path) {
            drifts.push(DriftEntry {
                path: path.clone(),
                kind: DriftKind::OnlyOnServer,
            });
        }
    }

    drifts.sort_by(|a, b| a.path.cmp(&b.path));
    drifts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(entries: &[(&str, &str)]) -> Manifest {
        entries
            .iter()
            .map(|(p, h)| (p.to_string(), h.to_string()))
            .collect()
    }

    #[test]
    fn identical_manifests_have_no_drift() {
        let git = manifest(&[("index.html", "abc"), ("style.css", "def")]);
        let server = git.clone();
        assert!(detect_drift(&git, &server).is_empty());
    }

    #[test]
    fn detects_file_added_only_on_server() {
        let git = manifest(&[("index.html", "abc")]);
        let server = manifest(&[("index.html", "abc"), ("uploaded_by_hand.php", "xyz")]);
        let drifts = detect_drift(&git, &server);
        assert_eq!(
            drifts,
            vec![DriftEntry {
                path: "uploaded_by_hand.php".to_string(),
                kind: DriftKind::OnlyOnServer,
            }]
        );
    }

    #[test]
    fn detects_file_missing_on_server() {
        let git = manifest(&[("index.html", "abc"), ("removed.js", "def")]);
        let server = manifest(&[("index.html", "abc")]);
        let drifts = detect_drift(&git, &server);
        assert_eq!(
            drifts,
            vec![DriftEntry {
                path: "removed.js".to_string(),
                kind: DriftKind::OnlyInGit,
            }]
        );
    }

    #[test]
    fn detects_hash_mismatch_when_content_changed_on_server() {
        let git = manifest(&[("index.html", "abc")]);
        let server = manifest(&[("index.html", "modified_by_hand")]);
        let drifts = detect_drift(&git, &server);
        assert_eq!(
            drifts,
            vec![DriftEntry {
                path: "index.html".to_string(),
                kind: DriftKind::HashMismatch {
                    git_hash: "abc".to_string(),
                    server_hash: "modified_by_hand".to_string(),
                },
            }]
        );
    }

    #[test]
    fn detects_multiple_drifts_sorted_by_path() {
        let git = manifest(&[("b.txt", "1"), ("a.txt", "2")]);
        let server = manifest(&[("b.txt", "1"), ("a.txt", "changed"), ("c.txt", "new")]);
        let drifts = detect_drift(&git, &server);
        let paths: Vec<&str> = drifts.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(paths, vec!["a.txt", "c.txt"]);
    }
}
