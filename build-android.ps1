# Build script for Android targets
# Usage: .\build-android.ps1 [-target arm64|arm32|x86|x86_64] [-release]

param(
    [ValidateSet('arm64', 'arm32', 'x86', 'x86_64')]
    [string]$target = 'arm64',
    [switch]$release = $false
)

Write-Host "Building Book App for Android..." -ForegroundColor Green

# Map target names to Rust targets
$targetMap = @{
    'arm64'  = 'aarch64-linux-android'
    'arm32'  = 'armv7-linux-android'
    'x86'    = 'i686-linux-android'
    'x86_64' = 'x86_64-linux-android'
}

$rustTarget = $targetMap[$target]
$buildType = $release ? '--release' : ''

Write-Host "Target: $target ($rustTarget)" -ForegroundColor Cyan
Write-Host "Build type: $(if ($release) { 'Release' } else { 'Debug' })" -ForegroundColor Cyan

# Ensure target is installed
Write-Host "Ensuring Rust target is installed..." -ForegroundColor Yellow
rustup target add $rustTarget

# Build
Write-Host "Building..." -ForegroundColor Yellow
$buildCommand = "cargo build --target $rustTarget $buildType"
Invoke-Expression $buildCommand

if ($LASTEXITCODE -eq 0) {
    $outDir = if ($release) { 'release' } else { 'debug' }
    $libPath = "target\$rustTarget\$outDir\libbook_slint.so"

    Write-Host "`nBuild successful!" -ForegroundColor Green
    Write-Host "Output: $libPath" -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Yellow
    Write-Host "1. Create Android project in android/ directory"
    Write-Host "2. Update build.gradle with native library path"
    Write-Host "3. Build APK with 'gradle build'"
    Write-Host "4. Deploy with 'adb install app/build/outputs/apk/release/app-release.apk'"
} else {
    Write-Host "`nBuild failed!" -ForegroundColor Red
    exit 1
}
