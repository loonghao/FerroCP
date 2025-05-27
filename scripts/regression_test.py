#!/usr/bin/env python3
"""Performance regression testing script."""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List

import subprocess


class BenchmarkResult:
    """Represents a benchmark result."""
    
    def __init__(self, name: str, mean: float, stddev: float, min_val: float, max_val: float):
        self.name = name
        self.mean = mean
        self.stddev = stddev
        self.min = min_val
        self.max = max_val
    
    @classmethod
    def from_dict(cls, data: dict) -> 'BenchmarkResult':
        """Create from benchmark JSON data."""
        stats = data['stats']
        return cls(
            name=data['name'],
            mean=stats['mean'],
            stddev=stats['stddev'],
            min_val=stats['min'],
            max_val=stats['max']
        )


class RegressionAnalyzer:
    """Analyze performance regressions between benchmark runs."""
    
    def __init__(self, threshold: float = 0.1):
        """Initialize with regression threshold (10% by default)."""
        self.threshold = threshold
    
    def load_results(self, file_path: Path) -> List[BenchmarkResult]:
        """Load benchmark results from JSON file."""
        with open(file_path) as f:
            data = json.load(f)
        
        results = []
        for benchmark in data.get('benchmarks', []):
            results.append(BenchmarkResult.from_dict(benchmark))
        
        return results
    
    def compare_results(
        self, 
        baseline: List[BenchmarkResult], 
        current: List[BenchmarkResult]
    ) -> Dict[str, Dict]:
        """Compare current results against baseline."""
        baseline_dict = {r.name: r for r in baseline}
        current_dict = {r.name: r for r in current}
        
        comparisons = {}
        
        # Find common benchmarks
        common_names = set(baseline_dict.keys()) & set(current_dict.keys())
        
        for name in common_names:
            base = baseline_dict[name]
            curr = current_dict[name]
            
            # Calculate percentage change
            change = (curr.mean - base.mean) / base.mean
            
            # Determine if this is a regression
            is_regression = change > self.threshold
            is_improvement = change < -self.threshold
            
            # Calculate statistical significance (simple approach)
            # A more sophisticated approach would use t-tests
            combined_stddev = (base.stddev + curr.stddev) / 2
            significance = abs(change) > (2 * combined_stddev / base.mean)
            
            comparisons[name] = {
                'baseline_mean': base.mean,
                'current_mean': curr.mean,
                'change_percent': change * 100,
                'is_regression': is_regression,
                'is_improvement': is_improvement,
                'is_significant': significance,
                'baseline_stddev': base.stddev,
                'current_stddev': curr.stddev,
            }
        
        return comparisons
    
    def generate_report(self, comparisons: Dict[str, Dict]) -> str:
        """Generate a human-readable report."""
        report = []
        report.append("# Performance Regression Analysis")
        report.append("")
        
        # Summary
        total = len(comparisons)
        regressions = sum(1 for c in comparisons.values() if c['is_regression'])
        improvements = sum(1 for c in comparisons.values() if c['is_improvement'])
        
        report.append("## Summary")
        report.append(f"- Total benchmarks: {total}")
        report.append(f"- Regressions: {regressions}")
        report.append(f"- Improvements: {improvements}")
        report.append(f"- Threshold: {self.threshold * 100:.1f}%")
        report.append("")
        
        # Regressions
        if regressions > 0:
            report.append("## ⚠️ Performance Regressions")
            report.append("")
            for name, data in comparisons.items():
                if data['is_regression']:
                    significance = " (significant)" if data['is_significant'] else ""
                    report.append(f"- **{name}**: {data['change_percent']:+.1f}%{significance}")
                    report.append(f"  - Baseline: {data['baseline_mean']:.4f}s")
                    report.append(f"  - Current: {data['current_mean']:.4f}s")
            report.append("")
        
        # Improvements
        if improvements > 0:
            report.append("## ✅ Performance Improvements")
            report.append("")
            for name, data in comparisons.items():
                if data['is_improvement']:
                    significance = " (significant)" if data['is_significant'] else ""
                    report.append(f"- **{name}**: {data['change_percent']:+.1f}%{significance}")
                    report.append(f"  - Baseline: {data['baseline_mean']:.4f}s")
                    report.append(f"  - Current: {data['current_mean']:.4f}s")
            report.append("")
        
        # Detailed results
        report.append("## Detailed Results")
        report.append("")
        report.append("| Benchmark | Baseline (s) | Current (s) | Change (%) | Status |")
        report.append("|-----------|--------------|-------------|------------|--------|")
        
        for name, data in sorted(comparisons.items()):
            status = "🔴 Regression" if data['is_regression'] else \
                    "🟢 Improvement" if data['is_improvement'] else \
                    "⚪ No change"
            
            report.append(f"| {name} | {data['baseline_mean']:.4f} | "
                         f"{data['current_mean']:.4f} | {data['change_percent']:+.1f}% | {status} |")
        
        return "\n".join(report)


