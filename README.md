# sftp-git

📖 English: [README-English.md](README-English.md)

WEBサイト開発者向けに、**SFTPアップロードとGit管理**を統合し、
**バージョンレスAPI**と**バージョン管理**のハイブリッド運用で、
テストサーバーと本番環境の差分チェック・デプロイ作業の手間とミスを
減らすことを目指すツール。5つの中核機能(ドリフト検出・
VersionlessAPIハイブリッド・不要ファイルAI削除判定・DUAL DATABASE・
AI差分解析)のロジック実装+実サーバーでのE2E検証まで完了(2026-08-15)。
LSPサーバー化・VS Code拡張(Node製薄層)も実装・実機起動確認済み。

参考にした既存サービス: https://lp.smartrelease.cloud/

## 配布形態(ユーザー指定、2026-08-15)

- **VS Code拡張機能**として開発する。
- 加えて、**Windows/Mac/Linux**用のネイティブアプリと、
  **Android(スマホ・タブレット)/iPhone/iPad**用のモバイルアプリも
  開発する。
- 現時点では方針のみで、VS Code拡張機能とネイティブ/モバイルアプリの
  コードをどこまで共有するか(ロジックを共通ライブラリ化しUIだけ
  各プラットフォームで作る、等)は未設計。詳細は[DESIGN.md](DESIGN.md)
  「7. マルチプラットフォーム配布」節参照。
- **開発ツール連携(ユーザー指示、2026-08-15)**: LSPサーバーは標準の
  Language Server Protocolに準拠しているため、VS Code以外の任意の
  LSPクライアントからも同じRust製バイナリで連携可能(Claude Code等)。
- **複数AIプロバイダ対応(ユーザー指示、2026-08-15)**: AI差分解析の
  バックエンドとして、aruaru-llmに加えClaude(Anthropic)・
  ChatGPT(OpenAI)・Gemini(Google)・DeepSeek・Grok(xAI)を選択可能に
  した(`src/ai_providers.rs`)。各社APIキーは環境変数で設定
  (未設定時はそのプロバイダを使わず正直にエラーを返す)。

## 目指す機能(構想)

- **Git管理とSFTP UPLOADのハイブリッド**: Gitで変更履歴を追いつつ、
  従来のSFTP運用に慣れた開発者にも扱えるアップロード導線を両立する。
- **バージョンレス×バージョン管理のハイブリッド**: VersionlessAPIの
  身軽さと、厳密なバージョン管理の安全性の良いとこ取りを狙う。
