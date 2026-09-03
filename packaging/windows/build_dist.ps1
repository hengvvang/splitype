# Build and package a standalone Windows distribution for splitype.
# Usage: powershell -ExecutionPolicy Bypass -File packaging/windows/build_dist.ps1 [-Profile release|debug] [-NoArchive]

[CmdletBinding()]
param (
    [ValidateSet("release", "debug")]
    [string]$Profile = "release",

    [switch]$NoArchive
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = (Resolve-Path "$ScriptDir/../..").Path
$DistDir = Join-Path $ProjectRoot "dist"
$PackageName = "splitype-windows-x64"
$StageDir = Join-Path $DistDir $PackageName

Write-Host "==> Cleaning old distribution artifacts..." -ForegroundColor Cyan
if (Test-Path $StageDir) {
    Remove-Item -Recurse -Force $StageDir
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

Write-Host "==> Compiling $Profile binary (crates/app)..." -ForegroundColor Cyan
$CargoArgs = @("build", "--manifest-path", "$ProjectRoot/Cargo.toml", "-p", "app")
if ($Profile -eq "release") {
    $CargoArgs += "--release"
}
& cargo @CargoArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo build failed with exit code $LASTEXITCODE"
    exit $LASTEXITCODE
}

Write-Host "==> Assembling Windows distribution files..." -ForegroundColor Cyan
$BinaryDir = Join-Path $ProjectRoot "target/$Profile"
$SourceExe = if (Test-Path "$BinaryDir/splitype.exe") {
    "$BinaryDir/splitype.exe"
} else {
    "$BinaryDir/app.exe"
}

if (-not (Test-Path $SourceExe)) {
    Write-Error "Cannot find compiled binary at: $SourceExe"
    exit 1
}

# 1. Main executable (renamed to splitype.exe)
$TargetExe = Join-Path $StageDir "splitype.exe"
Copy-Item -Path $SourceExe -Destination $TargetExe -Force
Write-Host "    Deployed: splitype.exe" -ForegroundColor Gray

# 2. Side-by-side application manifest (UTF-8 codepage, Long paths, PerMonitorV2 DPI)
$SourceManifest = Join-Path $ScriptDir "splitype.manifest"
if (Test-Path $SourceManifest) {
    Copy-Item -Path $SourceManifest -Destination "$TargetExe.manifest" -Force
    Write-Host "    Deployed: splitype.exe.manifest (side-by-side activation context)" -ForegroundColor Gray
}

# 3. Application icon
$SourceIco = Join-Path $ScriptDir "splitype.ico"
if (Test-Path $SourceIco) {
    Copy-Item -Path $SourceIco -Destination (Join-Path $StageDir "splitype.ico") -Force
    Write-Host "    Deployed: splitype.ico" -ForegroundColor Gray
}

# 4. Project README and LICENSE
foreach ($doc in @("README.md", "LICENSE")) {
    $docPath = Join-Path $ProjectRoot $doc
    if (Test-Path $docPath) {
        Copy-Item -Path $docPath -Destination (Join-Path $StageDir $doc) -Force
    }
}

if (-not $NoArchive) {
    $ZipPath = Join-Path $DistDir "$PackageName.zip"
    Write-Host "==> Creating distribution archive: $ZipPath..." -ForegroundColor Cyan
    if (Test-Path $ZipPath) {
        Remove-Item -Force $ZipPath
    }
    Compress-Archive -Path "$StageDir/*" -DestinationPath $ZipPath -Force
    Write-Host "==> Finished archive: $ZipPath" -ForegroundColor Green
}

Write-Host "==> Application distribution staged successfully at: $StageDir" -ForegroundColor Green
Write-Host "    Launch executable directly: $TargetExe" -ForegroundColor Yellow
