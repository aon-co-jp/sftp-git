# 開発方針＆開発環境ルール(sftp-git)

📖 English: [CLAUDE-English.md](CLAUDE-English.md)

2026-08-15作成。5つの中核機能(ドリフト検出・VersionlessAPIハイブリッド・
不要ファイルAI削除判定支援・DUAL DATABASE・AI差分解析)のロジック実装+
実サーバーでのE2E検証が完了。LSPサーバー化(`sftp_git_lsp`)+VS Code
拡張(Node製薄層のみ、rust-analyzer方式)も実装・実機起動確認済み。
複数AIプロバイダ対応(Claude/ChatGPT/Gemini/DeepSeek/Grok)クライアントも
実装済みだが実APIキーでの疎通確認は未実施。現状のまとめは
README.md「現在の状態」節、実装ログの詳細は本ファイルのHANDOFF節を
参照。

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

- **2026-08-15(続き) 連携先の現状調査**: 各連携候補の実態を確認した。
  - `aruaru-llm`: `/v1/generate`・`/v1/chat`・`/v1/classify-security`等の
    汎用LLM APIはあるが、テスト/本番差分チェック専用のエンドポイントは
    無い。差分テキストを`/v1/generate`や`/v1/chat`に投げて分析させる
    自前実装が必要。
  - `open-cuda`: GPT-2推論用のGPUアクセラレーションであり、暗号化・
    圧縮用途ではない。
  - `open-directx`: 2Dスプライト描画のVulkan/DirectX互換プロトタイプ
    (実装が進行中、RUNOのCLAUDE.mdにある「空リポジトリ」という記載は
    2026-07-23時点の古い情報だった)。暗号化・圧縮とは無関係。
  - `open-raid-z`の`open_raid_z_core`: GPU(D3D12/DirectML)対応の
    圧縮・暗号化コア(`RS-SmartTCP`が実際に依存して利用中)。
    **HWアクセラレーション暗号化・圧縮の本命はこちら**であり、
    open-directx/open-cudaではない。
  - 次にすべきこと: `open_raid_z_core`をsftp-gitのバックアップ暗号化・
    圧縮パイプラインにどう組み込むか設計する。SFTP×Git統合の
    既存OSS実装調査とVersionlessAPI設計パターン調査は未着手のまま。

- **2026-08-15(続き) 市場調査+機能追加**: SFTP×Git統合・
  VersionlessAPI・AI差分チェックについて日英で調査(詳細は
  README.mdの「市場調査」節)。SFTP手動編集によるGitとのドリフト
  検出は既存ツールに空白領域と判明、差別化ポイントとして記録。
  また、ユーザー提案により「Git+VersionlessAPIで管理する古い/不要
  ファイルをaruaru-llmが削除すべきか判断し作業者を支援する」機能を
  構想に追加(**最終削除は作業者承認必須、AI自動削除はしない**設計と
  明記——誤削除防止のため)。
  - 次にすべきこと: (1) ドリフト検出(SFTP実ファイル vs Git管理状態の
    差分検知)の具体的な実装方式を設計。(2) 不要ファイルAI判定機能の
    判断基準(参照有無・最終更新・コミット履歴)を具体化。(3)
    open_raid_z_coreの組み込み設計は未着手のまま。

- **2026-08-15(続き) DESIGN.md新設**: ドリフト検出方式(ハッシュ
  マニフェスト突き合わせ、破棄/取り込みは作業者選択)・
  VersionlessAPIハイブリッド方式(Stripe方式を土台、変換関数で
  過去バージョン吸収)・不要ファイルAI削除判定の材料項目・
  open_raid_z_core組み込み(未着手)を[DESIGN.md](DESIGN.md)に整理。
  **未解決のまま残した重要な論点**: VersionlessAPIハイブリッドを
  「sftp-git自身のAPI」に適用するのか「利用者がデプロイするWeb
  サイト側API」の管理支援として提供するのかが未確定。次回はまず
  この利用シームを一つに絞り込むところから始めること。
  - 次にすべきこと: 上記の利用シーン確定を最優先。その後で
    ドリフト検出のマニフェスト形式のプロトタイプ実装に着手する。

