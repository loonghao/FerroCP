# Format Check Script for FerroCP (Windows)
# This script checks and optionally fixes code formatting

param(
    [switch]$Fix,
    [switch]$Check,
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

# Check if we're in a git repository
try {
    $null = git rev-parse --git-dir 2>$null
} catch {
    Write-Error "Not in a git repository"
    exit 1
}

# Get project root
$ProjectRoot = git rev-parse --show-toplevel
Set-Location $ProjectRoot

Write-Info "FerroCP Code Formatting Tool"
Write-Info "============================="

# Check if Rust is installed
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Error "Rust is not installed. Please install from https://rustup.rs/"
    exit 1
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
}

# Check if rustfmt is available
try {
    $null = rustup component list --installed | Select-String "rustfmt"
} catch {
    Write-Info "Installing rustfmt component..."
    rustup component add rustfmt
}

# Determine action based on parameters
$Action = "check"  # default action

if ($Fix) {
    $Action = "fix"
} elseif ($Check) {
    $Action = "check"
} else {
    # Interactive mode
    Write-Info "What would you like to do?"
    Write-Host "1. Check formatting (default)"
    Write-Host "2. Fix formatting"
    Write-Host ""
    $choice = Read-Host "Enter your choice (1-2)"
    
    switch ($choice) {
        "2" { $Action = "fix" }
        default { $Action = "check" }
    }
}

Write-Info "Action: $Action"
Write-Host ""

if ($Action -eq "fix") {
    Write-Info "Fixing code formatting..."
    
    try {
        cargo fmt --all
        
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Code formatting fixed successfully!"
            
            # Check if there are any changes
            $changes = git diff --name-only
            if ($changes) {
                Write-Info "Files that were formatted:"
                $changes | ForEach-Object { Write-Host "  $_" }
                Write-Warning "Don't forget to commit these formatting changes!"
            } else {
                Write-Info "No formatting changes were needed."
            }
        } else {
            Write-Error "Failed to format code (Exit code: $LASTEXITCODE)"
            exit 1
        }
    }
    catch {
        Write-Error "Error running cargo fmt: $($_.Exception.Message)"
        exit 1
    }
    
} elseif ($Action -eq "check") {
    Write-Info "Checking code formatting..."
    
    try {
        if ($Verbose) {
            cargo fmt --all -- --check --verbose
        } else {
            cargo fmt --all -- --check
        }
        
        if ($LASTEXITCODE -eq 0) {
            Write-Success "All code is properly formatted!"
        } else {
            Write-Error "Code formatting issues found!"
            Write-Info "Run the following command to fix formatting:"
            Write-Host "  cargo fmt --all" -ForegroundColor Cyan
            Write-Info "Or run this script with -Fix parameter:"
            Write-Host "  .\scripts\format-check.ps1 -Fix" -ForegroundColor Cyan
            exit 1
        }
    }
    catch {
        Write-Error "Error checking formatting: $($_.Exception.Message)"
        exit 1
    }
}

# Additional checks
Write-Info "Running additional checks..."

# Check for common issues in staged files (if in git repo)
try {
    $stagedFiles = git diff --cached --name-only --diff-filter=ACM | Where-Object { $_ -match '\.(rs|toml|md)$' }
    
    if ($stagedFiles) {
        Write-Info "Checking staged files for common issues..."
        
        # Check for TODO/FIXME comments
        $todoFiles = @()
        foreach ($file in $stagedFiles) {
            if (Test-Path $file) {
                $content = Get-Content $file -Raw
                if ($content -match "TODO|FIXME") {
                    $todoFiles += $file
                }
            }
        }
        
        if ($todoFiles.Count -gt 0) {
            Write-Warning "Found TODO/FIXME comments in staged files:"
            $todoFiles | ForEach-Object { Write-Host "  $_" }
        }
        
        # Check for debug prints in Rust files
        $debugFiles = @()
        $rustFiles = $stagedFiles | Where-Object { $_ -match '\.rs$' }
        foreach ($file in $rustFiles) {
            if (Test-Path $file) {
                $content = Get-Content $file -Raw
                if ($content -match "println!|dbg!|eprintln!") {
                    $debugFiles += $file
                }
            }
        }
        
        if ($debugFiles.Count -gt 0) {
            Write-Warning "Found debug prints in staged Rust files:"
            $debugFiles | ForEach-Object { Write-Host "  $_" }
        }
    }
} catch {
    # Not in a git repo or no staged files, skip this check
}

Write-Success "Format check completed!"

exit 0
