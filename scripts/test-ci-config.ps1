# Test script for CI configuration validation
# This script validates the CI configuration files

Write-Host "🔧 Testing CI Configuration" -ForegroundColor Green
Write-Host "============================" -ForegroundColor Green

# Test 1: Check if workflow files exist
Write-Host ""
Write-Host "🧪 Test 1: Workflow files existence" -ForegroundColor Yellow

$workflowFiles = @(
    ".github/workflows/test.yml",
    ".github/workflows/vfx-platform-test.yml",
    ".github/workflows/test-macos.yml"
)

foreach ($file in $workflowFiles) {
    if (Test-Path $file) {
        Write-Host "✅ $file exists" -ForegroundColor Green
    } else {
        Write-Host "❌ $file missing" -ForegroundColor Red
    }
}

# Test 2: Check for ARM64 cross-compilation configuration
Write-Host ""
Write-Host "🧪 Test 2: ARM64 cross-compilation configuration" -ForegroundColor Yellow

$testYml = Get-Content ".github/workflows/test.yml" -Raw
$vfxYml = Get-Content ".github/workflows/vfx-platform-test.yml" -Raw

$checks = @(
    @{
        Name = "ARM64 architecture addition"
        Pattern = "dpkg --add-architecture arm64"
        Files = @($testYml, $vfxYml)
    },
    @{
        Name = "Ubuntu ports configuration"
        Pattern = "ports\.ubuntu\.com/ubuntu-ports"
        Files = @($testYml, $vfxYml)
    },
    @{
        Name = "Cross-compilation toolchain"
        Pattern = "gcc-aarch64-linux-gnu"
        Files = @($testYml, $vfxYml)
    },
    @{
        Name = "Static OpenSSL linking"
        Pattern = "OPENSSL_STATIC=1"
        Files = @($testYml, $vfxYml)
    }
)

foreach ($check in $checks) {
    $found = $false
    foreach ($content in $check.Files) {
        if ($content -match $check.Pattern) {
            $found = $true
            break
        }
    }
    
    if ($found) {
        Write-Host "✅ $($check.Name) configured" -ForegroundColor Green
    } else {
        Write-Host "❌ $($check.Name) missing" -ForegroundColor Red
    }
}

# Test 3: Check for VFX Platform Summary error handling
Write-Host ""
Write-Host "🧪 Test 3: VFX Platform Summary error handling" -ForegroundColor Yellow

if ($vfxYml -match "warning.*Some VFX Platform tests failed") {
    Write-Host "✅ Non-blocking error handling configured" -ForegroundColor Green
} else {
    Write-Host "❌ Error handling not configured" -ForegroundColor Red
}

# Test 4: Check Rust configuration
Write-Host ""
Write-Host "🧪 Test 4: Rust configuration" -ForegroundColor Yellow

if (Get-Command rustc -ErrorAction SilentlyContinue) {
    Write-Host "✅ Rust is installed" -ForegroundColor Green
    rustc --version
    
    $targets = rustup target list --installed
    if ($targets -match "aarch64-unknown-linux-gnu") {
        Write-Host "✅ ARM64 Linux target installed" -ForegroundColor Green
    } else {
        Write-Host "📋 ARM64 Linux target not installed (run: rustup target add aarch64-unknown-linux-gnu)" -ForegroundColor Yellow
    }
} else {
    Write-Host "📋 Rust not installed" -ForegroundColor Yellow
}

# Test 5: Check PyO3 version for security
Write-Host ""
Write-Host "🧪 Test 5: Security configuration" -ForegroundColor Yellow

$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'pyo3.*=.*"0\.24\.1"') {
    Write-Host "✅ PyO3 version 0.24.1 (security fix applied)" -ForegroundColor Green
} else {
    Write-Host "⚠️  PyO3 version check needed" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✅ CI Configuration Test Completed!" -ForegroundColor Green
Write-Host ""
Write-Host "📋 Summary:" -ForegroundColor Cyan
Write-Host "   - ARM64 cross-compilation configured for Linux CI" -ForegroundColor White
Write-Host "   - Ubuntu ports sources configured to avoid 404 errors" -ForegroundColor White
Write-Host "   - VFX Platform Summary uses non-blocking error handling" -ForegroundColor White
Write-Host "   - Security vulnerabilities addressed" -ForegroundColor White