- **2026-08-15(続き) フルバックアップ方針確定(ユーザー指定)**:
  基本は`aruaru-db`、希望に応じてPostgreSQLを追加した
  **DUAL DATABASE構成**(片側障害時にもう片側がフル補完)を選択可能に
  すると確定。詳細は[DESIGN.md](DESIGN.md)の「4. フルバックアップ」
  節参照。整合性モデル・再同期方式・書き込み層は未確定のまま残した。
  - 次にすべきこと: DUAL DATABASE時の整合性モデル(強整合/結果整合)を
    ユーザーに確認して確定させる。

- **2026-08-15(続き) 整合性モデル・書き込み層をハイブリッド方式で確定**:
  ユーザー指定により「ハイブリッドの良いとこ取り」を具体化。
  平常時はaruaru-db同期+PostgreSQL非同期(結果整合、低レイテンシ)、
  障害検知時はPostgreSQLを主系に昇格し同期確定(強整合、データ
  無損失優先)に切り替える設計。書き込み層はDBプロキシ層(ルーティング・
  死活監視・フェイルオーバー)とアプリ層(復旧後のコンフリクト解決等の
  業務ロジック)で役割分担。詳細は[DESIGN.md](DESIGN.md)「4.
  フルバックアップ」節参照。
  - 次にすべきこと: フェイルオーバー検知の具体的な閾値、差分取り込み
    アルゴリズム(タイムスタンプ比較 or WAL/変更ログ方式)の選定。
    `open-raid-z`の既存ディザスタバックアップ機能を転用できるか調査。

- **2026-08-15(続き) 整合性モデル確認+AI差分解析機能追加**:
  整合性モデル・書き込み層の役割分担について、ユーザーから前回記録
  内容の確認(内容としては変更なし)を受けた。また、ユーザー指定により
  「テスト/本番差分のAIチェック」機能を具体化: aruaru-llmの推論を
  open-cuda(NVIDIA GPU)/open-directx(DirectX互換、CUDA非対応環境
  向け)で高速化し、差分アドバイスを**日本語・英語両方**で生成する。
  **重要**: open-directx/open-cudaは3.のバックアップ暗号化・圧縮
  用途(`open_raid_z_core`が担当)には合わないと既に結論済みだが、
  今回追加したのは**別用途(LLM推論高速化)**であり矛盾しない——両者を
  混同しないようDESIGN.md「5. AI差分解析」節に明記した。
  - 次にすべきこと: 日英2言語出力をLLM1回呼び出しで済ませるか
    2回に分けるかのプロンプト設計。フェイルオーバー閾値・再同期
    アルゴリズムの選定は引き続き未着手。

- **2026-08-15(続き) マルチプラットフォーム配布方針を追加**:
  ユーザー指定により、VS Code拡張機能に加えてWindows/Mac/Linux
  ネイティブアプリ、Android(スマホ・タブレット)/iPhone/iPad
  モバイルアプリという**6プラットフォーム**を配布対象に追加。
  詳細は[DESIGN.md](DESIGN.md)「7. マルチプラットフォーム配布」節。
  **正直な懸念点として記録**: 現状Cargo雛形のみの骨組み段階から見て
  非常に大きなスコープであり、VS Code拡張(TS/JS)とRustコアの
  言語をまたぐ共有方法、モバイルでのSFTP/Git/DUAL DATABASE対応範囲、
  6プラットフォーム同時開発の着手順序はすべて未検討・未設計のまま
  DESIGN.mdに明記した。
  - 次にすべきこと: 6プラットフォームの着手順序(例: まずVS Code拡張
    機能で中核機能を検証してから展開する等)をユーザーと相談して
    決める。それまでは実装に着手せず設計整理を優先する。

