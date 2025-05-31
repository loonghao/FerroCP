# Cross-platform Clippy Check Script
# This script runs clippy with platform-aware configurations

param(
    [switch]$Fix,
    [switch]$Strict,
    [string]$Target = "all"
)

Write-Host "🔍 Running cross-platform Clippy check..." -ForegroundColor Cyan

# Clean build artifacts to avoid version conflicts
Write-Host "🧹 Cleaning build artifacts..." -ForegroundColor Yellow
cargo clean

# Basic clippy check with cross-platform friendly settings
$ClippyArgs = @(
    "clippy"
    "--workspace"
    "--all-targets"
    "--"
    "-A", "clippy::cargo_common_metadata"
    "-A", "clippy::module_name_repetitions"
    "-A", "clippy::missing_errors_doc"
    "-A", "clippy::missing_panics_doc"
    "-A", "clippy::too_many_arguments"
    "-A", "clippy::too_many_lines"
    "-A", "clippy::similar_names"
)

if ($Fix) {
    $ClippyArgs += "--fix"
    Write-Host "🔧 Running with --fix flag" -ForegroundColor Green
}

if (-not $Strict) {
    # Add more lenient settings for cross-platform compatibility
    $ClippyArgs += @(
        "-A", "clippy::redundant_pub_crate"
        "-A", "clippy::wildcard_imports"
        "-A", "clippy::single_match_else"
    )
    Write-Host "📋 Running in lenient mode (cross-platform friendly)" -ForegroundColor Blue
} else {
    Write-Host "⚡ Running in strict mode" -ForegroundColor Red
}

Write-Host "🚀 Executing: cargo $($ClippyArgs -join ' ')" -ForegroundColor Gray

try {
    & cargo @ClippyArgs
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
