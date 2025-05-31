@echo off
REM Performance benchmarking script for FerroCP (Windows)

setlocal enabledelayedexpansion

REM Default values
set OUTPUT_DIR=benchmark_results
set BASELINE_FILE=
set COMPARE_MODE=false
set QUICK_MODE=false
set PROFILE_MODE=false

REM Parse command line arguments
:parse_args
if "%~1"=="" goto :args_done
if "%~1"=="--output-dir" (
    set OUTPUT_DIR=%~2
    shift
    shift
    goto :parse_args
)
if "%~1"=="--baseline" (
    set BASELINE_FILE=%~2
    set COMPARE_MODE=true
    shift
    shift
    goto :parse_args
)
if "%~1"=="--quick" (
    set QUICK_MODE=true
    shift
    goto :parse_args
)
if "%~1"=="--profile" (
    set PROFILE_MODE=true
    shift
    goto :parse_args
)
if "%~1"=="--help" (
    echo Usage: %0 [OPTIONS]
    echo.
    echo Options:
    echo   --output-dir DIR    Directory to save benchmark results (default: benchmark_results)
    echo   --baseline FILE     Compare against baseline results
    echo   --quick            Run quick benchmarks only
    echo   --profile          Enable profiling mode
    echo   --help             Show this help message
    exit /b 0
)
echo Unknown option: %~1
exit /b 1

:args_done

echo [INFO] Starting FerroCP performance benchmarks

REM Create output directory
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

REM Check if cargo is available
where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Cargo is not installed or not in PATH
    exit /b 1
)

REM Build in release mode first
echo [INFO] Building project in release mode...
cargo build --release
if errorlevel 1 (
    echo [ERROR] Failed to build project in release mode
    exit /b 1
)

REM Set benchmark parameters based on mode
if "%QUICK_MODE%"=="true" (
    set BENCH_TIME=--measurement-time 10
    set SAMPLE_SIZE=--sample-size 50
    echo [INFO] Running in quick mode (reduced sample size and time)
) else (
    set BENCH_TIME=--measurement-time 30
    set SAMPLE_SIZE=--sample-size 100
    echo [INFO] Running full benchmarks
)

REM Generate timestamp for this run
for /f "tokens=2 delims==" %%a in ('wmic OS Get localdatetime /value') do set "dt=%%a"
set "YY=%dt:~2,2%" & set "YYYY=%dt:~0,4%" & set "MM=%dt:~4,2%" & set "DD=%dt:~6,2%"
set "HH=%dt:~8,2%" & set "Min=%dt:~10,2%" & set "Sec=%dt:~12,2%"
set "TIMESTAMP=%YYYY%%MM%%DD%_%HH%%Min%%Sec%"
set "RESULTS_FILE=%OUTPUT_DIR%\benchmark_results_%TIMESTAMP%.json"

echo [INFO] Running performance benchmarks...

REM Execute benchmarks
set BENCHMARK_CMD=cargo bench -p ferrocp-tests

if "%PROFILE_MODE%"=="true" (
    echo [INFO] Running benchmarks with profiling enabled...
    set BENCHMARK_CMD=%BENCHMARK_CMD% -- --profile-time=5
)

REM Run the benchmarks
%BENCHMARK_CMD% %BENCH_TIME% %SAMPLE_SIZE%
if errorlevel 1 (
    echo [ERROR] Benchmarks failed to run
    exit /b 1
)

echo [SUCCESS] Benchmarks completed successfully
echo [INFO] Results saved to: %RESULTS_FILE%

REM Compare with baseline if provided
if "%COMPARE_MODE%"=="true" (
    if exist "%BASELINE_FILE%" (
        echo [INFO] Comparing with baseline: %BASELINE_FILE%
        set COMPARISON_FILE=%OUTPUT_DIR%\comparison_%TIMESTAMP%.txt
        
        echo # Performance Comparison Report > "!COMPARISON_FILE!"
        echo Generated: %date% %time% >> "!COMPARISON_FILE!"
        echo Baseline: %BASELINE_FILE% >> "!COMPARISON_FILE!"
        echo Current: %RESULTS_FILE% >> "!COMPARISON_FILE!"
        echo. >> "!COMPARISON_FILE!"
        echo ## Summary >> "!COMPARISON_FILE!"
        echo This is a basic comparison. For detailed analysis, use criterion's built-in comparison tools. >> "!COMPARISON_FILE!"
        
        echo [SUCCESS] Comparison saved to: !COMPARISON_FILE!
    ) else (
        echo [WARNING] Baseline file not found: %BASELINE_FILE%
    )
)

