> Latest translations / 翻訳の最新版: [0-LATEST-TRANSLATIONS.md](0-LATEST-TRANSLATIONS.md)

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
