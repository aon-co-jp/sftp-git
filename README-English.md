# sftp-git

📖 日本語: [README.md](README.md)

A tool for web developers that integrates **SFTP upload** and **Git
management**, combining a **versionless API** with traditional
**versioned API** management to reduce the manual effort and mistakes
involved in checking diffs between test and production servers and
deploying. As of 2026-08-15, the logic for 5 core features (drift
detection, versionless-API hybrid, AI-assisted stale-file deletion
advice, dual database, AI diff analysis) has been implemented and
verified end-to-end against real servers. LSP-server mode and a VS
Code extension (thin Node.js layer) have also been implemented and
verified to launch on a real machine.

Reference service: https://lp.smartrelease.cloud/

## Distribution plan (per user instruction, 2026-08-15)

- Developed primarily as a **VS Code extension**.
- Also planned: native apps for **Windows/Mac/Linux**, and mobile
  apps for **Android (phone/tablet) / iPhone / iPad**.
- Only the direction is decided so far; how much code the VS Code
  extension and the native/mobile apps will share is still
  undesigned. See "7. Multi-platform distribution" in
  [DESIGN.md](DESIGN.md).
- **Dev-tool interoperability**: the LSP server follows the standard
  Language Server Protocol, so any LSP client (not just VS Code,
  including Claude Code) can talk to the same Rust binary.
- **Multi-AI-provider support**: in addition to aruaru-llm, the AI
  diff-analysis backend can now select Claude (Anthropic), ChatGPT
  (OpenAI), Gemini (Google), DeepSeek, or Grok (xAI) —
  see `src/ai_providers.rs`. Each provider's API key is read from an
  environment variable; if unset, that provider honestly returns an
  error instead of silently doing nothing.

## Planned features

- **Git + SFTP upload hybrid**: track changes with Git while still
  supporting an upload workflow familiar to developers used to plain
  SFTP.
- **Versionless × versioned API hybrid**: aim for the best of both —
  the lightness of a versionless API and the safety of strict version
  management.
