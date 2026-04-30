# Agents

## Cursor Cloud specific instructions

This is a Rust workspace monorepo for a P2P-first encrypted messenger. All build/test/lint commands use Cargo. See `README.md` for full development commands.

### Quick reference

- **Build:** `cargo build --workspace`
- **Test:** `cargo test --workspace`
- **Lint:** `cargo clippy --workspace -- -D warnings`
- **Run server:** `cargo run -p messenger-server` (listens on `127.0.0.1:8080`)
- **Health check:** `curl http://127.0.0.1:8080/health`
- **End-to-end smoke test:** `bash scripts/dev-relay-smoke.sh`

### Non-obvious notes

- The workspace enforces strict Clippy lints: `unwrap_used = "deny"` and `expect_used = "deny"`. Use `match` / `if let` / `map_err` instead of `.unwrap()` / `.expect()`.
- `unsafe_code` is forbidden workspace-wide.
- The `rusqlite` dependency uses the `bundled` feature, so no system SQLite library is needed — it compiles SQLite from C source.
- The smoke test (`scripts/dev-relay-smoke.sh`) starts its own server instance on port 8080 and cleans up after itself. Stop any manually-started server on that port before running the smoke test to avoid port conflicts.
- By default the relay queue is in-memory. Set `MESSENGER_SQLITE_PATH=./relay.db` to persist relay envelopes across server restarts.
- The `apps/flutter/` directory is a placeholder — no Flutter code exists yet.