- **テスト/本番差分のAIチェック**: 差分検知・レビューに
  [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)を利用する構想。
  差分解析そのものの推論を高速化するため、
  [open-cuda](https://github.com/aon-co-jp/open-cuda)(NVIDIA GPU)と
  DirectX互換の[open-directx](https://github.com/aon-co-jp/open-directx)
  (CUDA非対応環境向け)をaruaru-llmのHWアクセラレーションバックエンド
  として使い分ける。加えて、検出した差分について**日本語・英語
  両方**でAIによる改善アドバイスを生成する機能を持たせる。
- **不要ファイルのAI削除判定支援**: Git+VersionlessAPIで管理する
  古いソース・使われなくなったファイルについて、削除してよいか
  [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)が判断材料
  (参照されているか、最終更新からの経過、コミット履歴上の扱われ方等)
  を示して作業者の判断を支援する。**最終的な削除実行は作業者の承認を
  必須とし、AIが自動で削除する設計にはしない**(誤削除リスクのため)。
- **フルバックアップ**: 基本は[aruaru-db](https://github.com/aon-co-jp/aruaru-db)。
  希望に応じてPostgreSQLも追加し、**DUAL DATABASE構成**(片側に
  不具合が発生してももう片側がフル補完・サポートする冗長構成)を
  選択可能にする。Rust+[RPoem](https://github.com/aon-co-jp/RPoem)
  で機能追加。
- **ハードウェアアクセラレーション(用途別に使い分け)**:
  - 暗号化・圧縮・展開の高速化には
    [open-raid-z](https://github.com/aon-co-jp/open-raid-z)の
    `open_raid_z_core`(GPU/D3D12対応、
    [RS-SmartTCP](https://github.com/aon-co-jp/RS-SmartTCP)が実際に
    利用中)を使う。調査の結果、この用途には
    [open-directx](https://github.com/aon-co-jp/open-directx)・
    [open-cuda](https://github.com/aon-co-jp/open-cuda)は合わないと
    判明したため対象外。
  - AI差分解析・アドバイス生成(aruaru-llmの推論)の高速化には
    [open-cuda](https://github.com/aon-co-jp/open-cuda)(NVIDIA GPU)と
    [open-directx](https://github.com/aon-co-jp/open-directx)(DirectX
    互換、CUDA非対応環境向け)を使い分ける。こちらは元々LLM推論向けの
    実装であり、用途に合致する。

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

## 現在の状態(2026-08-15時点)

**5機能のロジック実装+実サーバーでのE2E検証が完了**:

| 機能 | 実装 | 実サーバーE2E |
|---|---|---|
| SFTPドリフト検出 | `src/drift.rs`+`src/sftp_client.rs` | ✅ ローカルOpenSSH Serverで成功 |
| VersionlessAPIハイブリッド | `src/versionless_api.rs` | (ロジックのみ、RPoem実依存は未着手) |
| 不要ファイルAI削除判定支援 | `src/cleanup_advisor.rs` | (ロジックのみ、実削除はしない設計) |
| DUAL DATABASE | `src/dual_database.rs`+`src/dual_database_client.rs` | ✅ ローカルaruaru-serverで書き込み成功 |
| AI差分解析(日英) | `src/ai_diff_advisor.rs`+`src/aruaru_llm_client.rs` | ✅ 接続成功だがモデル品質は実用に耐えず(下記参照) |

**LSPサーバー化+VS Code拡張**: `src/bin/sftp_git_lsp.rs`(5機能全てを
カスタムリクエストとして公開)+`vscode-extension/`(Node製の薄い
接続層のみ、rust-analyzer方式)。**5機能全てにコマンドパレット経由の
UI導線を実装済み**(2026-08-15、`sftpGit.cleanupAdvice`/
`sftpGit.detectDrift`/`sftpGit.versionlessResolve`/`sftpGit.analyzeDiff`/
`sftpGit.dualDatabaseState`系3コマンド)。VS Code拡張ホストの実起動→LSP
サーバーの子プロセスspawn→正常終了まで確認済み。

**重要な発見(正直な開示)**: 小型GPT-2系モデル(`distilgpt2`/
`gpt2-medium`)は指示追従非対応のため、フォーマット指定プロンプトに
従えない。`ai_diff_advisor`は日英別プロンプトで2回呼び出す方式へ
設計変更済み。

**複数AIプロバイダ対応**: aruaru-llmに加え、Claude/ChatGPT/Gemini/
DeepSeek/Grokを選択可能にするクライアント(`src/ai_providers.rs`)を
実装。**実APIキーでの疎通確認は未実施**(このセッションにはいずれの
有効なキーも無かったため)。

詳細な実装ログ・実測結果はCLAUDE.mdのHANDOFF節を参照。

**RPoem実依存**: `src/versionless_api.rs`にRPoem
(`open-runo-versionless-api`、path依存)の`CompatibilityRule`/
`apply_compatibility`を使った`json_registry::JsonVersionRegistry`を
実装済み。

## 未着手・次の課題

- 複数AIプロバイダの実HTTP疎通確認(有効なAPIキーが必要)
- Windows/Mac/Linuxデスクトップアプリ・Android/iOSモバイルアプリ
  (着手順序: VS Code拡張→デスクトップ→モバイル、DESIGN.md参照)
- VS Code拡張のUI(現状は`showInputBox`によるJSON貼り付け形式、
  もう少しリッチなUI〈Webview等〉への発展は今後の検討課題)

## 関連リポジトリ

- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — AI差分チェック連携先候補
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — フルバックアップ連携先候補
- [RPoem](https://github.com/aon-co-jp/RPoem) — Rust機能追加の連携先候補
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — HWアクセラレーション暗号化・圧縮の本命候補(`open_raid_z_core`)
- 全体索引: [RUNO](https://github.com/aon-co-jp/RUNO)