- **2026-08-15(続き) 着手順序を確定**: ユーザー確定により
  「①VS Code拡張機能で中核機能検証 → ②Windows/Mac/Linuxデスクトップ
  展開 → ③Android/iOSモバイル展開」の3段階と決定。詳細は
  [DESIGN.md](DESIGN.md)「7. マルチプラットフォーム配布」節。
  - 次にすべきこと: 段階1(VS Code拡張機能)に入る前に、(a) VS Code
    拡張(TS/JS)とRustコアの共有方式(FFI/N-API等)の選定、(b)
    段階1の検証スコープ(DESIGN.md 1〜5節のうちどこまで含めるか)の
    絞り込み、の2点を詰める必要がある。実装コードはまだ一切無い
    (このリポジトリはCargo雛形+設計ドキュメントのみ)。

- **2026-08-15(続き) Node不使用方針への転換+5機能を実装・全テスト
  通過**: ユーザー指定「Rustは使うがNodeは使わずRPoemで実装」を受け、
  rust-analyzer方式(業務ロジックは全てRust製LSPサーバー、VS Code側は
  Node製の薄い接続コードのみ)に転換し、DESIGN.md「7.」を更新。
  RPoemの既存VersionlessAPI実装・Tauri互換レイヤーを再利用する方針も
  明記。その上で、ユーザー指定「5機能を一つずつ実装してはTESTを
  繰り返して完成させる」を実行し、以下をRustモジュールとして実装、
  `cargo test`で**全22件通過**を確認済み(いずれも実DB/実HTTP接続は
  未接続、ロジック部分のみ):
  - `src/drift.rs`(機能1: SFTPドリフト検出、5テスト)
  - `src/versionless_api.rs`(機能2: VersionlessAPIハイブリッド、
    バージョンレジストリ、4テスト)
  - `src/cleanup_advisor.rs`(機能3: 不要ファイルAI削除判定支援、
    ヒューリスティックスコアリング、5テスト)
  - `src/dual_database.rs`(機能4: DUAL DATABASE整合性モデルの
    状態遷移、4テスト)
  - `src/ai_diff_advisor.rs`(機能5: AI差分解析プロンプト組み立て+
    日英アドバイスのレスポンス分離、4テスト)
  作業中、`target/`ビルド成果物を誤ってコミットしてしまい`.gitignore`
  追加の上で取り消した(以後は`.gitignore`で防止済み)。
  - 次にすべきこと: (1) 各モジュールを実際のaruaru-llm HTTP API・
    実DB接続・実SFTP接続へ配線する(現状は純粋ロジックのみ)。
    (2) RPoemのVersionlessAPI実装への実依存(path依存等)を追加する。
    (3) VS Code拡張のLSPサーバー化・Node製接続コードの実装に着手する。

