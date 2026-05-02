#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_DIR="$ROOT_DIR/apps/flutter"

require_command() {
  local command_name="$1"
  local install_hint="$2"

  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "Missing required command: $command_name" >&2
    echo "$install_hint" >&2
    exit 1
  fi
}

require_command flutter "Install Flutter SDK and ensure 'flutter' is on PATH."

cd "$FLUTTER_DIR"

flutter create \
  --project-name messenger_app \
  --org dev.messenger \
  --platforms=android,linux,macos,windows \
  .

flutter pub get

echo "Flutter desktop and Android platform directories are ready."
