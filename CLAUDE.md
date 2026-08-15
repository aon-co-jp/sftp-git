# 開発方針＆開発環境ルール(sftp-git)

このリポジトリは構想段階(2026-08-15作成)。README.mdに記載した
構想(SFTP×Git統合、バージョンレス/バージョン管理ハイブリッド、
aruaru-llmによるテスト/本番差分AIチェック、aruaru-db+PostgreSQLへの
フルバックアップ、open-directx/open-cudaによるHWアクセラレーション
暗号化・圧縮)は、いずれも他リポジトリとの連携方式・実現可能性が
未調査。実装を始める前に、各連携先リポジトリの現状(APIの有無、
実装状況)を確認すること。

作業ドライブは`F:\runo`(全体の作業ドライブ移行状況は
[RUNO](https://github.com/aon-co-jp/RUNO)のCLAUDE.md参照)。

## HANDOFF

- **2026-08-15 新規作成**: ユーザー指示によりGitHub
  `aon-co-jp/sftp-git`として新規作成。リポジトリ名の変遷は
  当初`open-smart-relrase-u`(スペルミス)→ユーザー訂正で
  `open-smart-release-u`→ユーザー指示で最終的に`sftp-git`に改名。
  Cargo雛形(`Cargo.toml`/`src/lib.rs`)と
  README.md/CLAUDE.md/PORTING.mdのみ作成し、実装は未着手。
  - 次にすべきこと: (1) aruaru-llmの既存APIでテスト/本番差分チェックが
    どこまで転用できるか調査。(2) SFTP×Git統合の既存OSS実装の
    有無を日英で検索。(3) VersionlessAPIとバージョン管理を両立させる
    設計パターンの調査。(4) open-directx(空リポジトリ、RUNOの
    CLAUDE.md参照)の状況を踏まえ、HWアクセラレーション連携が
    現実的かどうか判断。
