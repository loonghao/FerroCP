# Profile-Guided Optimization Build Script for FerroCP
# This script performs a complete PGO build process for Windows

param(
    [string]$OutputDir = "target\pgo-release",
    [string]$ProfileDir = "pgo-data",
    [switch]$Clean = $false,
    [switch]$Verbose = $false
)

# Set error action preference
$ErrorActionPreference = "Stop"

# Colors for output
$Red = [System.ConsoleColor]::Red
$Green = [System.ConsoleColor]::Green
$Yellow = [System.ConsoleColor]::Yellow
$Blue = [System.ConsoleColor]::Blue
$White = [System.ConsoleColor]::White

function Write-ColorOutput {
    param(
        [string]$Message,
        [System.ConsoleColor]$Color = $White
    )
    $originalColor = $Host.UI.RawUI.ForegroundColor
    $Host.UI.RawUI.ForegroundColor = $Color
    Write-Output $Message
    $Host.UI.RawUI.ForegroundColor = $originalColor
}

function Write-Status {
    param([string]$Message)
    Write-ColorOutput "[INFO] $Message" $Blue
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "[SUCCESS] $Message" $Green
}

function Write-Warning {
    param([string]$Message)
    Write-ColorOutput "[WARNING] $Message" $Yellow
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "[ERROR] $Message" $Red
}

function Test-Prerequisites {
    Write-Status "Checking prerequisites..."
    
    # Check if Rust is installed
    try {
        $rustVersion = cargo --version
        Write-Status "Found Rust: $rustVersion"
    }
    catch {
        Write-Error "Rust/Cargo not found. Please install Rust toolchain."
        exit 1
    }
    
    # Check if LLVM tools are available
    try {
        $llvmProfdata = Get-Command llvm-profdata -ErrorAction SilentlyContinue
        if (-not $llvmProfdata) {
            Write-Warning "llvm-profdata not found. Installing LLVM tools..."
            rustup component add llvm-tools-preview
        }
        Write-Status "LLVM tools available"
    }
    catch {
        Write-Error "Failed to install LLVM tools. Please install manually."
        exit 1
    }
    
    Write-Success "All prerequisites satisfied"
}

function Initialize-Directories {
    Write-Status "Initializing directories..."
    
    if ($Clean -and (Test-Path $OutputDir)) {
        Write-Status "Cleaning output directory: $OutputDir"
        Remove-Item -Recurse -Force $OutputDir
    }
    
    if ($Clean -and (Test-Path $ProfileDir)) {
        Write-Status "Cleaning profile directory: $ProfileDir"
        Remove-Item -Recurse -Force $ProfileDir
    }
    
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    New-Item -ItemType Directory -Force -Path $ProfileDir | Out-Null
    
    Write-Success "Directories initialized"
}

function Build-InstrumentedBinary {
    Write-Status "Building instrumented binary for profile collection..."
    
    $env:RUSTFLAGS = "-Cprofile-generate=$ProfileDir"
    $env:CARGO_TARGET_DIR = "$OutputDir\instrumented"
    
    try {
        if ($Verbose) {
            cargo build --profile release-pgo --bin ferrocp --verbose
        } else {
            cargo build --profile release-pgo --bin ferrocp
        }
        Write-Success "Instrumented binary built successfully"
    }
    catch {
        Write-Error "Failed to build instrumented binary"
        exit 1
    }
    finally {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    }
}

function Collect-ProfileData {
    Write-Status "Collecting profile data with representative workloads..."
    
    $instrumentedBinary = "$OutputDir\instrumented\release-pgo\ferrocp.exe"
    
    if (-not (Test-Path $instrumentedBinary)) {
        Write-Error "Instrumented binary not found: $instrumentedBinary"
        exit 1
    }
    
    # Create temporary test data
    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    $sourceDir = Join-Path $tempDir "source"
    $destDir = Join-Path $tempDir "dest"
    
    New-Item -ItemType Directory -Force -Path $sourceDir | Out-Null
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    
    try {
        Write-Status "Creating test files for profile collection..."
        
        # Create various file sizes for comprehensive profiling
        $fileSizes = @(
            @{Size=1KB; Count=50; Name="small"},
            @{Size=64KB; Count=20; Name="medium"},
            @{Size=1MB; Count=10; Name="large"},
            @{Size=16MB; Count=3; Name="xlarge"}
        )
        
        foreach ($fileType in $fileSizes) {
            Write-Status "Creating $($fileType.Count) $($fileType.Name) files ($($fileType.Size) each)..."
            for ($i = 1; $i -le $fileType.Count; $i++) {
                $fileName = "$($fileType.Name)_file_$i.txt"
                $filePath = Join-Path $sourceDir $fileName
                $content = "A" * $fileType.Size
                Set-Content -Path $filePath -Value $content -NoNewline
            }
        }
        
        Write-Status "Running file copy operations for profile collection..."
        
        # Run various copy scenarios
        $scenarios = @(
            @{Source="$sourceDir\small_*.txt"; Dest=$destDir; Description="Small files"},
            @{Source="$sourceDir\medium_*.txt"; Dest=$destDir; Description="Medium files"},
            @{Source="$sourceDir\large_*.txt"; Dest=$destDir; Description="Large files"},
            @{Source="$sourceDir\xlarge_*.txt"; Dest=$destDir; Description="Extra large files"}
        )
        
        foreach ($scenario in $scenarios) {
            Write-Status "Profiling: $($scenario.Description)"
            $files = Get-ChildItem -Path $scenario.Source
            foreach ($file in $files) {
                $destFile = Join-Path $scenario.Dest $file.Name
                & $instrumentedBinary copy $file.FullName $destFile
            }
        }
        
        # Run directory copy operations
        Write-Status "Profiling directory operations..."
        $testSubDir = Join-Path $sourceDir "subdir"
        New-Item -ItemType Directory -Force -Path $testSubDir | Out-Null
        
        # Create some files in subdirectory
        for ($i = 1; $i -le 10; $i++) {
            $fileName = "subdir_file_$i.txt"
            $filePath = Join-Path $testSubDir $fileName
            Set-Content -Path $filePath -Value ("Content for file $i" * 100) -NoNewline
        }
        
        $destSubDir = Join-Path $destDir "subdir"
        & $instrumentedBinary copy $testSubDir $destSubDir --recursive
        
        Write-Success "Profile data collection completed"
    }
    catch {
        Write-Error "Failed during profile data collection: $_"
        exit 1
    }
    finally {
        # Clean up temporary files
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir
        }
    }
}