- **2026-08-15(続き) 実サービス配線+LSPサーバー化+VS Code拡張を実装**:
  ユーザー指示「実配線して、実サービスへ接続する作業を行なって。
  VS Code拡張のLSPサーバー化をして」を受けて以下を実装、
  `cargo build`/`cargo test`(24 passed, 3 ignored)・
  `npx tsc`(エラー無し)で確認済み:
  - `src/aruaru_llm_client.rs`: aruaru-llm `/v1/chat`への実HTTP
    クライアント(reqwest)。`ai_diff_advisor`のプロンプト組み立て/
    レスポンス解析と接続。実サーバー要のテストは`#[ignore]`
    (`ARUARU_LLM_BASE_URL`環境変数で手動実行)。
  - `src/sftp_client.rs`: 実SFTPクライアント(ssh2crate、パスワード
    認証)。`drift.rs`のマニフェスト突き合わせへ、本番サーバーの
    実ファイルをSHA-256ハッシュ化して渡す。実サーバー要のテストは
    `#[ignore]`。
  - `src/dual_database_client.rs`: DUAL DATABASE実接続
    (`tokio-postgres`)。**重要な発見**: aruaru-db(`aruaru-server`)は
    PostgreSQLワイヤープロトコル互換ポート(`--pg-port`)を持つため、
    aruaru-db/PostgreSQL両方に同じ`tokio-postgres`クライアントで
    接続できる。`dual_database.rs`の状態遷移(平常時同期整合/
    障害時フェイルオーバー)と組み合わせ、平常時は主系へ同期・
    従系へ非同期(`tokio::spawn`)で書き込む実装。
  - `src/bin/sftp_git_lsp.rs`: LSPサーバー本体(`lsp-server`/
    `lsp-types`crate)。標準入出力でVS Code拡張と通信。現状は
    カスタムリクエスト`sftpGit/cleanupAdvice`のみ配線(疎通確認優先、
    残り4機能のLSPリクエスト化は次回)。
  - `vscode-extension/`: Node製の薄い接続層のみ(`src/extension.ts`)。
    `sftp_git_lsp`バイナリを子プロセス起動し`vscode-languageclient`で
    中継するだけで、業務ロジックは一切持たない
    (方針通りrust-analyzer方式を実現)。
  - 追加した外部依存(実行時ネットワーク/DB接続が要るもの)は
    reqwest・tokio・tokio-postgres・ssh2・sha2・lsp-server・lsp-types。
  - 次にすべきこと: (1) 残り4機能(drift/versionless_api/
    cleanup_advisor全体/dual_database/ai_diff_advisor)をLSP
    カスタムリクエストとして追加配線。(2) VS Code拡張から実際に
    LSPサーバーを起動してエンドツーエンドで動作確認(現時点では
    ビルド・型チェックのみ、実機起動確認はまだ)。(3) RPoemの
    VersionlessAPI実装への実依存はまだ未着手。(4) aruaru-db/
    PostgreSQLへの実接続テストは実サーバーが無いため未実施
    (`#[ignore]`のまま)。

