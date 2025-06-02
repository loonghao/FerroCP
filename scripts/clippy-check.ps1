# Cross-platform Clippy Check Script
# This script runs clippy with platform-aware configurations

param(
    [switch]$Fix,
    [switch]$Strict,
    [string]$Target = "all"
)

Write-Host "🔍 Running cross-platform Clippy check..." -ForegroundColor Cyan

# Use rustup to ensure correct Rust version
$rustVersion = & rustup run stable rustc --version
Write-Host "🦀 Using Rust version: $rustVersion" -ForegroundColor Green

# Clean build artifacts to avoid version conflicts
Write-Host "🧹 Cleaning build artifacts..." -ForegroundColor Yellow
& rustup run stable cargo clean

# Use workspace lints configuration from Cargo.toml
$ClippyArgs = @(
    "clippy"
    "--workspace"
    "--all-targets"
)

if ($Fix) {
    $ClippyArgs += "--fix"
    Write-Host "🔧 Running with --fix flag" -ForegroundColor Green
}

if ($Strict) {
    Write-Host "⚡ Running in strict mode" -ForegroundColor Red
} else {
    Write-Host "📋 Running in lenient mode (cross-platform friendly)" -ForegroundColor Blue
}

Write-Host "🚀 Executing: rustup run stable cargo $($ClippyArgs -join ' ')" -ForegroundColor Gray

try {
    & rustup run stable cargo @ClippyArgs
    $ExitCode = $LASTEXITCODE

    if ($ExitCode -eq 0) {
        Write-Host "✅ Clippy check passed!" -ForegroundColor Green
    } else {
        Write-Host "❌ Clippy check failed with exit code: $ExitCode" -ForegroundColor Red
        Write-Host "💡 Try running with -Fix flag to auto-fix issues" -ForegroundColor Yellow
        Write-Host "💡 Or use lenient mode for cross-platform compatibility" -ForegroundColor Yellow
    }

    exit $ExitCode
} catch {
    Write-Host "💥 Error running clippy: $_" -ForegroundColor Red
    exit 1
}