def run_benchmark(output_file: Path) -> bool:
    """Run benchmarks and save results."""
    try:
        cmd = [
            "uvx", "nox", "-s", "benchmark", "--",
            f"--benchmark-json={output_file}"
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode != 0:
            print(f"Benchmark failed: {result.stderr}")
            return False
        
        return output_file.exists()
    
    except Exception as e:
        print(f"Error running benchmark: {e}")
        return False


def main():
    """Main function."""
    parser = argparse.ArgumentParser(description="Performance regression testing")
    parser.add_argument("--baseline", type=Path, 
                       help="Baseline benchmark results file")
    parser.add_argument("--current", type=Path,
                       help="Current benchmark results file")
    parser.add_argument("--threshold", type=float, default=0.1,
                       help="Regression threshold (default: 0.1 = 10%)")
    parser.add_argument("--output", type=Path, default="regression_report.md",
                       help="Output report file")
    parser.add_argument("--run-baseline", action="store_true",
                       help="Run benchmark to create baseline")
    parser.add_argument("--run-current", action="store_true",
                       help="Run benchmark for current comparison")
    parser.add_argument("--fail-on-regression", action="store_true",
                       help="Exit with error code if regressions found")
    
    args = parser.parse_args()
    
    # Create results directory
    results_dir = Path("benchmarks/results")
    results_dir.mkdir(parents=True, exist_ok=True)
    
    # Run baseline if requested
    if args.run_baseline:
        baseline_file = results_dir / "baseline.json"
        print("Running baseline benchmark...")
        if not run_benchmark(baseline_file):
            print("Failed to run baseline benchmark")
            return 1
        args.baseline = baseline_file
    
    # Run current if requested
    if args.run_current:
        current_file = results_dir / "current.json"
        print("Running current benchmark...")
        if not run_benchmark(current_file):
            print("Failed to run current benchmark")
            return 1
        args.current = current_file
    
    # Check required files
    if not args.baseline or not args.baseline.exists():
        print("Baseline file not found. Use --run-baseline or provide --baseline")
        return 1
    
    if not args.current or not args.current.exists():
        print("Current file not found. Use --run-current or provide --current")
        return 1
    
    # Analyze results
    analyzer = RegressionAnalyzer(threshold=args.threshold)
    
    try:
        baseline_results = analyzer.load_results(args.baseline)
        current_results = analyzer.load_results(args.current)
        
        print(f"Loaded {len(baseline_results)} baseline results")
        print(f"Loaded {len(current_results)} current results")
        
        comparisons = analyzer.compare_results(baseline_results, current_results)
        
        # Generate report
        report = analyzer.generate_report(comparisons)
        
        # Save report
        args.output.write_text(report)
        print(f"Report saved to {args.output}")
        
        # Print summary
        regressions = sum(1 for c in comparisons.values() if c['is_regression'])
        improvements = sum(1 for c in comparisons.values() if c['is_improvement'])
        
        print("\nSummary:")
        print(f"  Regressions: {regressions}")
        print(f"  Improvements: {improvements}")
        print(f"  Total benchmarks: {len(comparisons)}")
        
        # Exit with error if regressions found and requested
        if args.fail_on_regression and regressions > 0:
            print(f"\n❌ Found {regressions} performance regressions!")
            return 1
        
        if regressions > 0:
            print(f"\n⚠️  Found {regressions} performance regressions")
        else:
            print("\n✅ No performance regressions found")
        
        return 0
    
    except Exception as e:
        print(f"Error analyzing results: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
