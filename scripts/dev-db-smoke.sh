#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d)"
SERVER_LOG="$WORK_DIR/server.log"
SERVER_HOST="127.0.0.1"
SERVER_PORT="${MESSENGER_SMOKE_PORT:-18080}"
SERVER_URL="http://$SERVER_HOST:$SERVER_PORT"
MESSAGE="hello db workflow"
SERVER_PID=""

cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

cd "$ROOT_DIR"

echo "Building server and dev CLI..."
cargo build -p messenger-server -p messenger-dev

echo "Starting local relay server on $SERVER_URL..."
MESSENGER_SQLITE_PATH="$WORK_DIR/relay.db" \
MESSENGER_BIND_ADDR="$SERVER_HOST:$SERVER_PORT" \
  "$ROOT_DIR/target/debug/messenger-server" >"$SERVER_LOG" 2>&1 &
SERVER_PID="$!"

for _ in $(seq 1 50); do
  if bash -c "exec 3<>/dev/tcp/$SERVER_HOST/$SERVER_PORT" 2>/dev/null; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Server exited early. Log:"
    sed 's/^/  /' "$SERVER_LOG"
    exit 1
  fi
  sleep 0.1
done

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
  echo "Server did not stay running. Log:"
  sed 's/^/  /' "$SERVER_LOG"
  exit 1
fi

echo "Initializing Alice and Bob client databases..."
cargo run -p messenger-dev -- db init --db "$WORK_DIR/alice.db" --server "$SERVER_URL"
cargo run -p messenger-dev -- db init --db "$WORK_DIR/bob.db" --server "$SERVER_URL"
cargo run -p messenger-dev -- db public --db "$WORK_DIR/alice.db" --out "$WORK_DIR/alice.public.json" --server "$SERVER_URL"
cargo run -p messenger-dev -- db public --db "$WORK_DIR/bob.db" --out "$WORK_DIR/bob.public.json" --server "$SERVER_URL"

echo "Adding contacts..."
cargo run -p messenger-dev -- contact add \
  --db "$WORK_DIR/alice.db" \
  --server "$SERVER_URL" \
  --name Bob \
  --public "$WORK_DIR/bob.public.json"
cargo run -p messenger-dev -- contact add \
  --db "$WORK_DIR/bob.db" \
  --server "$SERVER_URL" \
  --name Alice \
  --public "$WORK_DIR/alice.public.json"

echo "Sending Alice -> Bob through DB-backed workflow..."
cargo run -p messenger-dev -- message send \
  --db "$WORK_DIR/alice.db" \
  --server "$SERVER_URL" \
  --to Bob \
  --text "$MESSAGE"

echo "Syncing Bob..."
SYNC_OUTPUT="$(cargo run -p messenger-dev -- sync --db "$WORK_DIR/bob.db" --server "$SERVER_URL")"
echo "$SYNC_OUTPUT"

if [[ "$SYNC_OUTPUT" != *"$MESSAGE"* ]]; then
  echo "Expected decrypted message not found in sync output."
  exit 1
fi

echo "Checking Bob message history..."
HISTORY_OUTPUT="$(cargo run -p messenger-dev -- messages list --db "$WORK_DIR/bob.db" --server "$SERVER_URL" --contact Alice)"
echo "$HISTORY_OUTPUT"

if [[ "$HISTORY_OUTPUT" != *"$MESSAGE"* ]]; then
  echo "Expected decrypted message not found in message history."
  exit 1
fi

echo "DB-backed relay smoke test passed."
