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

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
