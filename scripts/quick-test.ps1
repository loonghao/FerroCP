# Quick Test Script for FerroCP
# Tests basic compilation without clippy

param(
    [string]$Package = "ferrocp-types"
)

Write-Host "🚀 Quick test for FerroCP" -ForegroundColor Cyan
Write-Host "📦 Testing package: $Package" -ForegroundColor Green

# Use rustup to ensure correct Rust version
$rustVersion = & rustup run stable rustc --version
Write-Host "🦀 Using Rust version: $rustVersion" -ForegroundColor Yellow

try {
    Write-Host "🔍 Running cargo check..." -ForegroundColor Blue
    & rustup run stable cargo check --package $Package
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Package $Package compiled successfully!" -ForegroundColor Green
    } else {
        Write-Host "❌ Package $Package failed to compile" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} catch {
    Write-Host "💥 Error: $_" -ForegroundColor Red
    exit 1
}

Write-Host "🎉 Quick test completed!" -ForegroundColor Cyan
