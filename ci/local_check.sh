#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "Running cargo fmt..."
cargo fmt --all -- --check

echo "Running cargo clippy..."
cargo clippy --workspace --all-targets -- -D warnings

echo "Running cargo clippy (wasm32-wasip2)..."
cargo clippy --workspace --all-targets --target wasm32-wasip2 -- -D warnings

echo "Running cargo test..."
cargo test --workspace --all-targets

echo "Building wasm32-wasip2 (release)..."
cargo build --target wasm32-wasip2 --release

if command -v greentic-integration-tester >/dev/null 2>&1; then
  echo "Running README gtests..."
  greentic-integration-tester run --gtest tests/gtests/README --artifacts-dir artifacts/readme-gtests --errors
else
  echo "greentic-integration-tester not found; skipping README gtests."
fi

echo "All checks passed."
