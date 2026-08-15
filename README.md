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
- **不要ファイルのAI削除判定支援**: Git+VersionlessAPIで管理する
  古いソース・使われなくなったファイルについて、削除してよいか
  [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)が判断材料
  (参照されているか、最終更新からの経過、コミット履歴上の扱われ方等)
  を示して作業者の判断を支援する。**最終的な削除実行は作業者の承認を
  必須とし、AIが自動で削除する設計にはしない**(誤削除リスクのため)。
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

## 市場調査(2026-08-15)

日英で既存事例を調査した結果、以下が分かった。

- **SFTP×Git連携ツール**: DeployHQ・GitFTP-Deploy・Git FTP Sync
  (GitHub Actions)・AutoDeployToFTP・FTPbucket等、
  「git push→SFTP自動反映」という**片方向**の自動化ツールは複数存在する。
  しかし**SFTP側を手動編集した際にGitリポジトリとの乖離(ドリフト)を
  検出する逆方向の照合機能**を持つ専用ツールは見当たらなかった。
  → sftp-gitの差別化ポイントになり得る。
- **VersionlessAPI**: Stripeが2024-09以降採用している方式(アカウント
  単位の日付ベースAPIバージョン+リクエスト単位のヘッダー上書き、
  内部は現行実装+過去バージョンへの変換トランスフォーマー)が代表例。
  GitHub API(URI版+メディアタイプ併用)のような「破壊的変更のみ
  明示バージョン、それ以外は無破壊進化」というハイブリッド運用の
  先行事例はあるが、「versionless×versioned」を明示的に名付けた
  汎用設計パターンの文献は見当たらなかった。
- **ステージング/本番差分検出**: インフラのドリフト検出ツール
  `driftctl`(Snyk買収)は2023年にEOL済みで事実上放棄状態。
  AI活用のステージング/本番差分専用レビューツールは今回の調査では
  具体名を特定できなかった。

実現方式レベルの検討は[DESIGN.md](DESIGN.md)にまとめている。

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
