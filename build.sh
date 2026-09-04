#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
cd "$root"

echo "Testing HUD library..."
cargo test --manifest-path overlay/Cargo.toml --workspace

echo "Building overlay (macOS stub)..."
cargo build --release --manifest-path overlay/Cargo.toml

echo "Done. The overlay binary requires Windows; use 'cargo test' for HUD development on macOS."
