#!/usr/bin/env python3
"""
FerroCP Performance Analyzer

This module provides comprehensive performance analysis capabilities for FerroCP,
including benchmark data processing, regression detection, and report generation.

This is the main performance analysis tool for the FerroCP project, designed to be
called from CI/CD workflows and used for local performance analysis.
"""

import json
import os
import sys
import argparse
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

try:
    import pandas as pd
    import numpy as np
    import matplotlib.pyplot as plt
    import seaborn as sns
    import yaml
    HAS_ANALYSIS_DEPS = True

    # Try to import plotly for interactive charts
    try:
        import plotly.graph_objects as go
        import plotly.express as px
        HAS_PLOTLY = True
    except ImportError:
        HAS_PLOTLY = False

except ImportError:
    HAS_ANALYSIS_DEPS = False
    HAS_PLOTLY = False
    print("Warning: Analysis dependencies not available. Install with: pip install pandas numpy matplotlib seaborn pyyaml plotly")

class PerformanceAnalyzer:
    """Main class for performance analysis"""
    
    def __init__(self, config_path: str = ".github/performance-config.yml"):
        """Initialize the analyzer with configuration"""
        self.config = self._load_config(config_path)
        self.results_dir = Path("performance-results")
        self.results_dir.mkdir(exist_ok=True)
        
    def _load_config(self, config_path: str) -> Dict:
        """Load performance configuration"""
        try:
            with open(config_path, 'r') as f:
                if HAS_ANALYSIS_DEPS:
                    return yaml.safe_load(f)
                else:
                    # Fallback to basic JSON-like parsing if yaml not available
                    return {}
        except FileNotFoundError:
            print(f"Warning: Config file {config_path} not found, using defaults")
            return self._default_config()
    
    def _default_config(self) -> Dict:
        """Default configuration if file not found"""
        return {
            'regression_thresholds': {'default': 0.05},
            'retention': {'detailed_results': 90},
            'reporting': {
                'metrics': ['mean', 'median', 'stddev', 'min', 'max'],
                'chart_formats': ['png', 'html']
            }
        }
    
    def load_benchmark_data(self, data_dir: str) -> Tuple[List[Dict], Dict]:
        """Load benchmark data from directory"""
        results = []
        metadata = {
            'timestamp': datetime.now().isoformat(),
            'commit_sha': os.environ.get('GITHUB_SHA', 'unknown'),
            'ref': os.environ.get('GITHUB_REF', 'unknown'),
            'run_id': os.environ.get('GITHUB_RUN_ID', 'unknown'),
            'run_number': os.environ.get('GITHUB_RUN_NUMBER', 'unknown')
        }
        
        data_path = Path(data_dir)
        if not data_path.exists():
            print(f"Warning: Data directory {data_dir} not found")
            return [], metadata
        
        # Load JSON benchmark files
        for json_file in data_path.glob("**/*.json"):
            try:
                with open(json_file) as f:
                    data = json.load(f)
                    for benchmark in data.get('benchmarks', []):
                        result = {
                            'name': benchmark['name'],
                            'mean': benchmark['stats']['mean'],
                            'stddev': benchmark['stats']['stddev'],
                            'min': benchmark['stats']['min'],
                            'max': benchmark['stats']['max'],
                            'median': benchmark['stats'].get('median', benchmark['stats']['mean']),
                            'rounds': benchmark['stats'].get('rounds', 1),
                            'file': json_file.name,
                            'platform': self._extract_platform(json_file.name),
                            'python_version': self._extract_python_version(json_file.name)
                        }
                        result.update(metadata)
                        results.append(result)
            except Exception as e:
                print(f"Error processing {json_file}: {e}")
        
        return results, metadata
    
    def _extract_platform(self, filename: str) -> str:
        """Extract platform from filename"""
        if 'ubuntu' in filename.lower():
            return 'ubuntu'
        elif 'windows' in filename.lower():
            return 'windows'
        elif 'macos' in filename.lower():
            return 'macos'
        return 'unknown'
    
    def _extract_python_version(self, filename: str) -> str:
        """Extract Python version from filename"""
        import re
        match = re.search(r'python-(\d+\.\d+)', filename)
        return match.group(1) if match else 'unknown'
    
    def detect_regressions(self, current_data: List[Dict], baseline_data: List[Dict]) -> Tuple[List[Dict], List[Dict]]:
        """Detect performance regressions and improvements"""
        if not baseline_data:
            print("No baseline data available for comparison")
            return [], []
        
        # Convert to simple dictionaries for easier processing
        current_benchmarks = {item['name']: item['mean'] for item in current_data}
        baseline_benchmarks = {item['name']: item['mean'] for item in baseline_data}
        
        regressions = []
        improvements = []
        threshold = self.config.get('regression_thresholds', {}).get('default', 0.05)
        
        for name in current_benchmarks:
            if name not in baseline_benchmarks:
                continue
            
            baseline_mean = baseline_benchmarks[name]
            current_mean = current_benchmarks[name]
            
            if baseline_mean == 0:
                continue
            
            change_ratio = (current_mean - baseline_mean) / baseline_mean
            
            if change_ratio > threshold:
                regressions.append({
                    'name': name,
                    'baseline': baseline_mean,
                    'current': current_mean,
                    'change_ratio': change_ratio,
                    'change_percent': change_ratio * 100,
                    'threshold': threshold,
                    'severity': self._get_severity(change_ratio, threshold)
                })
            elif change_ratio < -threshold:
                improvements.append({
                    'name': name,
                    'baseline': baseline_mean,
                    'current': current_mean,
                    'change_ratio': change_ratio,
                    'change_percent': change_ratio * 100,
                    'threshold': threshold
                })
        
        return regressions, improvements
    
    def _get_severity(self, change_ratio: float, threshold: float) -> str:
        """Determine severity of regression"""
        if change_ratio > threshold * 3:
            return 'critical'
        elif change_ratio > threshold * 2:
            return 'major'
        elif change_ratio > threshold:
            return 'minor'
        return 'none'
    
    def generate_report(self, data: List[Dict], metadata: Dict, 
                       regressions: List[Dict] = None, improvements: List[Dict] = None) -> str:
        """Generate comprehensive performance report"""
        if not data:
            return "# Performance Report\n\n❌ No benchmark data available"
        
        regressions = regressions or []
        improvements = improvements or []
        
        report = []
        report.append("# 🚀 FerroCP Performance Report")
        report.append(f"**Generated:** {metadata['timestamp']}")
        commit_sha = metadata.get('commit_sha', 'unknown')
        if len(commit_sha) > 8:
            commit_sha = commit_sha[:8]
        report.append(f"**Commit:** {commit_sha}")
        report.append(f"**Run:** #{metadata['run_number']}")
        report.append("")
        
        # Summary statistics
        report.append("## 📊 Performance Summary")
        
        # Group by benchmark name
        benchmark_stats = {}
        for item in data:
            name = item['name']
            if name not in benchmark_stats:
                benchmark_stats[name] = []
            benchmark_stats[name].append(item['mean'])
        
        for name, times in benchmark_stats.items():
            count = len(times)
            mean_time = sum(times) / count
            min_time = min(times)
            max_time = max(times)
            
            report.append(f"### {name}")
            report.append(f"- **Runs:** {count}")
            report.append(f"- **Mean:** {mean_time:.6f}s")
            report.append(f"- **Range:** {min_time:.6f}s - {max_time:.6f}s")
            
            # Check against performance targets
            targets = self.config.get('performance_targets', {})
            target_key = name.replace('test_', '').replace('_', '_')
            if target_key in targets:
                target = targets[target_key]
                status = "✅" if mean_time <= target else "⚠️"
                report.append(f"- **Target:** {target:.6f}s {status}")
            
            report.append("")
        
        # Regression analysis
        if regressions:
            report.append("## ⚠️ Performance Regressions")
            for reg in regressions:
                severity_emoji = {"critical": "🔴", "major": "🟠", "minor": "🟡"}.get(reg['severity'], "⚠️")
                report.append(f"### {severity_emoji} {reg['name']} ({reg['severity'].title()})")
                report.append(f"- **Baseline:** {reg['baseline']:.6f}s")
                report.append(f"- **Current:** {reg['current']:.6f}s")
                report.append(f"- **Change:** +{reg['change_percent']:.2f}% (slower)")
                report.append(f"- **Threshold:** {reg['threshold']*100:.1f}%")
                report.append("")
        
        if improvements:
            report.append("## 🚀 Performance Improvements")
            for imp in improvements:
                report.append(f"### ✅ {imp['name']}")
                report.append(f"- **Baseline:** {imp['baseline']:.6f}s")
                report.append(f"- **Current:** {imp['current']:.6f}s")
                report.append(f"- **Change:** {imp['change_percent']:.2f}% (faster)")
                report.append("")
        
        return "\n".join(report)
    
    def save_results(self, data: List[Dict], metadata: Dict, 
                    regressions: List[Dict], improvements: List[Dict]):
        """Save analysis results"""
        # Save baseline data
        baseline_data = {
            'metadata': metadata,
            'benchmarks': data,
            'regressions': regressions,
            'improvements': improvements
        }
        
        with open(self.results_dir / 'baseline-performance.json', 'w') as f:
            json.dump(baseline_data, f, indent=2)
        
        # Save summary
        summary = {
            'total_benchmarks': len(data),
            'total_regressions': len(regressions),
            'total_improvements': len(improvements),
            'timestamp': metadata['timestamp']
        }
        
        with open(self.results_dir / 'summary.json', 'w') as f:
            json.dump(summary, f, indent=2)

    def create_visualizations(self, data: List[Dict]):
        """Create performance visualizations"""
        if not data or not HAS_ANALYSIS_DEPS:
            print("Skipping visualizations: missing data or dependencies")
            return

        # Convert to DataFrame for easier plotting
        df = pd.DataFrame(data)

        # Set style
        plt.style.use('seaborn-v0_8')
        sns.set_palette("husl")

        # Performance comparison chart
        fig, axes = plt.subplots(2, 2, figsize=(15, 12))
        fig.suptitle('FerroCP Performance Benchmarks', fontsize=16, fontweight='bold')

        # Mean performance by benchmark
        df_mean = df.groupby('name')['mean'].mean().sort_values()
        axes[0, 0].barh(df_mean.index, df_mean.values)
        axes[0, 0].set_title('Mean Performance by Benchmark')
        axes[0, 0].set_xlabel('Time (seconds)')

        # Performance distribution
        if len(df) > 1:
            sns.boxplot(data=df, x='name', y='mean', ax=axes[0, 1])
            axes[0, 1].set_title('Performance Distribution')
            axes[0, 1].tick_params(axis='x', rotation=45)

        # Platform comparison (if available)
        if 'platform' in df.columns and df['platform'].nunique() > 1:
            platform_data = df.groupby(['name', 'platform'])['mean'].mean().unstack(fill_value=0)
            platform_data.plot(kind='bar', ax=axes[1, 0])
            axes[1, 0].set_title('Performance by Platform')
            axes[1, 0].tick_params(axis='x', rotation=45)
            axes[1, 0].legend(title='Platform')

        # Performance trend (if multiple runs)
        if 'run_number' in df.columns and df['run_number'].nunique() > 1:
            for name in df['name'].unique():
                subset = df[df['name'] == name]
                axes[1, 1].plot(subset['run_number'], subset['mean'], marker='o', label=name)
            axes[1, 1].set_title('Performance Trend')
            axes[1, 1].set_xlabel('Run Number')
            axes[1, 1].set_ylabel('Time (seconds)')
            axes[1, 1].legend()

        plt.tight_layout()
        plt.savefig(self.results_dir / 'performance-charts.png', dpi=300, bbox_inches='tight')
        plt.close()

        # Create interactive Plotly chart if available
        if HAS_PLOTLY:
            self._create_interactive_chart(df)

    def _create_interactive_chart(self, df):
        """Create interactive Plotly chart"""
        fig_plotly = go.Figure()

        for name in df['name'].unique():
            subset = df[df['name'] == name]
            fig_plotly.add_trace(go.Scatter(
                x=list(range(len(subset))),
                y=subset['mean'],
                mode='lines+markers',
                name=name,
                hovertemplate=f'<b>{name}</b><br>Time: %{{y:.6f}}s<extra></extra>'
            ))

        fig_plotly.update_layout(
            title='FerroCP Performance Benchmarks',
            xaxis_title='Run Index',
            yaxis_title='Time (seconds)',
            hovermode='x unified'
        )

        fig_plotly.write_html(self.results_dir / 'performance-interactive.html')

