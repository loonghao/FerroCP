# Windows CI Test Script - Simulates CI environment on Windows

param(
    [switch]$SkipFormatCheck,
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [switch]$Verbose
)

# Colors for output
$Red = "Red"
$Green = "Green"
$Yellow = "Yellow"
$Blue = "Blue"

# Logging functions
function Write-ColorOutput($Message, $Color) {
    Write-Host $Message -ForegroundColor $Color
}

function Write-Info($Message) {
    Write-ColorOutput "[INFO] $Message" $Blue
}

function Write-Success($Message) {
    Write-ColorOutput "[SUCCESS] $Message" $Green
}

function Write-Warning($Message) {
    Write-ColorOutput "[WARNING] $Message" $Yellow
}

function Write-Error($Message) {
    Write-ColorOutput "[ERROR] $Message" $Red
}

# Test command execution
function Test-Command($Description, $Command) {
    Write-Info "Testing: $Description"
    
    try {
        if ($Verbose) {
            Write-Info "Running: $Command"
        }
        
        Invoke-Expression $Command
        
        if ($LASTEXITCODE -eq 0) {
            Write-Success "$Description - PASSED"
            return $true
        } else {
            Write-Error "$Description - FAILED (Exit code: $LASTEXITCODE)"
            return $false
        }
    }
    catch {
        Write-Error "$Description - FAILED (Exception: $($_.Exception.Message))"
        return $false
    }
}

# Main test execution
function Main {
    Write-Info "Starting Windows CI Test Simulation"
    Write-Info "===================================="
    
    # Set environment variables like CI
    $env:CARGO_TERM_COLOR = "always"
    $env:RUST_BACKTRACE = "1"
    $env:BLAKE3_NO_ASM = "1"
    $env:CARGO_NET_GIT_FETCH_WITH_CLI = "true"
    $env:RUST_LOG = "debug"
    
    Write-Info "Environment variables set:"
    Write-Host "  CARGO_TERM_COLOR=$env:CARGO_TERM_COLOR"
    Write-Host "  RUST_BACKTRACE=$env:RUST_BACKTRACE"
    Write-Host "  BLAKE3_NO_ASM=$env:BLAKE3_NO_ASM"
    Write-Host "  CARGO_NET_GIT_FETCH_WITH_CLI=$env:CARGO_NET_GIT_FETCH_WITH_CLI"
    Write-Host "  RUST_LOG=$env:RUST_LOG"
    Write-Host ""
    
    # Check system dependencies
    Write-Info "Checking system dependencies..."
    
    if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
        Write-Error "Rust is not installed"
        exit 1
    }
    
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "Cargo is not installed"
        exit 1
    }
    
    # Display versions
    Write-Info "Tool versions:"
    rustc --version
    cargo --version
    Write-Host ""
    
    # Run the same tests as CI
    $failedTests = 0
    
    # 1. Check formatting
    if (-not $SkipFormatCheck) {
        if (-not (Test-Command "Code formatting check" "cargo fmt --all -- --check")) {
            $failedTests++
        }
    } else {
        Write-Warning "Skipping format check"
    }
    
    # 2. Run workspace tests (excluding Python extension)
    if (-not $SkipTests) {
        $testCommand = "cargo test --workspace --exclude ferrocp-python"
        if ($Verbose) {
            $testCommand += " --verbose"
        }
        
        if (-not (Test-Command "Rust workspace tests" $testCommand)) {
            $failedTests++
        }
    } else {
        Write-Warning "Skipping tests"
    }
    
    # 3. Build Python extension
    if (-not $SkipBuild) {
        if (-not (Test-Command "Python extension build" "cargo build -p ferrocp-python")) {
            $failedTests++
        }
    } else {
        Write-Warning "Skipping build"
    }
    
    # 4. Additional checks
    Write-Info "Running additional checks..."
    
    # Check for clippy (optional)
    if (Get-Command cargo-clippy -ErrorAction SilentlyContinue) {
        Test-Command "Clippy check (optional)" "cargo clippy --workspace --all-targets --all-features -- -D warnings" | Out-Null
    } else {
        Write-Warning "Clippy not available"
    }
    
    # Summary
    Write-Host ""
    Write-Info "Test Summary"
    Write-Info "============"
    
    if ($failedTests -eq 0) {
        Write-Success "All tests passed! ✅"
        Write-Success "Windows CI environment is working correctly."
        exit 0
    } else {
        Write-Error "$failedTests test(s) failed! ❌"
        Write-Error "CI environment needs fixes."
        exit 1
    }
}

# Run main function
Main
