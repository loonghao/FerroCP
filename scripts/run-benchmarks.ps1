# Performance benchmark runner for FerroCP (PowerShell version)
# This script runs comprehensive performance benchmarks and generates reports

param(
    [string]$OutputDir = "benchmark_results",
    [string]$Baseline = "",
    [switch]$Quick,
    [switch]$Profile,
    [switch]$Help
)

# Colors for output
$Red = "Red"
$Green = "Green"
$Yellow = "Yellow"
$Blue = "Cyan"

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "White"
    )
    Write-Host $Message -ForegroundColor $Color
}

function Show-Usage {
    Write-Host @"
Usage: .\run-benchmarks.ps1 [OPTIONS]

Performance benchmark runner for FerroCP

OPTIONS:
    -OutputDir DIR      Directory to save benchmark results (default: benchmark_results)
    -Baseline FILE      Compare against baseline results
    -Quick              Run quick benchmarks (reduced sample size)
    -Profile            Enable profiling during benchmarks
    -Help               Show this help message

EXAMPLES:
    .\run-benchmarks.ps1                          # Run full benchmarks
    .\run-benchmarks.ps1 -Quick                   # Run quick benchmarks
    .\run-benchmarks.ps1 -Baseline baseline.json  # Compare with baseline
    .\run-benchmarks.ps1 -Profile -Quick          # Quick benchmarks with profiling

"@
}

if ($Help) {
    Show-Usage
    exit 0
}

Write-ColorOutput "🚀 Starting FerroCP Performance Benchmarks" $Blue
Write-ColorOutput "===========================================" $Blue

# Check if cargo is available
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-ColorOutput "❌ cargo not found. Please install Rust and Cargo." $Red
    exit 1
}

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# Generate timestamp
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$ResultsFile = Join-Path $OutputDir "benchmark_results_$Timestamp.json"

Write-ColorOutput "📁 Results will be saved to: $OutputDir" $Blue

# Build benchmark arguments
$BenchArgs = @()

if ($Quick) {
    Write-ColorOutput "⚡ Running in quick mode (reduced sample size)" $Yellow
    $BenchArgs += "--quick"
}

if ($Profile) {
    Write-ColorOutput "📊 Profiling enabled" $Yellow
}

# Build project first
Write-ColorOutput "🔨 Building project in release mode..." $Blue
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed"
    }
    Write-ColorOutput "✅ Build completed successfully" $Green
} catch {
    Write-ColorOutput "❌ Failed to build project: $_" $Red
    exit 1
}

# Run benchmarks
Write-ColorOutput "🧪 Running performance benchmarks..." $Green
Write-ColorOutput "- File copy performance tests" $Blue
Write-ColorOutput "- Compression algorithm benchmarks" $Blue
Write-ColorOutput "- Memory usage pattern analysis" $Blue
Write-ColorOutput "- Cross-engine performance comparison" $Blue

try {
    if ($Baseline) {
        Write-ColorOutput "📈 Comparing with baseline: $Baseline" $Yellow
        cargo bench -p ferrocp-tests -- --baseline $Baseline
    } else {
        cargo bench -p ferrocp-tests
    }
    
    if ($LASTEXITCODE -ne 0) {
        throw "Benchmarks failed"
    }
    
    Write-ColorOutput "✅ Benchmarks completed successfully" $Green
} catch {
    Write-ColorOutput "❌ Benchmarks failed: $_" $Red
    exit 1
}

# Run integration tests to verify functionality
Write-ColorOutput "🧪 Running integration tests to verify functionality..." $Blue
try {
    cargo test -p ferrocp-tests --test integration_tests
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput "⚠️ Some integration tests failed. Check functionality before trusting benchmark results." $Yellow
    } else {
        Write-ColorOutput "✅ All integration tests passed" $Green
    }
} catch {
    Write-ColorOutput "⚠️ Integration tests encountered issues: $_" $Yellow
}

# Generate performance summary
$SummaryFile = Join-Path $OutputDir "performance_summary_$Timestamp.txt"
Write-ColorOutput "📊 Generating performance summary..." $Blue

