# PORTING (sftp-git)

📖 日本語: [PORTING.md](PORTING.md)

No migration source/destination information — this is a new project
with no prior tool being migrated from. If a migration path from an
existing SFTP client / Git-integration tool (DeployHQ, GitFTP-Deploy,
etc. — see "Market research" in README-English.md) becomes concrete in
the future, it will be documented here.

## Reusing this repository's functionality from other projects

This repository itself is not meant to be "ported into" other
projects, but several of its Rust modules are designed to be reusable
individually as `path` dependencies (each is self-contained and does
not require depending on the whole `sftp_git` crate):

- `sftp_git::drift` — for projects that only need the Git-manifest vs.
  real-server hash-matching logic (reusable for protocols other than
  SFTP too).
- `sftp_git::dual_database` / `dual_database_client` — reusable as-is
  by other projects that need an aruaru-db (PostgreSQL wire protocol
  compatible) + PostgreSQL redundant setup.
- `sftp_git::ai_providers` — intended for reuse by other projects that
  need a unified client for Claude/ChatGPT/Gemini/DeepSeek/Grok (real
  API-key connectivity has not been verified yet — see CLAUDE.md).

## Dependency repositories (sibling path dependencies)

This repository depends on the following (a mix of already-wired and
still-planned):

- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — via HTTP
  (`src/aruaru_llm_client.rs`; an HTTP API client, not a path
  dependency; already wired up)
- [RPoem](https://github.com/aon-co-jp/RPoem) — **wired up as a real
  path dependency (2026-08-15)**: `open-runo-versionless-api`
  (`CompatibilityRule` / `apply_compatibility`, used from
  `src/versionless_api.rs::json_registry`) and `open-runo-rustjson`
  (RS-JSON; not used yet, but added as a dependency for future
  external-JSON-string parsing).
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) —
  hardware-accelerated encryption/compression via `open_raid_z_core`
  (not started)
