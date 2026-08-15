// analyzeDiff専用のE2E確認(実aruaru-llmサーバーが必要、時間がかかるため分離)。
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
    await request("initialize", { processId: process.pid, rootUri: null, capabilities: {} });
    notify("initialized", {});
    console.log("[OK] initialize");

    console.log("analyzeDiff実行中(小型モデルのため数十秒かかります)...");
    const result = await request("sftpGit/analyzeDiff", {
      aruaru_llm_base_url: "http://127.0.0.1:14600",
      file_path: "index.html",
      diff_text: "-<h1>old</h1>\n+<h1>new</h1>",
    });
    if (typeof result.japanese !== "string" || result.japanese.length === 0) {
      throw new Error(`analyzeDiff: japaneseが空です ${JSON.stringify(result)}`);
    }
    if (typeof result.english !== "string" || result.english.length === 0) {
      throw new Error(`analyzeDiff: englishが空です ${JSON.stringify(result)}`);
    }
    console.log("[OK] analyzeDiff");
    console.log("japanese:", result.japanese.slice(0, 100));
    console.log("english:", result.english.slice(0, 100));
  }

  run()
    .catch((err) => {
      failed = true;
      console.error("失敗:", err);
    })
    .finally(() => {
      proc.kill();
      process.exit(failed ? 1 : 0);
    });
}
main();
