#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"
APP_BUNDLE="${HOME}/Applications/WordMock.app"

CARGO_HTTP_MULTIPLEXING=false cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
cp "$PROJECT_ROOT/target/release/wordmock" "$APP_BUNDLE/Contents/MacOS/wordmock"
cp "$PROJECT_ROOT/packaging/macos/Info.plist" "$APP_BUNDLE/Contents/Info.plist"
chmod +x "$APP_BUNDLE/Contents/MacOS/wordmock"

printf 'Installed WordMock to %s\n' "$APP_BUNDLE"