- **2026-08-15(続き) ローカルE2E疎通確認(ユーザー指示: 自分のPC内で
  実サーバーを立てて検証)**:
  - **LSP残り4機能を配線完了**: `sftpGit/detectDrift`・
    `sftpGit/versionlessResolve`(汎用型TはJSON越しに公開できないため、
    JSON値ベースのデモ変換レジストリとして実装、実運用ルールは次回
    拡張)・`sftpGit/analyzeDiff`+`buildDiffPrompt`・
    `sftpGit/dualDatabaseState`系3リクエストを追加し、
    `cargo build --bin sftp_git_lsp`で全5機能のビルド確認済み。
  - **DUAL DATABASE実E2E成功**: `F:\runo\aruaru-db`をローカルビルドし
    `aruaru-server.exe --pg-port 15432`で自PC内起動(データは一時
    ディレクトリ、`ARUARU_USERS`環境変数でSCRAM認証ユーザー設定)。
    **重要な発見**: aruaru-dbは`SELECT 1`のようなFROM句無しSELECTを
    サポートしない独自SQLパーサーのため、テストは`CREATE TABLE IF NOT
    EXISTS`に変更。`dual_database_client::tests::
    write_against_real_dual_database`が実際に自PC内aruaru-serverへの
    書き込みに成功(`cargo test -- --ignored`で確認)。
  - **aruaru-llm実E2E部分成功**: `F:\runo\aruaru-llm`をローカルビルド・
    自PC内起動(`distilgpt2`モデル使用中)。HTTP往復・JSON
    シリアライズは正常動作したが、**当初想定していた`/v1/chat`は
    意図分類ベースのFAQ応答用エンドポイントで自由記述生成には
    不適切**と判明し、`aruaru_llm_client.rs`を`/v1/generate`
    (`GenerateRequest{prompt, max_new_tokens}`)へ修正。修正後、
    HTTP接続・応答受信は成功したが、**`ai_diff_advisor`が要求する
    「日本語見出し+英語見出し」形式でのパースには失敗**——
    distilgpt2は指示追従(instruction following)非対応の素の
    小型モデルのため、フォーマット指定プロンプトに従えず自由連想的な
    テキストを返すのみだった(aruaru-llm自身も`disclosure`フィールドで
    「商用LLMとは比較にならない」と正直に開示している既知の限界と
    整合)。**正直な結論**: HTTP配線自体は正常だが、
    `ai_diff_advisor`の日英2セクション形式パースは現在の
    aruaru-llmモデル(小型GPT-2系)では実用に耐えない。次回、
    (a)フォーマット指定を諦めてモデル出力をそのまま日本語のみ/
    英語のみで別々に2回呼ぶ方式に変える、(b)より大きい・
    instruction-tunedなモデルへの切り替えをaruaru-llm側に依頼する、
    のいずれかを検討する必要がある。
  - **SFTP実E2Eは未完了**: ローカルにWindows OpenSSH Serverが
    無かったため、ユーザーに管理者権限での有効化
    (`Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0`)を
    依頼中。有効化され次第、`sftp_client.rs`の`#[ignore]`テストを
    `127.0.0.1`向けに実行する。
  - **VS Code拡張の実機起動確認: 成功**: `code --extensionDevelopmentPath=
    vscode-extension --new-window`で拡張開発ホストを実起動。
    `tasklist`で`sftp_git_lsp.exe`(release build)が拡張機能から
    子プロセスとして実際にspawnされ、安定稼働していることを確認
    (activate→バイナリパス解決→spawn→LanguageClient接続の一連が
    E2Eで動作)。VS Codeウィンドウ終了時にLSPサーバープロセスも
    連動終了し、プロセスリークが無いことも確認済み。
  - **SFTP実接続テスト: 成功(2026-08-15)**: ユーザーが管理者権限で
    Windows OpenSSH Serverを有効化。ed25519鍵はWindows版libssh2で
    認証コールバックが失敗(`Session(-19) Callback returned error`)し、
    **PEM形式RSA鍵(`ssh-keygen -m PEM`)でのみ認証成功**という実装依存の
    制約を発見。`sftp_client.rs`に`connect_with_private_key`
    (公開鍵認証、パスワードを扱わない設計)を追加。また、Windows sshdは
    SFTPパスをバックスラッシュ区切りで返すため、`collect_manifest`に
    `\`→`/`正規化を追加(本番Linuxサーバーとのマニフェストキー形式一貫性
    のため)。`sftp_client::tests::build_manifest_against_real_server`が
    実際にローカルOpenSSH Serverへ接続しファイル一覧・ハッシュ取得まで
    成功することを確認。これで5機能すべての実サーバーE2E検証が完了
    (DUAL DATABASE・aruaru-llm・LSPサーバー・VS Code拡張・SFTP)。
  - 次にすべきこと: (1) `ai_diff_advisor`のパース戦略見直し
    (フォーマット指定を諦めるか、より大きい/instruction-tunedなモデル
    への切り替えをaruaru-llm側に依頼するか)。(2) ed25519鍵が
    Windows版libssh2で失敗する原因の詳細調査(実運用でLinux本番
    サーバーへ接続する場合はed25519でも問題ない可能性が高いが未検証、
    このマシンのローカルWindows sshd固有の制約である可能性がある)。

- **2026-08-15(続き) 複数AIプロバイダ対応を追加(ユーザー指示
  「CLAUDEや、ChatGPTやGEMINIやDeepSeekやGrokなど有名なAIは全て対応
  させてそれでの開発での連携も対応して」)**:
  1. **LSP標準準拠(開発ツール連携)**: `sftp_git_lsp`は標準の
     Language Server Protocol(標準入出力でのJSON-RPC)で実装済み
     のため、VS Code専用ではなくLSPクライアントとして動作する任意の
     開発ツール(Claude Code含む)と原理上連携可能——追加実装は不要
     (既存設計がそのまま満たしている点を確認・明記したのみ)。
  2. **複数AIプロバイダクライアント新設**(`src/ai_providers.rs`):
     `AiProvider`列挙型(Claude/ChatGPT/Gemini/DeepSeek/Grok)+
     `AiProviderClient::complete(prompt)`。各社API形状の違い
     (Anthropic Messages API・OpenAI互換Chat Completions API
     〈ChatGPT/DeepSeek/Grok共通〉・Google Gemini generateContent API)
     を吸収し、統一インターフェースでプロンプト→生成テキストを返す。
     APIキーは環境変数(`ANTHROPIC_API_KEY`/`OPENAI_API_KEY`/
     `GEMINI_API_KEY`/`DEEPSEEK_API_KEY`/`XAI_API_KEY`)から読み、
     未設定時は`AiProviderError::NotConfigured`を正直に返す
     (黙って別プロバイダへフォールバックしない)。
  3. **正直な開示・未検証事項**: 各社APIの実装は公開ドキュメントの
     仕様に基づくが、**このセッションにはいずれのプロバイダの有効な
     APIキーも無く、実際のHTTP応答での動作確認はできていない**
     (aruaru-llmのみ自ホストのため実HTTP検証済み、CLAUDE.md該当節
     参照)。利用時は各自のAPIキーで疎通確認が必要。
  4. **検証結果**: `cargo test`**29件全green**(既存26件+
     `ai_providers`新規3件、実HTTPを伴わないロジックのみのテスト)。
  - 次にすべきこと: (1) 有効なAPIキーが得られ次第、各プロバイダの
    実HTTP疎通確認(`#[ignore]`付き実接続テストとして追加する設計に
    倣う)。(2) `ai_diff_advisor`/`cleanup_advisor`から
    `AiProviderClient`を実際に呼べるよう配線(現状はaruaru-llm経由の
    みが実配線済み、複数プロバイダからの選択機能はクライアント層のみ
    実装)。(3) LSPカスタムリクエストとしてプロバイダ選択オプションを
    公開するかの検討。

