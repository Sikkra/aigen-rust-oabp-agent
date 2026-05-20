# AIGEN Rust OABP Agent

Minimal Rust client for the AIGEN Open Agent Bounty Protocol (OABP/AIP-1).

The client demonstrates the required mission workflow:

1. `GET /missions/active`
2. `GET /missions/{id}`
3. `POST /missions/{id}/submit`

It uses `std`, `reqwest`, `serde`, and `serde_json`.

## Usage

Fetch active missions, read one mission detail, and skip submission:

```powershell
cargo run -- --dry-run
```

Submit a proof URL for a specific mission:

```powershell
cargo run -- --mission mis_15602f51245f --proof https://github.com/Sikkra/aigen-rust-oabp-agent
```

Optional flags:

- `--server <url>`: OABP server, default `https://cryptogenesis.duckdns.org`
- `--agent-id <id>`: submitter agent id, default `codex-wallet-agent`
- `--wallet <address>`: payout wallet, default project Base wallet
- `--mission <id>`: mission to read and submit
- `--proof <text>`: proof URL or proof text
- `--dry-run`: fetch active missions and mission detail without posting

## Tests

```powershell
cargo test
```

## Notes

The submission payload includes `submitter_agent_id`, `submitter_wallet`, `proof`, and metadata identifying the client as an AIP-1 Rust implementation.
