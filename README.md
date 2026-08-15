# sftp-git

WEBサイト開発者向けに、**SFTPアップロードとGit管理**を統合し、
**バージョンレスAPI**と**バージョン管理**のハイブリッド運用で、
テストサーバーと本番環境の差分チェック・デプロイ作業の手間とミスを
減らすことを目指すツール(構想段階、コード未実装)。

参考にした既存サービス: https://lp.smartrelease.cloud/

## 目指す機能(構想)

- **Git管理とSFTP UPLOADのハイブリッド**: Gitで変更履歴を追いつつ、
  従来のSFTP運用に慣れた開発者にも扱えるアップロード導線を両立する。
- **バージョンレス×バージョン管理のハイブリッド**: VersionlessAPIの
  身軽さと、厳密なバージョン管理の安全性の良いとこ取りを狙う。
- **テスト/本番差分のAIチェック**: 差分検知・レビューに
  [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)を利用する構想。
- **フルバックアップ**: [aruaru-db](https://github.com/aon-co-jp/aruaru-db)
  +PostgreSQLをベースに、Rust+[RPoem](https://github.com/aon-co-jp/RPoem)
  で機能追加。
- **ハードウェアアクセラレーション**: 調査の結果、暗号化・圧縮・展開の
  高速化には[open-raid-z](https://github.com/aon-co-jp/open-raid-z)の
  `open_raid_z_core`(GPU/D3D12対応、[RS-SmartTCP](https://github.com/aon-co-jp/RS-SmartTCP)
  が実際に利用中)が本命候補。当初想定していた
  [open-directx](https://github.com/aon-co-jp/open-directx)(2D描画)・
  [open-cuda](https://github.com/aon-co-jp/open-cuda)(LLM推論専用)は
  この用途には合わないと判明。

## 現在の状態

構想段階。Cargoパッケージの雛形のみで、実装はまだ無い。
上記の各連携先(aruaru-llm/aruaru-db/open-directx/open-cuda)は
それぞれ独立したリポジトリであり、実際の連携方式は未調査・未設計。

## 関連リポジトリ

- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — AI差分チェック連携先候補
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — フルバックアップ連携先候補
- [RPoem](https://github.com/aon-co-jp/RPoem) — Rust機能追加の連携先候補
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — HWアクセラレーション暗号化・圧縮の本命候補(`open_raid_z_core`)
- 全体索引: [RUNO](https://github.com/aon-co-jp/RUNO)
