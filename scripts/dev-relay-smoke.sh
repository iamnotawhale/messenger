#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d)"
SERVER_LOG="$WORK_DIR/server.log"
MESSAGE="hello from relay smoke"
SERVER_URL="http://127.0.0.1:8080"
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

echo "Starting local relay server..."
MESSENGER_SQLITE_PATH="$WORK_DIR/relay.db" \
  "$ROOT_DIR/target/debug/messenger-server" >"$SERVER_LOG" 2>&1 &
SERVER_PID="$!"

for _ in $(seq 1 50); do
  if bash -c 'exec 3<>/dev/tcp/127.0.0.1/8080' 2>/dev/null; then
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

echo "Creating Alice and Bob identities..."
cargo run -p messenger-dev -- identity new "$WORK_DIR/alice.json"
cargo run -p messenger-dev -- identity public "$WORK_DIR/alice.json" "$WORK_DIR/alice.public.json"
cargo run -p messenger-dev -- identity new "$WORK_DIR/bob.json"
cargo run -p messenger-dev -- identity public "$WORK_DIR/bob.json" "$WORK_DIR/bob.public.json"

echo "Sending Alice -> Bob..."
cargo run -p messenger-dev -- send \
  --server "$SERVER_URL" \
  --from "$WORK_DIR/alice.json" \
  --to "$WORK_DIR/bob.public.json" \
  --text "$MESSAGE"

echo "Receiving as Bob..."
RECEIVE_OUTPUT="$(cargo run -p messenger-dev -- receive \
  --server "$SERVER_URL" \
  --identity "$WORK_DIR/bob.json" \
  --from "$WORK_DIR/alice.public.json")"
echo "$RECEIVE_OUTPUT"

if [[ "$RECEIVE_OUTPUT" != *"$MESSAGE"* ]]; then
  echo "Expected decrypted message not found in receive output."
  exit 1
fi

echo "Checking Bob queue is empty after delivery..."
EMPTY_OUTPUT="$(cargo run -p messenger-dev -- receive \
  --server "$SERVER_URL" \
  --identity "$WORK_DIR/bob.json" \
  --from "$WORK_DIR/alice.public.json")"
echo "$EMPTY_OUTPUT"

if [[ "$EMPTY_OUTPUT" != *"no pending messages"* ]]; then
  echo "Expected empty queue after delivery."
  exit 1
fi

echo "Relay smoke test passed."
