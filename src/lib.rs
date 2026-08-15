//! sftp-git: SFTP UPLOADとGit管理を統合するハイブリッド運用ツール
//!
//! 詳細な構想・要件はREADME.md/DESIGN.md/CLAUDE.mdを参照。
//! 各機能は独立したモジュールとして1つずつ実装・テストしていく方針。

pub mod ai_diff_advisor;
pub mod aruaru_llm_client;
pub mod cleanup_advisor;
pub mod drift;
pub mod dual_database;
pub mod dual_database_client;
pub mod sftp_client;
pub mod versionless_api;