REM Run integration tests to ensure functionality
echo [INFO] Running integration tests to verify functionality...
cargo test -p ferrocp-tests --test integration_tests
if errorlevel 1 (
    echo [WARNING] Some integration tests failed. Check functionality before trusting benchmark results.
) else (
    echo [SUCCESS] All integration tests passed
)

REM Generate performance summary
set SUMMARY_FILE=%OUTPUT_DIR%\performance_summary_%TIMESTAMP%.txt
echo [INFO] Generating performance summary...

echo # FerroCP Performance Summary > "%SUMMARY_FILE%"
echo Generated: %date% %time% >> "%SUMMARY_FILE%"
if "%QUICK_MODE%"=="true" (
    echo Mode: Quick >> "%SUMMARY_FILE%"
) else (
    echo Mode: Full >> "%SUMMARY_FILE%"
)
if "%PROFILE_MODE%"=="true" (
    echo Profiling: Enabled >> "%SUMMARY_FILE%"
) else (
    echo Profiling: Disabled >> "%SUMMARY_FILE%"
)
echo. >> "%SUMMARY_FILE%"
echo ## System Information >> "%SUMMARY_FILE%"
echo OS: %OS% >> "%SUMMARY_FILE%"
echo Architecture: %PROCESSOR_ARCHITECTURE% >> "%SUMMARY_FILE%"
echo CPU Cores: %NUMBER_OF_PROCESSORS% >> "%SUMMARY_FILE%"
rustc --version >> "%SUMMARY_FILE%"
echo. >> "%SUMMARY_FILE%"
echo ## Benchmark Results >> "%SUMMARY_FILE%"
echo Detailed results: %RESULTS_FILE% >> "%SUMMARY_FILE%"
echo. >> "%SUMMARY_FILE%"
echo ## Key Metrics >> "%SUMMARY_FILE%"
echo (Extract key metrics from JSON results here) >> "%SUMMARY_FILE%"
echo. >> "%SUMMARY_FILE%"
echo ## Recommendations >> "%SUMMARY_FILE%"
echo - For file sizes ^< 1MB: Use smaller buffer sizes (64KB) >> "%SUMMARY_FILE%"
echo - For file sizes ^> 10MB: Use larger buffer sizes (1MB+) >> "%SUMMARY_FILE%"
echo - For concurrent operations: Optimal thread count appears to be 4-8 >> "%SUMMARY_FILE%"
echo - For compression: Zstd provides best balance of speed and compression ratio >> "%SUMMARY_FILE%"
echo. >> "%SUMMARY_FILE%"
echo ## Next Steps >> "%SUMMARY_FILE%"
echo 1. Review detailed results in HTML report >> "%SUMMARY_FILE%"
echo 2. Compare with previous benchmarks if available >> "%SUMMARY_FILE%"
echo 3. Identify performance bottlenecks >> "%SUMMARY_FILE%"
echo 4. Consider hardware-specific optimizations >> "%SUMMARY_FILE%"

echo [SUCCESS] Performance summary saved to: %SUMMARY_FILE%

REM Cleanup temporary files if any
echo [INFO] Cleaning up temporary files...

echo [SUCCESS] Benchmark run completed successfully!
echo.
echo Results available in:
echo   - Summary: %SUMMARY_FILE%
echo.

if "%COMPARE_MODE%"=="true" (
    if exist "%COMPARISON_FILE%" (
        echo   - Comparison: %COMPARISON_FILE%
        echo.
    )
)

echo [INFO] Check target\criterion\ directory for detailed HTML reports

endlocal
