# Development Philosophy & Environment Rules (sftp-git)

📖 日本語(正本、詳細な実装ログ): [CLAUDE.md](CLAUDE.md)

This file is a summary translation. The Japanese `CLAUDE.md` is the
canonical source of truth and contains the full, detailed HANDOFF log
of every implementation session — read it (or ask for a translation
of a specific entry) if you need full detail.

Created 2026-08-15. Logic for 5 core features has been implemented and
verified end-to-end against real servers:

1. **SFTP drift detection** (`src/drift.rs` + `src/sftp_client.rs`) —
   verified against a real local OpenSSH Server.
2. **Versionless API × versioned API hybrid** (`src/versionless_api.rs`)
   — now backed by a real path dependency on RPoem's
   `open-runo-versionless-api` crate (`CompatibilityRule` +
   `apply_compatibility`), via the `json_registry` submodule.
3. **AI-assisted stale-file deletion advice** (`src/cleanup_advisor.rs`)
   — logic only; by design this never deletes files itself, only
   surfaces decision material for a human to act on.
4. **Dual database (aruaru-db + PostgreSQL)** (`src/dual_database.rs` +
   `src/dual_database_client.rs`) — verified against a real local
   `aruaru-server` instance (aruaru-db speaks the PostgreSQL wire
   protocol, so the same `tokio-postgres` client works for both).
5. **AI diff analysis with bilingual (JA/EN) advice**
   (`src/ai_diff_advisor.rs` + `src/aruaru_llm_client.rs`) — connected
   successfully to a real local aruaru-llm instance, but discovered
   that small GPT-2-family models are not instruction-tuned and cannot
   reliably follow a format-constrained single-call prompt; redesigned
   to issue two separate calls (one per language) instead.

Also implemented: an LSP server (`src/bin/sftp_git_lsp.rs`) exposing
all 5 features as custom LSP requests, and a VS Code extension
(`vscode-extension/`) that is a thin Node.js connection layer only —
all business logic lives in the Rust LSP server (the same
"rust-analyzer pattern" used by that project). **All 5 features now
have a command-palette UI** in the extension (`sftpGit.cleanupAdvice`,
`sftpGit.detectDrift`, `sftpGit.versionlessResolve`,
`sftpGit.analyzeDiff`, and 3 dual-database commands). Verified that the
extension host launches, spawns the LSP server as a child process, and
shuts down cleanly with no process leaks.

Multi-AI-provider client (`src/ai_providers.rs`) supports Claude,
ChatGPT, Gemini, DeepSeek, and Grok in addition to aruaru-llm, but real
HTTP connectivity has not been verified (no valid API key was
available in this development session).

The work drive is `F:\runo` (see [RUNO](https://github.com/aon-co-jp/RUNO)'s
CLAUDE.md for the overall drive-migration status across the
ecosystem).

## Where to find things

- **README.md / README-English.md** — feature overview, current status
  table, market research findings.
- **DESIGN.md** — implementation-level design notes for each feature,
  including the multi-platform distribution plan (VS Code extension →
  desktop → mobile) and honest write-ups of open questions.
- **CLAUDE.md (Japanese)** — the full HANDOFF log, one dated entry per
  work session, in chronological order. This is where you'll find the
  measured benchmark numbers (e.g. the Android phone GPU vs desktop
  GPU comparison), the exact error messages hit during SFTP key-auth
  debugging, and so on.

## Honest open items (as of 2026-08-15)

- None of the 5 AI providers (Claude/ChatGPT/Gemini/DeepSeek/Grok)
  have been connectivity-tested with a real API key.
- The VS Code extension's UI is a plain `showInputBox`-based flow
  (paste JSON, type a version string) — clicking through each command
  end-to-end in a real VS Code window has not been verified this
  session (only that the extension host launches and spawns the LSP
  server correctly).
- Desktop (Windows/Mac/Linux) and mobile (Android/iOS) apps have not
  been started — the agreed order is VS Code extension first, then
  desktop, then mobile.
