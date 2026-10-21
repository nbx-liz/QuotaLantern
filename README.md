# QuotaLantern

AIの残り利用枠を、見失わない。

QuotaLantern is a small resident monitor for macOS (menu bar) and Windows (notification area) that shows the remaining usage quota, reset time, and context usage of AI coding tools — starting with Claude Code and Codex.

## Status

Pre-alpha. Nothing is implemented yet. The first milestone (G0) is verification, not UI: on both operating systems, establish what each acquisition path receives, what it stores, and where it communicates, before any path is declared supported.

## Principles

- **No persistence of usage data.** Measurements live in memory only. No temporary files, caches, databases, stable hashes, or diagnostic attachments.
- **No own server, no telemetry.** The core and UI make no external network requests.
- **Minimum data.** Prompts, responses, credentials, cookies, emails, organization IDs, and paths are never acquired on purpose, and anything received unavoidably is discarded at the adapter boundary.
- **Fail closed, but never block the AI tool.** Values that cannot be acquired safely are shown as "not acquired" or "unsupported" — never guessed, never shown as 0 or unlimited.
- **Official outputs only.** No cookie extraction, private APIs, traffic interception, or log scraping.

## Planned layout

```text
crates/core/            state machine, aggregation, time and freshness (RAM only)
crates/protocol/        internal event types and validation
crates/adapter-claude/  Claude Code status line: allowlisted field extraction
crates/adapter-codex/   Codex App Server: fixed read-only RPC
crates/ipc/             per-OS local IPC
apps/desktop/           resident UI (Tauri 2 subject to a storage audit)
schemas/                internal event contract (source of truth)
tests/fixtures/         synthetic data only
docs/adr/               architecture decision records
docs/security/          threat model and storage / network audits
```

## Documentation

Specifications and decision records are written in Japanese.

- [Requirements](docs/requirements.md)
- [Guarantee boundary](docs/security/guarantee-boundary.md) — what "no persistence" means and what is not guaranteed
- [ADR ledger](docs/adr/README.md)
- [Contributing](CONTRIBUTING.md) · [Security policy](SECURITY.md)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
