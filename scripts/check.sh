#!/usr/bin/env bash
# One-step developer pre-flight code quality check script.
# Usage: ./scripts/check.sh

set -euo pipefail

echo "==> [1/4] Checking code formatting (cargo fmt)..."
cargo fmt --all -- --check

echo "==> [2/4] Checking compilation (cargo check)..."
cargo check --workspace --all-targets

echo "==> [3/4] Running linter (cargo clippy)..."
cargo clippy --workspace --all-targets -- -D warnings

echo "==> [4/4] Running automated test suite (cargo test)..."
cargo test --workspace

echo "==> All quality checks PASSED! Code is in pristine condition."
