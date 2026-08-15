// sftp-git LSPサーバーへの実E2E確認スクリプト。
//
// VS Code拡張(extension.ts)が各コマンドから送るのと全く同じLSP
// カスタムリクエストを、実際にsftp_git_lsp.exeへ標準入出力(LSP標準の
// Content-Length方式)で送信し、応答を検証する。VS CodeのUIを手動で
// クリックする代わりに、拡張が実際に叩くのと同一のワイヤプロトコルを
// 自動テストすることで、「ダイアログ入力→結果表示」の裏側にある
// リクエスト/レスポンス経路を実際に検証する。

const { spawn } = require("child_process");
const path = require("path");

const exeName = process.platform === "win32" ? "sftp_git_lsp.exe" : "sftp_git_lsp";
const serverPath = path.join(__dirname, "..", "target", "debug", exeName);

let nextId = 1;
let buffer = Buffer.alloc(0);
const pending = new Map();

function encode(msg) {
  const json = JSON.stringify(msg);
  const body = Buffer.from(json, "utf8");
  const header = Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "utf8");
  return Buffer.concat([header, body]);
}

function tryParse() {
  const headerEnd = buffer.indexOf("\r\n\r\n");
  if (headerEnd === -1) return null;
  const header = buffer.slice(0, headerEnd).toString("utf8");
  const match = header.match(/Content-Length: (\d+)/i);
  if (!match) return null;
  const length = parseInt(match[1], 10);
  const bodyStart = headerEnd + 4;
  if (buffer.length < bodyStart + length) return null;
  const body = buffer.slice(bodyStart, bodyStart + length).toString("utf8");
  buffer = buffer.slice(bodyStart + length);
  return JSON.parse(body);
}

function main() {
  const proc = spawn(serverPath, [], { stdio: ["pipe", "pipe", "pipe"] });
  let failed = false;

  proc.stderr.on("data", (d) => process.stderr.write(`[server stderr] ${d}`));

  proc.stdout.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    let msg;
    while ((msg = tryParse())) {
      if (msg.id !== undefined && pending.has(msg.id)) {
        const { resolve, reject } = pending.get(msg.id);
        pending.delete(msg.id);
        if (msg.error) reject(new Error(JSON.stringify(msg.error)));
        else resolve(msg.result);
      }
    }
  });

  function request(method, params) {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      proc.stdin.write(encode({ jsonrpc: "2.0", id, method, params }));
    });
  }

  function notify(method, params) {
    proc.stdin.write(encode({ jsonrpc: "2.0", method, params }));
  }

  async function run() {
    console.log("=== sftp-git LSP E2E確認開始 ===");

    await request("initialize", { processId: process.pid, rootUri: null, capabilities: {} });
    notify("initialized", {});
    console.log("[OK] initialize");

    const cleanup = await request("sftpGit/cleanupAdvice", {
      path: "old_backup.bak",
      days_since_last_commit: 400,
      is_referenced: false,
      was_accessed_in_production: false,
    });
    assertField(cleanup, "recommendation", "LikelySafeToDelete", "cleanupAdvice");

    const drift = await request("sftpGit/detectDrift", {
      git_manifest: { "index.html": "abc" },
      server_manifest: { "index.html": "abc", "uploaded_by_hand.php": "xyz" },
    });
    if (!Array.isArray(drift) || drift.length !== 1 || drift[0].kind !== "OnlyOnServer") {
      throw new Error(`detectDrift: 想定外の結果 ${JSON.stringify(drift)}`);
    }
    console.log("[OK] detectDrift");

    const versionless = await request("sftpGit/versionlessResolve", {
      current: { full_name: "Taro Yamada" },
      requested_version: "2020-01-01",
    });
    if (versionless.resolved.first_name !== "Taro") {
      throw new Error(`versionlessResolve: 想定外の結果 ${JSON.stringify(versionless)}`);
    }
    console.log("[OK] versionlessResolve");

    const state1 = await request("sftpGit/dualDatabaseState", {});
    assertField(state1, "primary", "AruaruDb", "dualDatabaseState(initial)");

    const state2 = await request("sftpGit/dualDatabaseOnPrimaryFailureDetected", {});
    assertField(state2, "primary", "PostgreSql", "dualDatabaseOnPrimaryFailureDetected");
    assertField(state2, "mode", "Failover", "dualDatabaseOnPrimaryFailureDetected(mode)");

    const state3 = await request("sftpGit/dualDatabaseOnRecoveredAndResynced", {});
    assertField(state3, "primary", "AruaruDb", "dualDatabaseOnRecoveredAndResynced");
    assertField(state3, "mode", "Normal", "dualDatabaseOnRecoveredAndResynced(mode)");

    const prompt = await request("sftpGit/buildDiffPrompt", {
      file_path: "index.html",
      diff_text: "-old\n+new",
    });
    if (!prompt.prompt.includes("index.html")) {
      throw new Error(`buildDiffPrompt: 想定外の結果 ${JSON.stringify(prompt)}`);
    }
    console.log("[OK] buildDiffPrompt");

    console.log("=== 全項目 E2E確認成功(analyzeDiffは実aruaru-llmサーバーが必要なため別途確認) ===");
  }

  function assertField(obj, field, expected, label) {
    if (obj[field] !== expected) {
      throw new Error(`${label}: ${field}が${expected}ではなく${obj[field]}でした`);
    }
    console.log(`[OK] ${label}`);
  }

  run()
    .catch((err) => {
      failed = true;
      console.error("E2E確認失敗:", err);
    })
    .finally(() => {
      proc.kill();
      process.exit(failed ? 1 : 0);
    });
}

main();
