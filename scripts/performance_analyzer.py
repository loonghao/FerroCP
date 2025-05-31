#!/usr/bin/env python3
"""
FerroCP Performance Analyzer (Compatibility Wrapper)

This script provides backward compatibility for the FerroCP performance analysis system.
It now delegates to the refactored analyzer module in scripts/benchmark/analyzer.py

This wrapper ensures that existing CI/CD workflows continue to work without modification
while providing access to the improved, modular performance analysis functionality.
"""

import sys
from pathlib import Path

def main():
    """Main entry point - delegates to the new analyzer module"""
    # Add the benchmark module to the path
    benchmark_dir = Path(__file__).parent / "benchmark"
    sys.path.insert(0, str(benchmark_dir))
    
    try:
        from analyzer import main as analyzer_main
        print("🔄 Using refactored performance analyzer...")
        return analyzer_main()
        
    except ImportError as e:
        print(f"❌ Error importing refactored analyzer: {e}")
        print("📋 Please ensure the benchmark module is properly installed.")
        print("💡 Try running: pip install pandas numpy matplotlib seaborn pyyaml")
        print("❌ Performance analyzer not available due to missing dependencies.")
        return 1

if __name__ == '__main__':
    sys.exit(main())