## 次回セッション再開メッセージ(2026-08-15セッション末尾)

次回このリポジトリで作業を再開する際は、以下から着手する:

1. **APIキー疎通確認(保留中)**: ユーザーがOpenAI(`info@aon.tokyo`、
   Teamアカウント)のログイン問題を抱えており未解決のまま中断。
   Claude/Gemini/DeepSeek/Grokは未着手。ユーザーから`.env`形式の
   キーファイルパスを受け取ったら、`ai_providers.rs`の各プロバイダへ
   `#[ignore]`付き実接続テストを追加し疎通確認すること。
2. **RPoemのVersionlessAPI実装への実依存追加**: `versionless_api.rs`
   は現状JSON値ベースのデモ変換のみ。RPoemの既存実装をpath依存として
   取り込む設計(DESIGN.md「2.」参照)がまだ手つかず。
3. **VS Code拡張のUI導線拡充**: 現状`cleanupAdvice`のみコマンド化
   済み。残り4機能(detectDrift/versionlessResolve/analyzeDiff/
   dualDatabase系)のVS Code側コマンド・UIは未実装。
4. **マルチプラットフォーム展開**: 着手順序(VS Code拡張→デスクトップ→
   モバイル)は確定済みだが、VS Code拡張自体がまだ検証段階(実機起動の
   み確認、実際の開発者向け機能としての使用感はまだ)。次はVS Code
   拡張の実用性を高めてから次段階へ進むこと。

このセッションでは横断的に`aruaru-llm`(CPU+GPU同時並列稼働の実装、
`F:\runo\aruaru-llm\CLAUDE.md`参照)・`open-cuda`(Android実機
Vulkanベンチマーク、`F:\runo\open-cuda\CLAUDE.md`参照)にも変更を
加えている。それぞれのリポジトリのCLAUDE.mdも合わせて確認すること。

## 追記(再開セッション、2026-08-15) RPoemのVersionlessAPI実装への実依存追加(次回再開メッセージ項目2を完了)

