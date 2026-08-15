# PORTING (sftp-git)

📖 English: [PORTING-English.md](PORTING-English.md)

移植元・移植先の情報なし(新規構想のため既存ツールからの移行元は無い)。
将来、既存のSFTPクライアント/Git連携ツール(DeployHQ・GitFTP-Deploy等、
README.md「市場調査」節参照)からの移行パスが具体化した場合にここへ
記載する。

## 他プロジェクトからこのリポジトリの機能を利用する場合

このリポジトリ自体は他プロジェクトへ「移植」する対象ではなく、以下の
Rustモジュールを`path`依存として個別に再利用できる設計にしている
(各モジュールはリポジトリ内で完結し、`sftp_git`クレート全体への
依存を要求しない)。

- `sftp_git::drift` — Gitマニフェストと実サーバーのハッシュ突き合わせ
  ロジックのみが必要な場合(SFTP以外のプロトコルでも転用可能な形)。
- `sftp_git::dual_database` / `dual_database_client` — aruaru-db
  (PostgreSQLワイヤー互換)+PostgreSQLの冗長構成が必要な他プロジェクト
  でそのまま再利用できる。
- `sftp_git::ai_providers` — Claude/ChatGPT/Gemini/DeepSeek/Grokへの
  統一クライアントが必要な他プロジェクトでの再利用を想定
  (実APIキーでの疎通確認は未実施、CLAUDE.md参照)。

## 依存先リポジトリ(sibling path依存)

このリポジトリ自身が以下をpath依存として利用する想定(現時点では
`aruaru-llm`のHTTPクライアントのみ実配線、他は構想段階):

- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — HTTP経由
  (`src/aruaru_llm_client.rs`、path依存ではなくHTTP API利用)
- [RPoem](https://github.com/aon-co-jp/RPoem) — VersionlessAPI実装の
  再利用を想定(未着手)
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) —
  `open_raid_z_core`によるHWアクセラレーション暗号化・圧縮(未着手)
