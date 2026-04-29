# Build script for iOS targets
# Usage: .\build-ios.ps1 [-target device|sim|sim-intel] [-release]

param(
    [ValidateSet('device', 'sim', 'sim-intel')]
    [string]$target = 'device',
    [switch]$release = $false
)

Write-Host "Building Book App for iOS..." -ForegroundColor Green

# Map target names to Rust targets
$targetMap = @{
    'device'    = 'aarch64-apple-ios'
    'sim'       = 'aarch64-apple-ios-sim'
    'sim-intel' = 'x86_64-apple-ios'
}

$rustTarget = $targetMap[$target]
$buildType = $release ? '--release' : ''

Write-Host "Target: $target ($rustTarget)" -ForegroundColor Cyan
Write-Host "Build type: $(if ($release) { 'Release' } else { 'Debug' })" -ForegroundColor Cyan

# Check for Xcode
Write-Host "Checking for Xcode..." -ForegroundColor Yellow
$xcode = xcode-select --print-path 2>$null
if (-not $xcode) {
    Write-Host "ERROR: Xcode not found. Please install Xcode Command Line Tools." -ForegroundColor Red
    exit 1
}
Write-Host "Found Xcode at: $xcode" -ForegroundColor Green

# Ensure target is installed
Write-Host "Ensuring Rust target is installed..." -ForegroundColor Yellow
rustup target add $rustTarget

# Build
Write-Host "Building..." -ForegroundColor Yellow
$buildCommand = "cargo build --target $rustTarget $buildType"
Invoke-Expression $buildCommand

if ($LASTEXITCODE -eq 0) {
    $outDir = if ($release) { 'release' } else { 'debug' }
    $libPath = "target\$rustTarget\$outDir\libbook_slint.a"

    Write-Host "`nBuild successful!" -ForegroundColor Green
    Write-Host "Output: $libPath" -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Yellow

    switch ($target) {
        'device' {
            Write-Host "1. Open Xcode: `cargo xcode`"
            Write-Host "2. Select provisioning profile for device"
            Write-Host "3. Build and run on connected device"
        }
        'sim' {
            Write-Host "1. Start iOS simulator"
            Write-Host "2. Run: cargo run --target aarch64-apple-ios-sim"
        }
        'sim-intel' {
            Write-Host "1. Start iOS simulator"
            Write-Host "2. Run: cargo run --target x86_64-apple-ios"
        }
    }
} else {
    Write-Host "`nBuild failed!" -ForegroundColor Red
    exit 1
}