- **AI-assisted test/production diff review**: uses
  [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) for diff
  detection/review. To speed up the inference itself,
  [open-cuda](https://github.com/aon-co-jp/open-cuda) (NVIDIA GPU) and
  the DirectX-compatible
  [open-directx](https://github.com/aon-co-jp/open-directx)
  (for non-CUDA environments) are used as interchangeable hardware
  acceleration backends for aruaru-llm. Also generates improvement
  advice for detected diffs in **both Japanese and English**.
- **AI-assisted stale-file deletion advice**: for old/unused files
  managed under Git + versionless API,
  [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) presents
  decision material (whether referenced, time since last update, how
  it's treated in commit history, etc.) to help the developer decide.
  **The final delete action always requires the developer's approval
  — the AI never deletes automatically** (to avoid accidental
  deletion).
- **Full backup**: primarily [aruaru-db](https://github.com/aon-co-jp/aruaru-db).
  Optionally add PostgreSQL for a **DUAL DATABASE** setup (redundant
  configuration where either side can fully back up the other if it
  fails). Implemented in Rust, extended with
  [RPoem](https://github.com/aon-co-jp/RPoem).
- **Hardware acceleration (split by use case)**:
  - For encryption/compression/decompression speed-up, use
    [open-raid-z](https://github.com/aon-co-jp/open-raid-z)'s
    `open_raid_z_core` (GPU/D3D12-capable, already used in production
    by [RS-SmartTCP](https://github.com/aon-co-jp/RS-SmartTCP)).
    Investigation found that
    [open-directx](https://github.com/aon-co-jp/open-directx) and
    [open-cuda](https://github.com/aon-co-jp/open-cuda) are not a good
    fit for this use case, so they are excluded here.
  - For AI diff-analysis / advice-generation inference speed-up
    (aruaru-llm), use
    [open-cuda](https://github.com/aon-co-jp/open-cuda) (NVIDIA GPU)
    and [open-directx](https://github.com/aon-co-jp/open-directx)
    (DirectX-compatible, for non-CUDA environments) as appropriate.
    These were originally built for LLM inference, so they fit this
    use case well.

## Market research (2026-08-15)

Researched existing prior art in both Japanese and English:

- **SFTP × Git integration tools**: several one-directional
  "git push → auto-sync to SFTP" tools exist (DeployHQ, GitFTP-Deploy,
  Git FTP Sync via GitHub Actions, AutoDeployToFTP, FTPbucket, etc.).
  However, no dedicated tool was found that does the **reverse**:
  detecting drift between manually-edited SFTP content and the Git
  repository. This could be a differentiator for sftp-git.
- **Versionless API**: Stripe's approach since 2024-09 (per-account,
  date-based API version + per-request header override; internally, a
  current canonical implementation plus transformers to past versions)
  is the leading example. Prior art for a hybrid approach ("only
  breaking changes get an explicit version, everything else evolves
  non-breaking") exists, e.g. the GitHub API (URI versioning + media
  type), but no literature was found that explicitly names a general
  "versionless × versioned" design pattern.
- **Staging/production diff detection**: the infrastructure
  drift-detection tool `driftctl` (acquired by Snyk) reached EOL in
  2023 and is effectively abandoned. No AI-powered
  staging/production-diff-specific review tool could be identified in
  this research.

See [DESIGN.md](DESIGN.md) for implementation-level design notes.

## Current state (as of 2026-08-15)

**Logic for 5 features implemented, with real-server E2E
verification completed:**

| Feature | Implementation | Real-server E2E |
|---|---|---|
| SFTP drift detection | `src/drift.rs` + `src/sftp_client.rs` | ✅ Succeeded against a local OpenSSH Server |
| Versionless API hybrid | `src/versionless_api.rs` (now backed by RPoem's `open-runo-versionless-api` as a real path dependency) | (logic only, real-server not applicable) |
| AI-assisted stale-file deletion advice | `src/cleanup_advisor.rs` | (logic only; by design never deletes) |
| Dual database | `src/dual_database.rs` + `src/dual_database_client.rs` | ✅ Succeeded writing to a local aruaru-server |
| AI diff analysis (JA/EN) | `src/ai_diff_advisor.rs` + `src/aruaru_llm_client.rs` | ✅ Connected successfully, but current model quality is not production-ready (see below) |

**LSP server + VS Code extension**: `src/bin/sftp_git_lsp.rs` exposes
all 5 features as custom LSP requests. `vscode-extension/` is a thin
Node.js connection layer only (rust-analyzer pattern). **All 5
features now have a command-palette UI** (as of 2026-08-15):
`sftpGit.cleanupAdvice` / `sftpGit.detectDrift` /
`sftpGit.versionlessResolve` / `sftpGit.analyzeDiff` /
`sftpGit.dualDatabaseState` and 2 related dual-database commands.
Verified that the VS Code extension host launches, spawns the LSP
server as a child process, and shuts down cleanly.

**Important finding (disclosed honestly)**: small GPT-2-family models
(`distilgpt2` / `gpt2-medium`) are not instruction-tuned and cannot
follow format-constrained prompts. `ai_diff_advisor` was redesigned to
issue two separate calls (one per language) instead of relying on a
single call with a required output format.

**Multi-AI-provider support**: in addition to aruaru-llm, a unified
client (`src/ai_providers.rs`) now supports selecting Claude, ChatGPT,
Gemini, DeepSeek, or Grok. **Real HTTP connectivity has not been
verified** — this session had no valid API key for any of these
providers.

See the HANDOFF section in CLAUDE.md for detailed implementation logs
and measured results.

## Not yet started / next steps

- Verify real HTTP connectivity for each AI provider (requires a
  valid API key).
- Windows/Mac/Linux desktop apps and Android/iOS mobile apps (planned
  order: VS Code extension → desktop → mobile; see DESIGN.md).
- The VS Code extension's UI is currently `showInputBox`-based
  (paste JSON, type a version string); a richer UI (e.g. a Webview)
  is a future improvement to consider.

## Related repositories

- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — candidate for AI diff-check integration
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — candidate for full-backup integration
- [RPoem](https://github.com/aon-co-jp/RPoem) — candidate for Rust feature extension
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — leading candidate for hardware-accelerated encryption/compression (`open_raid_z_core`)
- Ecosystem index: [RUNO](https://github.com/aon-co-jp/RUNO)
