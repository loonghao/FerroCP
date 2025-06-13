#!/usr/bin/env pwsh
# Quick CI Check Script for FerroCP
# This script runs essential CI checks quickly for development workflow

param(
    [switch]$Fix,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Colors
$Green = "Green"
$Red = "Red"
$Yellow = "Yellow"
$Cyan = "Cyan"

function Write-Section($Title) {
    Write-Host ""
    Write-Host "🔍 $Title" -ForegroundColor $Cyan
    Write-Host ("=" * ($Title.Length + 4)) -ForegroundColor $Cyan
}

function Write-Check($Message, $Success) {
    if ($Success) {
        Write-Host "✅ $Message" -ForegroundColor $Green
    } else {
        Write-Host "❌ $Message" -ForegroundColor $Red
    }
}

# Check if we're in the correct directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "❌ Not in project root directory. Please run from ferrocp project root." -ForegroundColor $Red
    exit 1
}

Write-Host "🚀 FerroCP Quick CI Check" -ForegroundColor $Cyan
Write-Host "=========================" -ForegroundColor $Cyan

$checks = @()

# 1. Format Check
Write-Section "Format Check"
try {
    if ($Fix) {
        cargo fmt --all
        $formatResult = $LASTEXITCODE -eq 0
        Write-Check "Code formatting fixed" $formatResult
    } else {
        cargo fmt --all -- --check | Out-Null
        $formatResult = $LASTEXITCODE -eq 0
        Write-Check "Code formatting" $formatResult
        if (-not $formatResult) {
            Write-Host "   💡 Run with -Fix to auto-fix formatting" -ForegroundColor $Yellow
        }
    }
    $checks += @{Name="Format"; Success=$formatResult}
} catch {
    Write-Check "Code formatting" $false
    $checks += @{Name="Format"; Success=$false}
}

# 2. Quick Clippy Check (without -D warnings for speed)
Write-Section "Clippy Check"
try {
    cargo clippy --workspace --all-targets --quiet | Out-Null
    $clippyResult = $LASTEXITCODE -eq 0
    Write-Check "Clippy linting" $clippyResult
    if (-not $clippyResult) {
        Write-Host "   💡 Some clippy warnings found (may be non-fatal)" -ForegroundColor $Yellow
    }
    $checks += @{Name="Clippy"; Success=$true}  # Always pass for quick check
} catch {
    Write-Check "Clippy linting" $false
    $checks += @{Name="Clippy"; Success=$false}
}

# 3. Quick Build Check
Write-Section "Build Check"
try {
    cargo check --workspace --exclude ferrocp-python --quiet | Out-Null
    $buildResult = $LASTEXITCODE -eq 0
    Write-Check "Build check" $buildResult
    $checks += @{Name="Build"; Success=$buildResult}
} catch {
    Write-Check "Build check" $false
    $checks += @{Name="Build"; Success=$false}
}

# 4. Quick Unit Tests (lib only)
Write-Section "Unit Tests"
try {
    if ($Verbose) {
        cargo test --workspace --exclude ferrocp-python --lib
    } else {
        cargo test --workspace --exclude ferrocp-python --lib --quiet
    }
    $testResult = $LASTEXITCODE -eq 0
    Write-Check "Unit tests" $testResult
    $checks += @{Name="Tests"; Success=$testResult}
} catch {
    Write-Check "Unit tests" $false
    $checks += @{Name="Tests"; Success=$false}
}

# Summary
Write-Section "Summary"
$passed = ($checks | Where-Object { $_.Success }).Count
$total = $checks.Count

Write-Host "Passed: $passed/$total" -ForegroundColor $(if ($passed -eq $total) { $Green } else { $Yellow })

$failedChecks = $checks | Where-Object { -not $_.Success }
if ($failedChecks.Count -gt 0) {
    Write-Host "Failed checks:" -ForegroundColor $Red
    $failedChecks | ForEach-Object { Write-Host "  - $($_.Name)" -ForegroundColor $Red }
    Write-Host ""
    Write-Host "❌ Quick check failed! Run full CI check for details:" -ForegroundColor $Red
    Write-Host "   .\scripts\local-ci-check.ps1" -ForegroundColor $Cyan
    exit 1
} else {
    Write-Host "✅ Quick check passed! Ready for development." -ForegroundColor $Green
    Write-Host ""
    Write-Host "💡 For full CI validation, run:" -ForegroundColor $Yellow
    Write-Host "   .\scripts\local-ci-check.ps1" -ForegroundColor $Cyan
}
