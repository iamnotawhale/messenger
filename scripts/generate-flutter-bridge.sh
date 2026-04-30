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
require_command dart "Install Flutter SDK and ensure 'dart' is on PATH."
require_command flutter_rust_bridge_codegen \
  "Install with: cargo install flutter_rust_bridge_codegen"

cd "$FLUTTER_DIR"
flutter pub get

cd "$ROOT_DIR"
rm -f "$ROOT_DIR/apps/flutter/lib/src/bridge/generated/api.dart"
rm -f "$ROOT_DIR/apps/flutter/lib/src/bridge/generated/bridge_generated.dart"
rm -f "$ROOT_DIR/apps/flutter/rust/generated/frb_generated.h"
flutter_rust_bridge_codegen generate

echo "flutter_rust_bridge bindings generated."
