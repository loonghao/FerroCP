#!/usr/bin/env python3
"""
FerroCP Benchmark Index Updater

This script updates the index files in the ferrocp-benchmarks repository
to maintain a searchable catalog of all benchmark data.
"""

import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

class BenchmarkIndexUpdater:
    """Updates index files for benchmark data"""
    
    def __init__(self, repo_dir: Path = Path("ferrocp-benchmarks")):
        self.repo_dir = repo_dir
        self.data_dir = repo_dir / "data"
        self.reports_dir = repo_dir / "reports"
        self.charts_dir = repo_dir / "charts"
        
    def scan_data_files(self) -> List[Dict]:
        """Scan all data files and extract metadata"""
        data_files = []
        
        if not self.data_dir.exists():
            print("⚠️  Data directory not found")
            return data_files
        
        # Scan for metadata files
        for metadata_file in self.data_dir.glob("**/*-metadata.json"):
            try:
                with open(metadata_file, 'r') as f:
                    metadata = json.load(f)
                
                # Add file path information
                metadata['metadata_file'] = str(metadata_file.relative_to(self.repo_dir))
                
                # Validate required fields
                required_fields = ['run_number', 'commit_sha', 'timestamp', 'files']
                if all(field in metadata for field in required_fields):
                    data_files.append(metadata)
                else:
                    print(f"⚠️  Invalid metadata file: {metadata_file}")
                    
            except (json.JSONDecodeError, FileNotFoundError) as e:
                print(f"❌ Error reading metadata file {metadata_file}: {e}")
        
        # Sort by timestamp (newest first)
        data_files.sort(key=lambda x: x['timestamp'], reverse=True)
        
        print(f"📊 Found {len(data_files)} benchmark runs")
        return data_files
    
    def create_main_index(self, data_files: List[Dict]) -> Dict:
        """Create the main index file"""
        index = {
            'last_updated': datetime.now().isoformat(),
            'total_runs': len(data_files),
            'latest_run': data_files[0] if data_files else None,
            'runs': []
        }
        
        # Add summary information for each run
        for data in data_files:
            run_summary = {
                'run_number': data['run_number'],
                'commit_sha': data['commit_sha'],
                'ref_name': data.get('ref_name', 'unknown'),
                'timestamp': data['timestamp'],
                'date': data.get('date', {}),
                'files': list(data['files'].keys()),
                'metadata_file': data['metadata_file']
            }
            index['runs'].append(run_summary)
        
        return index
    
    def create_yearly_index(self, data_files: List[Dict]) -> Dict[str, Dict]:
        """Create yearly index files"""
        yearly_indices = {}
        
        for data in data_files:
            year = data.get('date', {}).get('year', 'unknown')
            if year not in yearly_indices:
                yearly_indices[year] = {
                    'year': year,
                    'last_updated': datetime.now().isoformat(),
                    'total_runs': 0,
                    'months': {}
                }
            
            yearly_indices[year]['total_runs'] += 1
            
            month = data.get('date', {}).get('month', 'unknown')
            if month not in yearly_indices[year]['months']:
                yearly_indices[year]['months'][month] = {
                    'month': month,
                    'runs': []
                }
            
            run_info = {
                'run_number': data['run_number'],
                'commit_sha': data['commit_sha'],
                'timestamp': data['timestamp'],
                'day': data.get('date', {}).get('day', 'unknown'),
                'files': data['files']
            }
            yearly_indices[year]['months'][month]['runs'].append(run_info)
        
        return yearly_indices
    
    def create_statistics(self, data_files: List[Dict]) -> Dict:
        """Create statistics about the benchmark data"""
        if not data_files:
            return {'total_runs': 0, 'date_range': None, 'file_types': {}}
        
        # Date range
        timestamps = [data['timestamp'] for data in data_files]
        date_range = {
            'earliest': min(timestamps),
            'latest': max(timestamps)
        }
        
        # File type statistics
        file_types = {}
        for data in data_files:
            for file_name in data['files'].keys():
                ext = Path(file_name).suffix or 'no_extension'
                file_types[ext] = file_types.get(ext, 0) + 1
        
        # Monthly distribution
        monthly_distribution = {}
        for data in data_files:
            year_month = f"{data.get('date', {}).get('year', 'unknown')}-{data.get('date', {}).get('month', 'unknown')}"
            monthly_distribution[year_month] = monthly_distribution.get(year_month, 0) + 1
        
        # Repository statistics
        repositories = {}
        for data in data_files:
            repo = data.get('repository', 'unknown')
            repositories[repo] = repositories.get(repo, 0) + 1
        
        statistics = {
            'total_runs': len(data_files),
            'date_range': date_range,
            'file_types': file_types,
            'monthly_distribution': monthly_distribution,
            'repositories': repositories,
            'generated_at': datetime.now().isoformat()
        }
        
        return statistics
    
    def save_index_files(self, main_index: Dict, yearly_indices: Dict[str, Dict], statistics: Dict):
        """Save all index files"""
        # Save main index
        main_index_file = self.repo_dir / "index.json"
        with open(main_index_file, 'w') as f:
            json.dump(main_index, f, indent=2)
        print(f"📄 Created main index: {main_index_file}")
        
        # Save yearly indices
        indices_dir = self.repo_dir / "indices"
        indices_dir.mkdir(exist_ok=True)
        
        for year, yearly_index in yearly_indices.items():
            yearly_file = indices_dir / f"{year}.json"
            with open(yearly_file, 'w') as f:
                json.dump(yearly_index, f, indent=2)
            print(f"📄 Created yearly index: {yearly_file}")
        
        # Save statistics
        stats_file = self.repo_dir / "statistics.json"
        with open(stats_file, 'w') as f:
            json.dump(statistics, f, indent=2)
        print(f"📊 Created statistics: {stats_file}")
        
        # Create a simple summary for quick access
        summary = {
            'total_runs': main_index['total_runs'],
            'latest_run': main_index['latest_run'],
            'last_updated': main_index['last_updated'],
            'available_years': list(yearly_indices.keys())
        }
        
        summary_file = self.repo_dir / "summary.json"
        with open(summary_file, 'w') as f:
            json.dump(summary, f, indent=2)
        print(f"📋 Created summary: {summary_file}")
    
    def create_readme_update(self, main_index: Dict, statistics: Dict):
        """Update or create README with current statistics"""
        readme_content = f"""# FerroCP Benchmark Results

This repository contains historical benchmark results for the FerroCP project.

## 📊 Current Statistics

- **Total Benchmark Runs**: {statistics['total_runs']}
- **Date Range**: {statistics['date_range']['earliest'][:10] if statistics['date_range'] else 'N/A'} to {statistics['date_range']['latest'][:10] if statistics['date_range'] else 'N/A'}
- **Last Updated**: {main_index['last_updated'][:19]}

## 📁 Repository Structure

```
data/           # Raw benchmark data organized by year/month
├── YYYY/
│   └── MM/
│       ├── YYYYMMDD-RUN-detailed.csv
│       ├── YYYYMMDD-RUN-baseline.json
│       └── YYYYMMDD-RUN-metadata.json
reports/        # Performance reports
├── YYYY/
│   └── MM/
│       └── YYYYMMDD-RUN-report.md
charts/         # Visualization charts
├── YYYY/
│   └── MM/
│       ├── YYYYMMDD-RUN-charts.png
│       └── YYYYMMDD-RUN-interactive.html
latest/         # Symlinks to latest results
indices/        # Index files for navigation
```

## 🔍 Navigation

- [`index.json`](index.json) - Main index of all benchmark runs
- [`summary.json`](summary.json) - Quick summary of available data
- [`statistics.json`](statistics.json) - Detailed statistics
- [`latest/`](latest/) - Latest benchmark results

## 📈 File Types

"""
        
        for file_type, count in statistics['file_types'].items():
            readme_content += f"- **{file_type}**: {count} files\n"
        
        readme_content += f"""
## 🗓️ Monthly Distribution

"""
        
        for month, count in sorted(statistics['monthly_distribution'].items()):
            readme_content += f"- **{month}**: {count} runs\n"
        
        readme_content += f"""
## 🚀 Latest Run

"""
        
        if main_index['latest_run']:
            latest = main_index['latest_run']
            readme_content += f"""- **Run Number**: #{latest['run_number']}
- **Commit**: {latest['commit_sha'][:8]}
- **Branch**: {latest.get('ref_name', 'unknown')}
- **Date**: {latest['timestamp'][:19]}
- **Files**: {len(latest['files'])} files

"""
        else:
            readme_content += "No benchmark runs available.\n\n"
        
        readme_content += """## 🔗 Links

- [FerroCP Repository](https://github.com/loonghao/py-eacopy)
- [Benchmark Workflow](https://github.com/loonghao/py-eacopy/actions/workflows/benchmark.yml)

---

*This repository is automatically updated by the FerroCP benchmark workflow.*
"""
        
        readme_file = self.repo_dir / "README.md"
        with open(readme_file, 'w') as f:
            f.write(readme_content)
        print(f"📖 Updated README: {readme_file}")
    
    def update_indices(self) -> bool:
        """Main method to update all index files"""
        print("🔄 Updating benchmark indices...")
        
        # Scan for data files
        data_files = self.scan_data_files()
        
        # Create indices
        main_index = self.create_main_index(data_files)
        yearly_indices = self.create_yearly_index(data_files)
        statistics = self.create_statistics(data_files)
        
        # Save all files
        self.save_index_files(main_index, yearly_indices, statistics)
        self.create_readme_update(main_index, statistics)
        
        print("✅ Index update completed successfully")
        return True

def main():
    """Main entry point"""
    import argparse
    
    parser = argparse.ArgumentParser(description='Update benchmark data indices')
    parser.add_argument('--repo-dir', type=Path, default=Path('ferrocp-benchmarks'),
                       help='Path to the ferrocp-benchmarks repository')
    
    args = parser.parse_args()
    
    if not args.repo_dir.exists():
        print(f"❌ Repository directory not found: {args.repo_dir}")
        return 1
    
    updater = BenchmarkIndexUpdater(args.repo_dir)
    
    if updater.update_indices():
        print("✅ Index update completed successfully")
        return 0
    else:
        print("❌ Index update failed")
        return 1

if __name__ == '__main__':
    sys.exit(main())
