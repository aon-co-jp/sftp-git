// sftp-git VS Code拡張機能: Node製の薄い接続層のみ(rust-analyzer方式)。
//
// 業務ロジックは一切ここに置かない。すべてRust製LSPサーバー
// (../target/{debug,release}/sftp_git_lsp)が持ち、このファイルは
// 子プロセスとして起動し標準入出力でLSPメッセージを中継するだけ。

import * as path from "path";
import * as fs from "fs";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
const outputChannel = vscode.window.createOutputChannel("sftp-git");

function resolveServerBinaryPath(context: vscode.ExtensionContext): string {
  const exeName = process.platform === "win32" ? "sftp_git_lsp.exe" : "sftp_git_lsp";
  const candidates = [
    path.join(context.extensionPath, "..", "target", "release", exeName),
    path.join(context.extensionPath, "..", "target", "debug", exeName),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(
    `sftp_git_lspバイナリが見つかりません。事前に \`cargo build --bin sftp_git_lsp\` を実行してください(探索先: ${candidates.join(", ")})`
  );
}

export function activate(context: vscode.ExtensionContext): void {
  const serverBinary = resolveServerBinaryPath(context);

  const serverOptions: ServerOptions = {
    command: serverBinary,
    transport: 0, // stdio
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file" }],
  };

  client = new LanguageClient(
    "sftpGit",
    "sftp-git LSP",
    serverOptions,
    clientOptions
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.cleanupAdvice", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || !client) {
        return;
      }
      const filePath = editor.document.uri.fsPath;
      // 業務ロジック(参照有無・最終更新日数等の判定基準)はすべて
      // Rust側(cleanup_advisor.rs)にある。ここではパスを渡すだけ。
      const result = await client.sendRequest("sftpGit/cleanupAdvice", {
        path: filePath,
        days_since_last_commit: 0,
        is_referenced: false,
        was_accessed_in_production: null,
      });
      vscode.window.showInformationMessage(JSON.stringify(result));
    })
  );

  // 2026-08-15追加: 残り4機能のUIコマンド。いずれもRust側LSPサーバーへ
  // パラメータを渡すだけの薄い接続層で、判定ロジック自体は持たない
  // (cleanupAdviceと同じ方針)。

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.detectDrift", async () => {
      if (!client) {
        return;
      }
      const gitManifestRaw = await vscode.window.showInputBox({
        prompt:
          "Gitマニフェスト(JSON、パス→ハッシュ)を貼り付けてください。例: " +
          '{"index.html":"abc123"}',
        placeHolder: '{"index.html":"abc123"}',
      });
      if (gitManifestRaw === undefined) {
        return;
      }
      const serverManifestRaw = await vscode.window.showInputBox({
        prompt:
          "本番サーバー側マニフェスト(JSON、パス→ハッシュ)を貼り付けてください。",
        placeHolder: '{"index.html":"def456"}',
      });
      if (serverManifestRaw === undefined) {
        return;
      }
      let gitManifest: Record<string, string>;
      let serverManifest: Record<string, string>;
      try {
        gitManifest = JSON.parse(gitManifestRaw);
        serverManifest = JSON.parse(serverManifestRaw);
      } catch (e) {
        vscode.window.showErrorMessage(`JSON解析に失敗しました: ${e}`);
        return;
      }
      const result = await client.sendRequest("sftpGit/detectDrift", {
        git_manifest: gitManifest,
        server_manifest: serverManifest,
      });
      const drifts = result as Array<{ path: string; kind: string }>;
      if (drifts.length === 0) {
        vscode.window.showInformationMessage(
          "sftp-git: ドリフトは検出されませんでした(GitとSFTPサーバーは一致しています)。"
        );
      } else {
        vscode.window.showWarningMessage(
          `sftp-git: ${drifts.length}件のドリフトを検出しました。詳細は出力パネルを確認してください。`
        );
        outputChannel.appendLine("=== SFTPドリフト検出結果 ===");
        for (const d of drifts) {
          outputChannel.appendLine(`${d.path}: ${d.kind}`);
        }
        outputChannel.show();
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.versionlessResolve", async () => {
      if (!client) {
        return;
      }
      const currentRaw = await vscode.window.showInputBox({
        prompt: "現行スキーマのJSON値を貼り付けてください。",
        placeHolder: '{"full_name":"Taro Yamada"}',
      });
      if (currentRaw === undefined) {
        return;
      }
      const requestedVersion = await vscode.window.showInputBox({
        prompt: "変換先バージョン(例: 2020-01-01)。空欄なら無変換のまま返します。",
        placeHolder: "2020-01-01",
      });
      if (requestedVersion === undefined) {
        return;
      }
      let current: unknown;
      try {
        current = JSON.parse(currentRaw);
      } catch (e) {
        vscode.window.showErrorMessage(`JSON解析に失敗しました: ${e}`);
        return;
      }
      const result = await client.sendRequest("sftpGit/versionlessResolve", {
        current,
        requested_version: requestedVersion.trim() === "" ? null : requestedVersion,
      });
      vscode.window.showInformationMessage(JSON.stringify(result));
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.analyzeDiff", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || !client) {
        return;
      }
      const baseUrl = await vscode.window.showInputBox({
        prompt: "aruaru-llmサーバーのベースURL",
        placeHolder: "http://127.0.0.1:4600",
        value: "http://127.0.0.1:4600",
      });
      if (baseUrl === undefined) {
        return;
      }
      const diffText = editor.selection.isEmpty
        ? editor.document.getText()
        : editor.document.getText(editor.selection);
      vscode.window.showInformationMessage(
        "sftp-git: AI差分解析を実行中です(小型モデルでは数十秒かかる場合があります)…"
      );
      try {
        const result = await client.sendRequest("sftpGit/analyzeDiff", {
          aruaru_llm_base_url: baseUrl,
          file_path: editor.document.uri.fsPath,
          diff_text: diffText,
        });
        const { japanese, english } = result as { japanese: string; english: string };
        outputChannel.appendLine("=== AI差分解析(日本語) ===");
        outputChannel.appendLine(japanese);
        outputChannel.appendLine("=== AI Diff Analysis (English) ===");
        outputChannel.appendLine(english);
        outputChannel.show();
      } catch (e) {
        vscode.window.showErrorMessage(`AI差分解析に失敗しました: ${e}`);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.dualDatabaseState", async () => {
      if (!client) {
        return;
      }
      const result = await client.sendRequest("sftpGit/dualDatabaseState", {});
      vscode.window.showInformationMessage(`DUAL DATABASE状態: ${JSON.stringify(result)}`);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.dualDatabaseOnPrimaryFailureDetected", async () => {
      if (!client) {
        return;
      }
      const result = await client.sendRequest(
        "sftpGit/dualDatabaseOnPrimaryFailureDetected",
        {}
      );
      vscode.window.showWarningMessage(
        `sftp-git: 主系障害を通知しました。新しい状態: ${JSON.stringify(result)}`
      );
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sftpGit.dualDatabaseOnRecoveredAndResynced", async () => {
      if (!client) {
        return;
      }
      const result = await client.sendRequest(
        "sftpGit/dualDatabaseOnRecoveredAndResynced",
        {}
      );
      vscode.window.showInformationMessage(
        `sftp-git: 復旧・再同期を通知しました。新しい状態: ${JSON.stringify(result)}`
      );
    })
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