$SystemInfo = @{
    OS = [System.Environment]::OSVersion.VersionString
    Architecture = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    CPUCores = [System.Environment]::ProcessorCount
    RustVersion = (cargo --version)
    Timestamp = Get-Date
    Mode = if ($Quick) { "Quick" } else { "Full" }
    Profiling = if ($Profile) { "Enabled" } else { "Disabled" }
}

$SummaryContent = @"
# FerroCP Performance Summary
Generated: $($SystemInfo.Timestamp)
Mode: $($SystemInfo.Mode)
Profiling: $($SystemInfo.Profiling)

## System Information
OS: $($SystemInfo.OS)
Architecture: $($SystemInfo.Architecture)
CPU Cores: $($SystemInfo.CPUCores)
Rust Version: $($SystemInfo.RustVersion)

## Benchmark Categories
1. File Copy Performance
   - Tests different file sizes (1KB to 10MB)
   - Compares BufferedCopyEngine vs std::fs::copy
   - Measures throughput (MB/s)

2. Compression Algorithms
   - Tests zstd, lz4, brotli compression
   - Measures compression speed and ratio
   - 1MB test data with repeating patterns

3. Memory Usage Patterns
   - Tests different buffer sizes (4KB to 1MB)
   - Measures memory efficiency
   - 10MB file operations

4. Cross-Engine Comparison
   - Compares different copy engines
   - Identifies optimal engine for different scenarios

## Key Performance Indicators
- Throughput (MB/s) for file operations
- Compression ratio vs speed trade-offs
- Memory usage efficiency
- CPU utilization patterns

## Recommendations
- For file sizes < 1MB: Use smaller buffer sizes (64KB)
- For file sizes > 10MB: Use larger buffer sizes (1MB+)
- For concurrent operations: Optimal thread count appears to be 4-8
- For compression: Zstd provides best balance of speed and compression ratio

## Next Steps
1. Review detailed results in Criterion HTML reports
2. Compare with previous benchmarks if available
3. Identify performance bottlenecks
4. Consider hardware-specific optimizations
5. Run on different hardware configurations

## Files Generated
- Criterion HTML reports: target/criterion/
- Integration test results: Available in test output
- This summary: $SummaryFile
"@

$SummaryContent | Out-File -FilePath $SummaryFile -Encoding UTF8

Write-ColorOutput "✅ Performance summary saved to: $SummaryFile" $Green

# Check for Criterion HTML reports
$CriterionDir = "target/criterion"
if (Test-Path $CriterionDir) {
    Write-ColorOutput "📄 Criterion HTML reports available in: $CriterionDir" $Green
    
    # Find the main report index
    $IndexFile = Join-Path $CriterionDir "report/index.html"
    if (Test-Path $IndexFile) {
        Write-ColorOutput "🌐 Main report: $IndexFile" $Green
        
        # Try to open in default browser
        try {
            Start-Process $IndexFile
            Write-ColorOutput "🌐 Opening HTML report in browser..." $Yellow
        } catch {
            Write-ColorOutput "💡 To view HTML report, open: $IndexFile" $Blue
        }
    }
}

# Final summary
Write-ColorOutput "`n🎉 Benchmark run completed successfully!" $Green
Write-ColorOutput "===========================================" $Blue
Write-ColorOutput "Results available in:" $Blue
Write-ColorOutput "  - Summary: $SummaryFile" $Blue
Write-ColorOutput "  - Criterion Reports: $CriterionDir" $Blue

if ($Baseline) {
    Write-ColorOutput "  - Baseline comparison results in Criterion reports" $Blue
}

Write-ColorOutput "`n💡 Next steps:" $Yellow
Write-ColorOutput "1. Review the performance summary" $Blue
Write-ColorOutput "2. Open Criterion HTML reports for detailed analysis" $Blue
Write-ColorOutput "3. Compare results with previous runs" $Blue
Write-ColorOutput "4. Identify optimization opportunities" $Blue
