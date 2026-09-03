#!/usr/bin/env bash
# Quick developer pre-flight code quality check script.
# Proxies directly to the canonical `cargo xtask check` runner.
#
# Usage:
#   ./scripts/dev/check.sh          # Run standard verification
#   ./scripts/dev/check.sh --fix    # Automatically fix formatting and safe clippy suggestions
#   ./scripts/dev/check.sh -p app   # Check only a specific package
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

(cd "$PROJECT_ROOT" && cargo xtask check "$@")