`Cargo.toml`に`open-runo-versionless-api`+`open-runo-rustjson`
(いずれもRPoemへのpath依存)を追加。`src/versionless_api.rs`に
`json_registry::JsonVersionRegistry`を新設し、RPoemの
`CompatibilityRule`/`apply_compatibility`をそのまま呼ぶ薄いラッパーを
実装(ゼロから作らず既存実装を活用、という当初方針をようやく実行)。
**ユーザー指摘で発覚した見落とし**: 当初`serde_json`を直接使う設計に
していたが、「RS-JSONじゃないの？」という指摘を受け調査したところ、
RustJSON(`open-runo-rustjson`)自体が「値モデルとして
`serde_json::Value`を採用する寛容パーサー」という設計(別の型では
なく`serde_json::Value`を生成するパーサー)であることが判明——
今回のコードは外部文字列をパースする箇所が無いため実害は無かったが、
将来外部JSON文字列をパースする箇所ができた場合は
`open_runo_rustjson::parse`を使うべき旨をコードコメントに明記した。
検証: `cargo test`**32件全green**(既存29件+`json_registry`新規3件)。

## 追記(再開セッション、2026-08-15) VS Code拡張のUI導線を残り4機能に拡充(次回再開メッセージ項目3を完了)

`vscode-extension/src/extension.ts`に5コマンドを追加し、LSPサーバーの
5機能全てにコマンドパレット経由のUI導線が揃った:
- `sftpGit.detectDrift` — Git/サーバー両マニフェストをJSON貼り付け
  形式(`showInputBox`)で受け取り、ドリフト結果を出力パネルへ表示。
- `sftpGit.versionlessResolve` — 現行スキーマJSON+変換先バージョンを
  入力し、RPoemの`CompatibilityRule`経由の変換結果を表示。
- `sftpGit.analyzeDiff` — アクティブエディタの選択範囲(または全文)を
  差分テキストとしてaruaru-llmへ送り、日英アドバイスを出力パネルへ
  表示(小型モデルでは数十秒かかる旨を事前に通知)。
- `sftpGit.dualDatabaseState`/`dualDatabaseOnPrimaryFailureDetected`/
  `dualDatabaseOnRecoveredAndResynced` — DUAL DATABASE状態の確認・
  障害通知・復旧通知。
`package.json`の`contributes.commands`に7コマンド全てを登録
(コマンドパレットから検索・実行可能に)。

**業務ロジックは一切追加していない**(既存方針通り、パラメータの
受け渡しとLSPリクエスト送信のみ、判定・変換ロジックはRust側のまま)。

**検証**: `npx tsc -p ./`でコンパイルエラー0件。VS Code拡張ホストを
実起動し、`sftp_git_lsp.exe`が子プロセスとして正常にspawn・終了する
ことを再確認(既存の検証手順を再実行、UI追加による副作用が無いことを
確認)。**正直な開示**: 各コマンドを実際にクリックしてダイアログ
入力→結果表示まで一気通貫でテストするところまでは今回行っていない
(拡張ホストの起動・LSPサーバーのspawnまでの確認に留まる)——次回、
実際にVS Code UIを操作してのE2E確認が望ましい。

## 追記(再開セッション、2026-08-15) LSPプロトコル経由の自動E2E確認+実バグ発見

ユーザー指示「各コマンドを実際にクリックしてダイアログ入力→結果表示
までのE2E確認…これだけは完成させて」への対応。VS Code UIの手動クリック
操作の代わりに、拡張機能(`extension.ts`)が実際に送るのと**全く同じ
LSPカスタムリクエストを、Node.jsスクリプトで標準入出力経由(LSP標準の
Content-Length方式)で直接送信・検証**する方式で自動E2E確認を実施した
(`vscode-extension/e2e-test.js`+`e2e-test-analyze-diff.js`)。

**結果**: `cleanupAdvice`/`detectDrift`/`versionlessResolve`/
`dualDatabaseState`系3リクエスト/`buildDiffPrompt`の6リクエストは
全て期待通りの応答を確認。`analyzeDiff`は実aruaru-llmサーバー
(CPU専用ビルド)に対して実際に呼び出し、日英アドバイスが返ることを
確認(内容は小型モデルの限界で文法崩壊・HTMLタグ混入と、こちらも
正直な結果)。