def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(description='FerroCP Performance Analyzer')
    parser.add_argument('--data-dir', default='benchmark-artifacts', 
                       help='Directory containing benchmark data')
    parser.add_argument('--baseline-dir', default='baseline-results',
                       help='Directory containing baseline data')
    parser.add_argument('--config', default='.github/performance-config.yml',
                       help='Configuration file path')
    parser.add_argument('--output-dir', default='performance-results',
                       help='Output directory for results')
    
    args = parser.parse_args()
    
    # Initialize analyzer
    analyzer = PerformanceAnalyzer(args.config)
    analyzer.results_dir = Path(args.output_dir)
    analyzer.results_dir.mkdir(exist_ok=True)
    
    # Load current data
    print("🔍 Loading current benchmark data...")
    current_data, metadata = analyzer.load_benchmark_data(args.data_dir)
    
    if not current_data:
        print("❌ No current benchmark data found")
        return 1
    
    print(f"✅ Loaded {len(current_data)} benchmark results")
    
    # Load baseline data
    print("📋 Loading baseline data...")
    baseline_data, _ = analyzer.load_benchmark_data(args.baseline_dir)
    
    # Detect regressions
    print("🔍 Analyzing performance changes...")
    regressions, improvements = analyzer.detect_regressions(current_data, baseline_data)
    
    # Generate report
    print("📊 Generating performance report...")
    report = analyzer.generate_report(current_data, metadata, regressions, improvements)

    with open(analyzer.results_dir / 'performance-report.md', 'w') as f:
        f.write(report)

    # Create visualizations
    print("📈 Creating visualizations...")
    analyzer.create_visualizations(current_data)

    # Save detailed CSV
    if HAS_ANALYSIS_DEPS:
        import pandas as pd
        df = pd.DataFrame(current_data)
        df.to_csv(analyzer.results_dir / 'benchmark-detailed.csv', index=False)

    # Save results
    print("💾 Saving results...")
    analyzer.save_results(current_data, metadata, regressions, improvements)
    
    # Print summary
    print(f"\n✅ Analysis complete!")
    print(f"   📊 {len(current_data)} benchmarks analyzed")
    print(f"   ⚠️  {len(regressions)} regressions detected")
    print(f"   🚀 {len(improvements)} improvements found")
    print(f"   📁 Results saved to {analyzer.results_dir}")
    
    # Exit with error code if critical regressions found
    critical_regressions = [r for r in regressions if r.get('severity') == 'critical']
    if critical_regressions:
        print(f"\n🔴 {len(critical_regressions)} critical regressions detected!")
        return 1
    
    return 0

if __name__ == '__main__':
    sys.exit(main())
