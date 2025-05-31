# Test Coverage Script for ferrocp
# This script runs comprehensive tests and generates coverage reports

param(
    [string]$OutputDir = "target/coverage",
    [switch]$Html,
    [switch]$Xml,
    [switch]$Json,
    [switch]$PropertyTests,
    [switch]$FuzzTests,
    [switch]$ErrorTests,
    [switch]$All
)

# Colors for output
$Green = "`e[32m"
$Red = "`e[31m"
$Yellow = "`e[33m"
$Blue = "`e[34m"
$Reset = "`e[0m"

function Write-ColorOutput {
    param([string]$Message, [string]$Color = $Reset)
    Write-Host "$Color$Message$Reset"
}

function Test-Command {
    param([string]$Command)
    try {
        Get-Command $Command -ErrorAction Stop | Out-Null
        return $true
    }
    catch {
        return $false
    }
}

# Check if cargo-tarpaulin is installed
if (-not (Test-Command "cargo-tarpaulin")) {
    Write-ColorOutput "Installing cargo-tarpaulin..." $Yellow
    cargo install cargo-tarpaulin
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "Failed to install cargo-tarpaulin" $Red
        exit 1
    }
}

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

Write-ColorOutput "🧪 Running ferrocp test coverage analysis..." $Blue

# Base tarpaulin command
$TarpaulinArgs = @(
    "tarpaulin",
    "--workspace",
    "--timeout", "120",
    "--out", "Html",
    "--output-dir", $OutputDir
)

# Add output formats
if ($Html -or $All) {
    $TarpaulinArgs += "--out", "Html"
}
if ($Xml -or $All) {
    $TarpaulinArgs += "--out", "Xml"
}
if ($Json -or $All) {
    $TarpaulinArgs += "--out", "Json"
}

# Default to HTML if no format specified
if (-not ($Html -or $Xml -or $Json -or $All)) {
    $TarpaulinArgs += "--out", "Html"
}

# Test categories
$TestCategories = @()

if ($PropertyTests -or $All) {
    $TestCategories += "property_tests"
}
if ($FuzzTests -or $All) {
    $TestCategories += "fuzz_tests"
}
if ($ErrorTests -or $All) {
    $TestCategories += "error_tests"
}

# If no specific tests requested, run all tests
if ($TestCategories.Count -eq 0 -or $All) {
    Write-ColorOutput "Running all tests with coverage..." $Green
    
    # Run comprehensive test coverage
    & cargo @TarpaulinArgs --all-features --exclude-files "*/tests/*" --exclude-files "*/benches/*"
    
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "Coverage analysis failed" $Red
        exit 1
    }
} else {
    # Run specific test categories
    foreach ($category in $TestCategories) {
        Write-ColorOutput "Running $category with coverage..." $Green
        
        $CategoryArgs = $TarpaulinArgs + @("--test", $category)
        & cargo @CategoryArgs
        
        if ($LASTEXITCODE -ne 0) {
            Write-ColorOutput "Coverage analysis failed for $category" $Red
            exit 1
        }
    }
}

# Run property tests separately if requested
if ($PropertyTests -or $All) {
    Write-ColorOutput "Running property tests for ferrocp-io..." $Green
    cargo test --package ferrocp-io property_tests --release
    
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "Property tests failed" $Red
        exit 1
    }
}

# Run fuzz tests separately if requested  
if ($FuzzTests -or $All) {
    Write-ColorOutput "Running fuzz tests for ferrocp-compression..." $Green
    cargo test --package ferrocp-compression fuzz_tests --release
    
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "Fuzz tests failed" $Red
        exit 1
    }
}

# Run error handling tests separately if requested
if ($ErrorTests -or $All) {
    Write-ColorOutput "Running error handling tests..." $Green
    cargo test --package ferrocp-io error_tests --release
    cargo test --package ferrocp-compression error_tests --release
    
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "Error handling tests failed" $Red
        exit 1
    }
}

# Generate summary report
Write-ColorOutput "`n📊 Test Coverage Summary" $Blue
Write-ColorOutput "=========================" $Blue

if (Test-Path "$OutputDir/tarpaulin-report.html") {
    Write-ColorOutput "✅ HTML coverage report: $OutputDir/tarpaulin-report.html" $Green
}

if (Test-Path "$OutputDir/cobertura.xml") {
    Write-ColorOutput "✅ XML coverage report: $OutputDir/cobertura.xml" $Green
}

if (Test-Path "$OutputDir/tarpaulin-report.json") {
    Write-ColorOutput "✅ JSON coverage report: $OutputDir/tarpaulin-report.json" $Green
}

# Parse coverage percentage from HTML report if available
if (Test-Path "$OutputDir/tarpaulin-report.html") {
    try {
        $htmlContent = Get-Content "$OutputDir/tarpaulin-report.html" -Raw
        if ($htmlContent -match 'Coverage: (\d+\.?\d*)%') {
            $coveragePercent = $matches[1]
            Write-ColorOutput "📈 Overall Coverage: $coveragePercent%" $Green
            
            # Coverage quality assessment
            $coverage = [double]$coveragePercent
            if ($coverage -ge 90) {
                Write-ColorOutput "🎉 Excellent coverage!" $Green
            } elseif ($coverage -ge 80) {
                Write-ColorOutput "✅ Good coverage" $Green
            } elseif ($coverage -ge 70) {
                Write-ColorOutput "⚠️  Acceptable coverage" $Yellow
            } else {
                Write-ColorOutput "❌ Coverage needs improvement" $Red
            }
        }
    }
    catch {
        Write-ColorOutput "Could not parse coverage percentage" $Yellow
    }
}

Write-ColorOutput "`n🎯 Test Categories Completed:" $Blue
if ($PropertyTests -or $All) {
    Write-ColorOutput "  ✅ Property Tests (ferrocp-io)" $Green
}
if ($FuzzTests -or $All) {
    Write-ColorOutput "  ✅ Fuzz Tests (ferrocp-compression)" $Green
}
if ($ErrorTests -or $All) {
    Write-ColorOutput "  ✅ Error Handling Tests" $Green
}

Write-ColorOutput "`n🏁 Coverage analysis completed successfully!" $Green

# Open HTML report if available and requested
if ($Html -and (Test-Path "$OutputDir/tarpaulin-report.html")) {
    Write-ColorOutput "Opening coverage report in browser..." $Blue
    Start-Process "$OutputDir/tarpaulin-report.html"
}