function Merge-ProfileData {
    Write-Status "Merging profile data..."
    
    $profileFiles = Get-ChildItem -Path $ProfileDir -Filter "*.profraw"
    if ($profileFiles.Count -eq 0) {
        Write-Error "No profile data files found in $ProfileDir"
        exit 1
    }
    
    Write-Status "Found $($profileFiles.Count) profile data files"
    
    $mergedProfile = Join-Path $ProfileDir "merged.profdata"
    $profilePaths = $profileFiles | ForEach-Object { $_.FullName }
    
    try {
        & llvm-profdata merge -o $mergedProfile @profilePaths
        Write-Success "Profile data merged successfully: $mergedProfile"
    }
    catch {
        Write-Error "Failed to merge profile data"
        exit 1
    }
}

function Build-OptimizedBinary {
    Write-Status "Building PGO-optimized binary..."
    
    $mergedProfile = Join-Path $ProfileDir "merged.profdata"
    if (-not (Test-Path $mergedProfile)) {
        Write-Error "Merged profile data not found: $mergedProfile"
        exit 1
    }
    
    $env:RUSTFLAGS = "-Cprofile-use=$mergedProfile"
    $env:CARGO_TARGET_DIR = "$OutputDir\optimized"
    
    try {
        if ($Verbose) {
            cargo build --profile release-pgo --bin ferrocp --verbose
        } else {
            cargo build --profile release-pgo --bin ferrocp
        }
        
        $optimizedBinary = "$OutputDir\optimized\release-pgo\ferrocp.exe"
        if (Test-Path $optimizedBinary) {
            # Copy to final location
            $finalBinary = Join-Path $OutputDir "ferrocp-pgo.exe"
            Copy-Item $optimizedBinary $finalBinary
            Write-Success "PGO-optimized binary created: $finalBinary"
        } else {
            Write-Error "Optimized binary not found after build"
            exit 1
        }
    }
    catch {
        Write-Error "Failed to build optimized binary"
        exit 1
    }
    finally {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    }
}

function Test-OptimizedBinary {
    Write-Status "Testing PGO-optimized binary..."
    
    $finalBinary = Join-Path $OutputDir "ferrocp-pgo.exe"
    if (-not (Test-Path $finalBinary)) {
        Write-Error "Final binary not found: $finalBinary"
        exit 1
    }
    
    try {
        # Test basic functionality
        $version = & $finalBinary --version
        Write-Status "Binary version: $version"
        
        # Test help command
        & $finalBinary --help | Out-Null
        
        Write-Success "PGO-optimized binary passed basic tests"
    }
    catch {
        Write-Error "PGO-optimized binary failed tests"
        exit 1
    }
}

# Main execution
Write-Status "Starting PGO build process for FerroCP..."

Test-Prerequisites
Initialize-Directories
Build-InstrumentedBinary
Collect-ProfileData
Merge-ProfileData
Build-OptimizedBinary
Test-OptimizedBinary

$finalBinary = Join-Path $OutputDir "ferrocp-pgo.exe"
$binarySize = (Get-Item $finalBinary).Length / 1MB

Write-Success "PGO build completed successfully!"
Write-Status "Final binary: $finalBinary"
Write-Status "Binary size: $([math]::Round($binarySize, 2)) MB"
Write-Status ""
Write-Status "Next steps:"
Write-Status "1. Run performance benchmarks to verify improvements"
Write-Status "2. Test the binary with real workloads"
Write-Status "3. Compare performance with regular release build"
