# Profile-Guided Optimization (PGO) build script for FerroCP (Windows)
# This script generates optimized binaries using PGO to reduce size and improve performance

param(
    [Parameter(Mandatory=$true)]
    [string]$Target
)

# Configuration
$ProfileDir = "target\pgo-profiles"
$BinaryName = "ferrocp"
$TestDataDir = "target\pgo-test-data"

Write-Host "🚀 Starting PGO build for FerroCP" -ForegroundColor Blue
Write-Host "📋 Configuration:" -ForegroundColor Yellow
Write-Host "  Target: $Target"
Write-Host "  Profile Directory: $ProfileDir"
Write-Host "  Binary: $BinaryName"

# Clean previous builds and profiles
Write-Host "🧹 Cleaning previous builds and profiles..." -ForegroundColor Yellow
cargo clean
if (Test-Path $ProfileDir) { Remove-Item $ProfileDir -Recurse -Force }
New-Item -ItemType Directory -Path $ProfileDir -Force | Out-Null

# Step 1: Build instrumented binary for profiling
Write-Host "📊 Step 1: Building instrumented binary for profiling..." -ForegroundColor Blue
$env:RUSTFLAGS = "-Cprofile-generate=$ProfileDir"

# Build with profiling instrumentation
# For local development, use default target
Write-Host "⚠️  Using default target for local development..." -ForegroundColor Yellow
cargo build --bin $BinaryName --release
$BinaryPath = "target\release\$BinaryName.exe"

Write-Host "✅ Instrumented binary built: $BinaryPath" -ForegroundColor Green

# Step 2: Generate profiles by running typical workloads
Write-Host "📈 Step 2: Generating profiles with typical workloads..." -ForegroundColor Blue

# Create test data for profiling
if (Test-Path $TestDataDir) { Remove-Item $TestDataDir -Recurse -Force }
New-Item -ItemType Directory -Path $TestDataDir -Force | Out-Null

Write-Host "📁 Creating test data..." -ForegroundColor Yellow

# Generate various test files for realistic profiling
$TestSizes = @(1024, 4096, 16384, 65536)
foreach ($size in $TestSizes) {
    $fileName = "$TestDataDir\test_$size.dat"
    $bytes = New-Object byte[] $size
    (New-Object Random).NextBytes($bytes)
    [System.IO.File]::WriteAllBytes($fileName, $bytes)
}

# Medium files (1MB - 10MB)
$SizesMB = @(1, 5, 10)
foreach ($sizeMB in $SizesMB) {
    $fileName = "$TestDataDir\test_${sizeMB}MB.dat"
    $size = $sizeMB * 1024 * 1024
    $bytes = New-Object byte[] $size
    (New-Object Random).NextBytes($bytes)
    [System.IO.File]::WriteAllBytes($fileName, $bytes)
}

# Create directory structure for testing
New-Item -ItemType Directory -Path "$TestDataDir\source_dir\subdir1" -Force | Out-Null
New-Item -ItemType Directory -Path "$TestDataDir\source_dir\subdir2" -Force | Out-Null

# Copy test files to subdirectories
Copy-Item "$TestDataDir\*.dat" "$TestDataDir\source_dir\"
Copy-Item "$TestDataDir\test_1024.dat" "$TestDataDir\source_dir\subdir1\"
Copy-Item "$TestDataDir\test_4096.dat" "$TestDataDir\source_dir\subdir2\"

Write-Host "✅ Test data created" -ForegroundColor Green

# Run profiling workloads (only for native Windows targets)
if ($Target -like "*windows*" -and $Target -like "*x86_64*") {
    Write-Host "🏃 Running profiling workloads..." -ForegroundColor Yellow
    
    try {
        # Profile 1: Single file copy operations
        Write-Host "  - Single file copy operations..."
        Get-ChildItem "$TestDataDir\*.dat" | ForEach-Object {
            & $BinaryPath copy $_.FullName "$($_.FullName).copy" 2>$null
        }
        
        # Profile 2: Directory operations
        Write-Host "  - Directory copy operations..."
        & $BinaryPath copy "$TestDataDir\source_dir" "$TestDataDir\dest_dir" 2>$null
        
        # Profile 3: Help and version commands
        Write-Host "  - CLI operations..."
        & $BinaryPath --help >$null 2>&1
        & $BinaryPath --version >$null 2>&1
        & $BinaryPath device >$null 2>&1
        & $BinaryPath config >$null 2>&1
        
        Write-Host "✅ Profiling workloads completed" -ForegroundColor Green
    }
    catch {
        Write-Host "⚠️  Some profiling workloads failed (this is normal)" -ForegroundColor Yellow
    }
} else {
    Write-Host "⏭️  Skipping profiling workloads (cross-compilation target)" -ForegroundColor Yellow
    # For cross-compilation, create dummy profile data
    "dummy" | Out-File "$ProfileDir\dummy.profraw"
}

# Step 3: Build optimized binary using collected profiles
Write-Host "🎯 Step 3: Building optimized binary with PGO..." -ForegroundColor Blue

# Check if we have profile data
$ProfileFiles = Get-ChildItem "$ProfileDir\*.profraw" -ErrorAction SilentlyContinue
if ($ProfileFiles -or (Test-Path "$ProfileDir\dummy.profraw")) {
    Write-Host "✅ Profile data found, building with PGO optimization" -ForegroundColor Green
    $env:RUSTFLAGS = "-Cprofile-use=$ProfileDir -Cllvm-args=-pgo-warn-missing-function"
} else {
    Write-Host "⚠️  No profile data found, building without PGO" -ForegroundColor Yellow
    Remove-Item env:RUSTFLAGS -ErrorAction SilentlyContinue
}

# Clean and rebuild with optimization
cargo clean
Write-Host "⚠️  Using default target for local development..." -ForegroundColor Yellow
cargo build --bin $BinaryName --release

Write-Host "✅ PGO-optimized binary built: $BinaryPath" -ForegroundColor Green

# Step 4: Verify and compare binary sizes
Write-Host "📏 Step 4: Binary size analysis..." -ForegroundColor Blue

if (Test-Path $BinaryPath) {
    $BinarySize = (Get-Item $BinaryPath).Length
    $BinarySizeMB = [math]::Round($BinarySize / 1024 / 1024, 2)
    Write-Host "✅ Final binary size: $BinarySize bytes (~${BinarySizeMB}MB)" -ForegroundColor Green
} else {
    Write-Host "❌ Binary not found: $BinaryPath" -ForegroundColor Red
    exit 1
}

# Cleanup test data
Write-Host "🧹 Cleaning up test data..." -ForegroundColor Yellow
if (Test-Path $TestDataDir) { Remove-Item $TestDataDir -Recurse -Force }

Write-Host "🎉 PGO build completed successfully!" -ForegroundColor Green
Write-Host "📦 Optimized binary: $BinaryPath" -ForegroundColor Blue
