<#
.SYNOPSIS
    Quick developer pre-flight code quality check script for Windows.
.DESCRIPTION
    Proxies directly to the workspace-level `cargo xtask check` task runner.
.EXAMPLE
    .\scripts\dev\check.ps1
    .\scripts\dev\check.ps1 --fix
    .\scripts\dev\check.ps1 -p app
#>

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = (Resolve-Path "$ScriptDir\..\..").Path

Push-Location $ProjectRoot
try {
    cargo xtask check @args
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}