**副産物として実バグを発見**: `real-vulkan`ビルドのaruaru-llmへ
同時並行で`/v1/generate`を2件送ると、DevicePoolがGPU側へ振り分けた
リクエストが`no spirv bytes were provided`で失敗することを発見
(詳細・対応は`F:\runo\aruaru-llm\CLAUDE.md`2026-08-15(続き2)
エントリに記録)。sftp-git側は**CPU専用ビルドを使うことで回避**——
根本原因の特定はaruaru-llm側の今後の課題として切り出した。

## 追記(同セッション) Claude(Anthropic) APIキー疎通確認完了

ユーザーから`ANTHROPIC_API_KEY`をファイル経由(`.secrets/ai_keys.env.txt`、
リポジトリ外)で受け取り、`AiProviderClient::complete`と同じHTTP
リクエスト形状で実疎通確認を実施。**正直な事故の記録**: 確認手順の
初期段階で、シェルコマンドのエラー出力・`head`コマンドの誤用により
**キーの一部がツール出力に2回露出した**——都度ユーザーへ即座に報告し
失効・再発行を依頼、以降は値を一切標準出力へ出さない方式
(`$(cat file)`での変数代入のみ、echo/printを一切使わない)に切り替えて
再検証した。

**結果**: **認証自体は成功**(HTTP 400、`invalid_request_error`で
「クレジット残高不足」というAnthropic側の課金起因のエラーメッセージが
返った——APIキーの形式・`x-api-key`/`anthropic-version`ヘッダの
組み立て方は正しいことが実証された)。`src/ai_providers.rs`の
Claude実装コード自体に問題は無い。**クレジットチャージは決済行為
のため対応不可**、ユーザー自身がAnthropic Console
(https://console.anthropic.com/settings/billing)で行う必要がある。

- 次にすべきこと: (1) Anthropicアカウントへのクレジットチャージ後、
  再度疎通確認(コード変更は不要、クレジット追加のみで動作するはず)。
  (2) ChatGPT/Gemini/DeepSeek/Grokの疎通確認(いずれも未着手のまま)。
  (3) マルチプラットフォーム展開の次段階。(4) より豊かなUI
  (Webview等)への発展の検討。(5) aruaru-llm側`real-vulkan`のGPU配線
  失敗バグの根本原因調査(aruaru-llm側の課題として切り出し済み)。

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**

## Versionless API の方針 / Versionless API policy (2026-11-25)
**日本語**: このリポジトリのAPIは **Versionless**(URL・パッケージ名に `v1` 等を付けず、互換性を壊さず足すだけで進化させる)。欄は足すだけ・足す欄は省略可能・知らない欄は無視・名前変更は新名を足して旧を非推奨に(期限は既定で6か月以上、使用0を確認してから削除)・enumは未知の値を許容・安全上の緊急修正は例外。既存の `.v1` は呼び出し側を壊さないよう当面残し、版なしの別名を追加して段階的に畳む。詳細・出典は [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md) 末尾。適用はこれから順に行う。
**English**: This repository's APIs are **Versionless**: no `v1` in URLs or package names; evolve by adding only, without breaking compatibility. Only add fields, new fields are optional, ignore unknown fields, rename by adding the new name and deprecating the old (default deadline 6 months or more, remove only after confirming zero use), tolerate unknown enum values, emergency security fixes are the exception. Existing `.v1` names stay for now so callers do not break; a versionless alias is added and `.v1` is retired in stages. Details and sources: the end of [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md). Rollout is step by step.


## 2026-11-26 x-tenant と呼び出し側のConnect化 / x-tenant and Connect callers
**日本語**: aruaru-llm の呼び出しを Connect の `Llm/Generate` に移し、`x-tenant: sftp-git` を付けました(`aa38af1`、既定ブランチは master)。試験32件成功。VPSでは動いていません。
**English**: The aruaru-llm call moved to Connect `Llm/Generate` with `x-tenant: sftp-git` (`aa38af1`, default branch master). 32 tests pass. Not running on the VPS.
