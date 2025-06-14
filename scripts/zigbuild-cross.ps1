#!/usr/bin/env pwsh
# FerroCP Cross-Compilation Script using cargo-zigbuild
# Optimized cross-compilation for VFX Platform targets

param(
    [Parameter(Mandatory=$false)]
    [string]$Target = "all",
    
    [Parameter(Mandatory=$false)]
    [switch]$Release = $false,
    
    [Parameter(Mandatory=$false)]
    [switch]$Verbose = $false,
    
    [Parameter(Mandatory=$false)]
    [switch]$Clean = $false
)

# VFX Platform supported targets
$VfxTargets = @(
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu", 
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc"
)

# Color output functions
function Write-ColorOutput {
    param([string]$Message, [string]$Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Write-Success { param([string]$Message) Write-ColorOutput "✅ $Message" "Green" }
function Write-Info { param([string]$Message) Write-ColorOutput "ℹ️  $Message" "Cyan" }
function Write-Warning { param([string]$Message) Write-ColorOutput "⚠️  $Message" "Yellow" }
function Write-Error { param([string]$Message) Write-ColorOutput "❌ $Message" "Red" }

# Check prerequisites
function Test-Prerequisites {
    Write-Info "Checking prerequisites..."
    
    # Check cargo-zigbuild
    if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
        Write-Error "Cargo not found. Please install Rust toolchain."
        exit 1
    }
    
    # Check if cargo-zigbuild is installed
    $zigbuildInstalled = cargo install --list | Select-String "cargo-zigbuild"
    if (-not $zigbuildInstalled) {
        Write-Info "Installing cargo-zigbuild..."
        cargo install --locked cargo-zigbuild
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install cargo-zigbuild"
            exit 1
        }
    }
    
    # Check zig installation
    if (-not (Get-Command "zig" -ErrorAction SilentlyContinue)) {
        Write-Info "Installing zig via pip..."
        pip install ziglang
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install zig. Please install manually."
            exit 1
        }
    }
    
    Write-Success "Prerequisites check completed"
}

# Install Rust targets
function Install-RustTargets {
    param([string[]]$Targets)
    
    Write-Info "Installing Rust targets..."
    foreach ($target in $Targets) {
        Write-Info "Adding target: $target"
        rustup target add $target
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "Failed to add target: $target"
        }
    }
}

# Build for specific target
function Build-Target {
    param(
        [string]$TargetTriple,
        [bool]$IsRelease,
        [bool]$IsVerbose
    )
    
    $buildArgs = @("zigbuild", "--bin", "ferrocp", "--target", $TargetTriple)
    
    if ($IsRelease) {
        $buildArgs += "--release"
    }
    
    if ($IsVerbose) {
        $buildArgs += "--verbose"
    }
    
    Write-Info "Building for target: $TargetTriple"
    Write-Info "Command: cargo $($buildArgs -join ' ')"
    
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    
    & cargo @buildArgs
    $exitCode = $LASTEXITCODE
    
    $stopwatch.Stop()
    $elapsed = $stopwatch.Elapsed.ToString("mm\:ss\.ff")
    
    if ($exitCode -eq 0) {
        Write-Success "Build completed for $TargetTriple in $elapsed"
        
        # Show binary info
        $profileDir = if ($IsRelease) { "release" } else { "debug" }
        $binaryPath = "target/$TargetTriple/$profileDir/ferrocp"
        if ($TargetTriple -like "*windows*") {
            $binaryPath += ".exe"
        }
        
        if (Test-Path $binaryPath) {
            $fileInfo = Get-Item $binaryPath
            $sizeKB = [math]::Round($fileInfo.Length / 1KB, 2)
            Write-Info "Binary size: $sizeKB KB"
        }
    } else {
        Write-Error "Build failed for $TargetTriple after $elapsed"
    }
    
    return $exitCode
}

# Main execution
function Main {
    Write-ColorOutput "🚀 FerroCP Cross-Compilation with cargo-zigbuild" "Magenta"
    Write-Info "Target: $Target | Release: $Release | Verbose: $Verbose"
    
    # Clean if requested
    if ($Clean) {
        Write-Info "Cleaning previous builds..."
        cargo clean
    }
    
    # Check prerequisites
    Test-Prerequisites
    
    # Determine targets to build
    $targetsToBuild = if ($Target -eq "all") {
        $VfxTargets
    } elseif ($Target -in $VfxTargets) {
        @($Target)
    } else {
        Write-Error "Invalid target: $Target. Valid targets: $($VfxTargets -join ', '), all"
        exit 1
    }
    
    # Install required targets
    Install-RustTargets -Targets $targetsToBuild
    
    # Build targets
    $failedBuilds = @()
    $successfulBuilds = @()
    
    foreach ($target in $targetsToBuild) {
        $result = Build-Target -TargetTriple $target -IsRelease $Release -IsVerbose $Verbose
        if ($result -eq 0) {
            $successfulBuilds += $target
        } else {
            $failedBuilds += $target
        }
    }
    
    # Summary
    Write-ColorOutput "`n📊 Build Summary:" "Magenta"
    Write-Success "Successful builds ($($successfulBuilds.Count)): $($successfulBuilds -join ', ')"
    
    if ($failedBuilds.Count -gt 0) {
        Write-Error "Failed builds ($($failedBuilds.Count)): $($failedBuilds -join ', ')"
        exit 1
    } else {
        Write-Success "All builds completed successfully! 🎉"
    }
}

# Run main function
Main
