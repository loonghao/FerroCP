# Simple test runner for ferrocp project
# This script runs the new test suites we've implemented

param(
    [switch]$PropertyTests,
    [switch]$FuzzTests,
    [switch]$ErrorTests,
    [switch]$All,
    [switch]$Verbose
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

function Run-Test {
    param(
        [string]$TestName,
        [string]$Package,
        [string]$TestFilter,
        [string]$Description
    )
    
    Write-ColorOutput "`n🧪 Running $Description..." $Blue
    Write-ColorOutput "Package: $Package, Filter: $TestFilter" $Yellow
    
    $args = @("test", "--package", $Package)
    if ($TestFilter) {
        $args += $TestFilter
    }
    if ($Verbose) {
        $args += "--verbose"
    }
    
    $startTime = Get-Date
    & cargo @args
    $endTime = Get-Date
    $duration = $endTime - $startTime
    
    if ($LASTEXITCODE -eq 0) {
        Write-ColorOutput "✅ $Description completed successfully in $($duration.TotalSeconds.ToString('F2'))s" $Green
        return $true
    } else {
        Write-ColorOutput "❌ $Description failed" $Red
        return $false
    }
}

# Main execution
Write-ColorOutput "🚀 ferrocp Test Suite Runner" $Blue
Write-ColorOutput "=============================" $Blue

$testResults = @()

# Run property tests
if ($PropertyTests -or $All) {
    $result = Run-Test -TestName "PropertyTests" -Package "ferrocp-io" -TestFilter "property_tests" -Description "Property Tests (ferrocp-io)"
    $testResults += @{ Name = "Property Tests"; Success = $result }
}

# Run fuzz tests
if ($FuzzTests -or $All) {
    $result = Run-Test -TestName "FuzzTests" -Package "ferrocp-compression" -TestFilter "fuzz_tests" -Description "Fuzz Tests (ferrocp-compression)"
    $testResults += @{ Name = "Fuzz Tests"; Success = $result }
}

# Run error handling tests
if ($ErrorTests -or $All) {
    $result1 = Run-Test -TestName "ErrorTestsIO" -Package "ferrocp-io" -TestFilter "error_tests" -Description "Error Handling Tests (ferrocp-io)"
    $result2 = Run-Test -TestName "ErrorTestsCompression" -Package "ferrocp-compression" -TestFilter "error_tests" -Description "Error Handling Tests (ferrocp-compression)"
    $testResults += @{ Name = "Error Tests (IO)"; Success = $result1 }
    $testResults += @{ Name = "Error Tests (Compression)"; Success = $result2 }
}

# If no specific tests requested, run all
if (-not ($PropertyTests -or $FuzzTests -or $ErrorTests)) {
    Write-ColorOutput "No specific test category selected. Running all new test suites..." $Yellow
    
    $result1 = Run-Test -TestName "PropertyTests" -Package "ferrocp-io" -TestFilter "property_tests" -Description "Property Tests (ferrocp-io)"
    $result2 = Run-Test -TestName "FuzzTests" -Package "ferrocp-compression" -TestFilter "fuzz_tests" -Description "Fuzz Tests (ferrocp-compression)"
    $result3 = Run-Test -TestName "ErrorTestsIO" -Package "ferrocp-io" -TestFilter "error_tests" -Description "Error Handling Tests (ferrocp-io)"
    $result4 = Run-Test -TestName "ErrorTestsCompression" -Package "ferrocp-compression" -TestFilter "error_tests" -Description "Error Handling Tests (ferrocp-compression)"
    
    $testResults += @{ Name = "Property Tests"; Success = $result1 }
    $testResults += @{ Name = "Fuzz Tests"; Success = $result2 }
    $testResults += @{ Name = "Error Tests (IO)"; Success = $result3 }
    $testResults += @{ Name = "Error Tests (Compression)"; Success = $result4 }
}

# Summary
Write-ColorOutput "`n📊 Test Results Summary" $Blue
Write-ColorOutput "======================" $Blue

$successCount = 0
$totalCount = $testResults.Count

foreach ($result in $testResults) {
    if ($result.Success) {
        Write-ColorOutput "✅ $($result.Name)" $Green
        $successCount++
    } else {
        Write-ColorOutput "❌ $($result.Name)" $Red
    }
}

Write-ColorOutput "`n📈 Overall Results:" $Blue
Write-ColorOutput "Passed: $successCount/$totalCount" $(if ($successCount -eq $totalCount) { $Green } else { $Yellow })

if ($successCount -eq $totalCount) {
    Write-ColorOutput "`n🎉 All tests passed successfully!" $Green
    exit 0
} else {
    Write-ColorOutput "`n⚠️  Some tests failed. Please check the output above." $Red
    exit 1
}
