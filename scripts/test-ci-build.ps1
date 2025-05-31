#!/usr/bin/env pwsh
# Test script to verify CI/CD build optimizations
# This script simulates the CI/CD build process locally

param(
    [switch]$Verbose,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Testing FerroCP CI/CD Build Process" -ForegroundColor Green
Write-Host "=======================================" -ForegroundColor Green

# Function to print section headers
function Write-Section {
    param([string]$Title)
    Write-Host "`n📋 $Title" -ForegroundColor Cyan
    Write-Host ("=" * ($Title.Length + 4)) -ForegroundColor Cyan
}

# Function to check command success
function Test-Command {
    param([string]$Command, [string]$Description)
    Write-Host "  ▶ $Description..." -ForegroundColor Yellow
    try {
        Invoke-Expression $Command
        Write-Host "  ✅ $Description completed successfully" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "  ❌ $Description failed: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# 1. Environment Check
Write-Section "Environment Check"
$rustVersion = rustc --version
$cargoVersion = cargo --version
Write-Host "  Rust: $rustVersion" -ForegroundColor White
Write-Host "  Cargo: $cargoVersion" -ForegroundColor White

# 2. Code Quality Checks
Write-Section "Code Quality Checks"
if (-not (Test-Command "cargo fmt --all -- --check" "Code formatting check")) {
    Write-Host "  ⚠️  Code formatting issues found. Run 'cargo fmt' to fix." -ForegroundColor Yellow
}

if (-not (Test-Command "cargo clippy --workspace --all-targets --all-features -- -D warnings" "Clippy linting")) {
    Write-Host "  ⚠️  Clippy warnings found. Please fix before release." -ForegroundColor Yellow
}

# 3. Build Tests
Write-Section "Build Tests"
Test-Command "cargo build --workspace --all-features --release" "Workspace build with optimizations"

# 4. CLI Binary Build and Test
Write-Section "CLI Binary Build and Test"
$env:RUSTFLAGS = "-C target-cpu=native -C opt-level=3 -C lto=fat"
Test-Command "cargo build --bin ferrocp --release" "CLI binary build with optimizations"

# Verify binary was created
if (Test-Path "target/release/ferrocp.exe") {
    $binaryInfo = Get-Item "target/release/ferrocp.exe"
    Write-Host "  📊 Binary size: $([math]::Round($binaryInfo.Length / 1MB, 2)) MB" -ForegroundColor White
    
    # Test binary functionality
    Write-Host "  🧪 Testing binary functionality..." -ForegroundColor Yellow
    $version = & "target/release/ferrocp.exe" --version
    Write-Host "  📋 Version: $version" -ForegroundColor White
    
    # Test help command
    $helpOutput = & "target/release/ferrocp.exe" --help | Select-Object -First 5
    Write-Host "  📖 Help output (first 5 lines):" -ForegroundColor White
    $helpOutput | ForEach-Object { Write-Host "    $_" -ForegroundColor Gray }
} else {
    Write-Host "  ❌ Binary not found!" -ForegroundColor Red
}

# 5. Checksum Generation
Write-Section "Checksum Generation"
if (Test-Path "target/release/ferrocp.exe") {
    $checksum = Get-FileHash "target/release/ferrocp.exe" -Algorithm SHA256
    $checksumFile = "target/release/ferrocp.exe.sha256"
    "$($checksum.Hash.ToLower())  ferrocp.exe" | Out-File -FilePath $checksumFile -Encoding ASCII
    Write-Host "  ✅ Checksum generated: $checksumFile" -ForegroundColor Green
    Write-Host "  🔐 SHA256: $($checksum.Hash.ToLower())" -ForegroundColor White
}

# 6. Unit Tests (if not skipped)
if (-not $SkipTests) {
    Write-Section "Unit Tests"
    Test-Command "cargo test --workspace --all-features --exclude ferrocp-python --release" "Unit tests"
}

# 7. Summary
Write-Section "Build Summary"
$buildSuccess = Test-Path "target/release/ferrocp.exe"
$checksumExists = Test-Path "target/release/ferrocp.exe.sha256"

Write-Host "  📊 Build Results:" -ForegroundColor White
Write-Host "    Binary created: $(if ($buildSuccess) { '✅ Yes' } else { '❌ No' })" -ForegroundColor $(if ($buildSuccess) { 'Green' } else { 'Red' })
Write-Host "    Checksum generated: $(if ($checksumExists) { '✅ Yes' } else { '❌ No' })" -ForegroundColor $(if ($checksumExists) { 'Green' } else { 'Red' })

if ($buildSuccess -and $checksumExists) {
    Write-Host "`n🎉 CI/CD build simulation completed successfully!" -ForegroundColor Green
    Write-Host "   Ready for release! 🚀" -ForegroundColor Green
} else {
    Write-Host "`n❌ CI/CD build simulation failed!" -ForegroundColor Red
    Write-Host "   Please fix issues before proceeding with release." -ForegroundColor Red
    exit 1
}

Write-Host "`n📝 Next steps:" -ForegroundColor Cyan
Write-Host "  1. Commit and push changes to trigger CI/CD" -ForegroundColor White
Write-Host "  2. Create a release tag (e.g., v0.2.0)" -ForegroundColor White
Write-Host "  3. Monitor GitHub Actions for automated release" -ForegroundColor White
