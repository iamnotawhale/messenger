#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLUTTER_DIR="$ROOT_DIR/apps/flutter"
GENERATED_DART_API="$ROOT_DIR/apps/flutter/lib/src/bridge/generated/api.dart"
GENERATED_DART_BRIDGE="$ROOT_DIR/apps/flutter/lib/src/bridge/generated/bridge_generated.dart"
GENERATED_C_HEADER="$ROOT_DIR/apps/flutter/rust/generated/frb_generated.h"

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
PLACEHOLDER_BACKUP=""
if [[ -f "$GENERATED_DART_API" ]]; then
  PLACEHOLDER_BACKUP="$(mktemp)"
  cp "$GENERATED_DART_API" "$PLACEHOLDER_BACKUP"
fi

rm -rf "$GENERATED_DART_API"
rm -f "$GENERATED_DART_BRIDGE"
rm -f "$GENERATED_C_HEADER"
flutter_rust_bridge_codegen generate

if [[ ! -f "$GENERATED_DART_API" && -n "$PLACEHOLDER_BACKUP" ]]; then
  rm -rf "$GENERATED_DART_API"
  cp "$PLACEHOLDER_BACKUP" "$GENERATED_DART_API"
  echo "flutter_rust_bridge did not emit generated/api.dart; restored analyze placeholder."
fi
if [[ -n "$PLACEHOLDER_BACKUP" ]]; then
  rm -f "$PLACEHOLDER_BACKUP"
fi

echo "flutter_rust_bridge bindings generated."
