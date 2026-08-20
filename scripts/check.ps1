<#
.SYNOPSIS
    One-step developer pre-flight code quality check script.
.DESCRIPTION
    Runs cargo fmt check, cargo check-all, cargo clippy (zero warnings), and cargo test.
#>

$ErrorActionPreference = "Stop"

Write-Host "==> [1/4] Checking code formatting (cargo fmt)..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo fmt failed! Run 'cargo fmt --all' to fix formatting."
    exit 1
}

Write-Host "==> [2/4] Checking compilation (cargo check)..." -ForegroundColor Cyan
cargo check --workspace --all-targets
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo check failed!"
    exit 1
}

Write-Host "==> [3/4] Running linter (cargo clippy)..." -ForegroundColor Cyan
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo clippy found warnings/errors!"
    exit 1
}

Write-Host "==> [4/4] Running automated test suite (cargo test)..." -ForegroundColor Cyan
cargo test --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo test failed!"
    exit 1
}

Write-Host "==> All quality checks PASSED! Code is in pristine condition." -ForegroundColor Green
