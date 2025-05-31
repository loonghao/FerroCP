#!/usr/bin/env python3
"""
FerroCP Benchmark Data Pusher

This script handles pushing benchmark results to the ferrocp-benchmarks repository
for long-term storage and web visualization.
"""

import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

class BenchmarkDataPusher:
    """Handles pushing benchmark data to ferrocp-benchmarks repository"""
    
    def __init__(self, target_repo: str = "loonghao/ferrocp-benchmarks"):
        self.target_repo = target_repo
        self.target_dir = Path("ferrocp-benchmarks")
        self.github_token = os.environ.get('GITHUB_TOKEN')
        self.run_number = os.environ.get('GITHUB_RUN_NUMBER', 'unknown')
        self.commit_sha = os.environ.get('GITHUB_SHA', 'unknown')
        self.ref_name = os.environ.get('GITHUB_REF_NAME', 'unknown')
        
        # Date-based organization
        self.current_date = datetime.now()
        self.year = self.current_date.strftime('%Y')
        self.month = self.current_date.strftime('%m')
        self.day = self.current_date.strftime('%d')
        
    def setup_git_config(self):
        """Configure git for automated commits"""
        try:
            subprocess.run([
                'git', 'config', '--global', 'user.name', 'github-actions[bot]'
            ], check=True, cwd=self.target_dir)
            
            subprocess.run([
                'git', 'config', '--global', 'user.email', 
                'github-actions[bot]@users.noreply.github.com'
            ], check=True, cwd=self.target_dir)
            
            print("✅ Git configuration set up successfully")
            return True
        except subprocess.CalledProcessError as e:
            print(f"❌ Failed to configure git: {e}")
            return False
    
    def clone_target_repository(self) -> bool:
        """Clone the ferrocp-benchmarks repository"""
        if self.target_dir.exists():
            shutil.rmtree(self.target_dir)
        
        try:
            clone_url = f"https://github.com/{self.target_repo}.git"
            if self.github_token:
                clone_url = f"https://{self.github_token}@github.com/{self.target_repo}.git"
            
            subprocess.run([
                'git', 'clone', clone_url, str(self.target_dir)
            ], check=True)
            
            print(f"✅ Successfully cloned {self.target_repo}")
            return True
        except subprocess.CalledProcessError as e:
            print(f"❌ Failed to clone repository: {e}")
            return False
    
    def create_directory_structure(self):
        """Create the data directory structure"""
        data_dir = self.target_dir / "data" / self.year / self.month
        reports_dir = self.target_dir / "reports" / self.year / self.month
        charts_dir = self.target_dir / "charts" / self.year / self.month
        
        for directory in [data_dir, reports_dir, charts_dir]:
            directory.mkdir(parents=True, exist_ok=True)
            print(f"📁 Created directory: {directory}")
        
        return data_dir, reports_dir, charts_dir
    
    def copy_benchmark_files(self, data_dir: Path, reports_dir: Path, charts_dir: Path) -> Dict[str, str]:
        """Copy benchmark files to the target repository"""
        copied_files = {}
        timestamp = f"{self.year}{self.month}{self.day}-{self.run_number}"
        
        # File mappings: source -> (target_dir, new_name)
        file_mappings = {
            'benchmark-detailed.csv': (data_dir, f"{timestamp}-detailed.csv"),
            'performance-report.md': (reports_dir, f"{timestamp}-report.md"),
            'performance-charts.png': (charts_dir, f"{timestamp}-charts.png"),
            'performance-interactive.html': (charts_dir, f"{timestamp}-interactive.html"),
            'baseline-performance.json': (data_dir, f"{timestamp}-baseline.json")
        }
        
        for source_file, (target_dir, new_name) in file_mappings.items():
            source_path = Path(source_file)
            if source_path.exists():
                target_path = target_dir / new_name
                shutil.copy2(source_path, target_path)
                copied_files[source_file] = str(target_path.relative_to(self.target_dir))
                print(f"📄 Copied {source_file} -> {target_path.relative_to(self.target_dir)}")
            else:
                print(f"⚠️  Source file not found: {source_file}")
        
        return copied_files
    
    def create_run_metadata(self, copied_files: Dict[str, str]) -> Dict:
        """Create metadata for this benchmark run"""
        metadata = {
            'run_number': self.run_number,
            'commit_sha': self.commit_sha,
            'ref_name': self.ref_name,
            'timestamp': self.current_date.isoformat(),
            'date': {
                'year': self.year,
                'month': self.month,
                'day': self.day
            },
            'files': copied_files,
            'repository': 'loonghao/py-eacopy',
            'workflow': 'benchmark.yml'
        }
        
        # Save metadata file
        metadata_file = self.target_dir / "data" / self.year / self.month / f"{self.year}{self.month}{self.day}-{self.run_number}-metadata.json"
        with open(metadata_file, 'w') as f:
            json.dump(metadata, f, indent=2)
        
        print(f"📋 Created metadata file: {metadata_file.relative_to(self.target_dir)}")
        return metadata
    
    def update_latest_symlinks(self, copied_files: Dict[str, str]):
        """Update symlinks to point to the latest files"""
        latest_dir = self.target_dir / "latest"
        latest_dir.mkdir(exist_ok=True)
        
        for source_file, target_path in copied_files.items():
            if source_file in ['benchmark-detailed.csv', 'performance-report.md', 'performance-charts.png']:
                # Create symlink to latest file
                symlink_name = source_file.replace('benchmark-detailed.csv', 'latest-detailed.csv')
                symlink_name = symlink_name.replace('performance-report.md', 'latest-report.md')
                symlink_name = symlink_name.replace('performance-charts.png', 'latest-charts.png')
                
                symlink_path = latest_dir / symlink_name
                
                # Remove existing symlink if it exists
                if symlink_path.exists() or symlink_path.is_symlink():
                    symlink_path.unlink()
                
                # Create relative symlink
                relative_target = Path("..") / target_path
                try:
                    symlink_path.symlink_to(relative_target)
                    print(f"🔗 Created symlink: {symlink_path.relative_to(self.target_dir)} -> {relative_target}")
                except OSError:
                    # Fallback: copy file instead of symlink (for Windows compatibility)
                    shutil.copy2(self.target_dir / target_path, symlink_path)
                    print(f"📄 Copied (symlink fallback): {symlink_path.relative_to(self.target_dir)}")
    
    def commit_and_push(self, metadata: Dict) -> bool:
        """Commit and push changes to the repository"""
        try:
            # Add all changes
            subprocess.run(['git', 'add', '.'], check=True, cwd=self.target_dir)
            
            # Check if there are changes to commit
            result = subprocess.run(
                ['git', 'diff', '--cached', '--quiet'], 
                cwd=self.target_dir, 
                capture_output=True
            )
            
            if result.returncode == 0:
                print("ℹ️  No changes to commit")
                return True
            
            # Commit changes
            commit_message = f"Add benchmark results for run #{self.run_number}\n\n" \
                           f"- Commit: {self.commit_sha[:8]}\n" \
                           f"- Branch: {self.ref_name}\n" \
                           f"- Date: {self.current_date.strftime('%Y-%m-%d %H:%M:%S')}\n" \
                           f"- Files: {len(metadata['files'])} files added"
            
            subprocess.run([
                'git', 'commit', '-m', commit_message
            ], check=True, cwd=self.target_dir)
            
            # Push changes
            subprocess.run(['git', 'push'], check=True, cwd=self.target_dir)
            
            print("✅ Successfully committed and pushed changes")
            return True
            
        except subprocess.CalledProcessError as e:
            print(f"❌ Failed to commit and push: {e}")
            return False
    
    def push_data(self) -> bool:
        """Main method to push benchmark data"""
        print("🚀 Starting benchmark data push process...")
        
        # Step 1: Clone repository
        if not self.clone_target_repository():
            return False
        
        # Step 2: Set up git configuration
        if not self.setup_git_config():
            return False
        
        # Step 3: Create directory structure
        data_dir, reports_dir, charts_dir = self.create_directory_structure()
        
        # Step 4: Copy benchmark files
        copied_files = self.copy_benchmark_files(data_dir, reports_dir, charts_dir)
        
        if not copied_files:
            print("⚠️  No files were copied, skipping push")
            return True
        
        # Step 5: Create metadata
        metadata = self.create_run_metadata(copied_files)
        
        # Step 6: Update latest symlinks
        self.update_latest_symlinks(copied_files)
        
        # Step 7: Commit and push
        return self.commit_and_push(metadata)

def main():
    """Main entry point"""
    import argparse
    
    parser = argparse.ArgumentParser(description='Push benchmark data to ferrocp-benchmarks repository')
    parser.add_argument('--repo', default='loonghao/ferrocp-benchmarks',
                       help='Target repository (default: loonghao/ferrocp-benchmarks)')
    parser.add_argument('--dry-run', action='store_true',
                       help='Perform a dry run without actually pushing')
    
    args = parser.parse_args()
    
    if args.dry_run:
        print("🔍 Dry run mode - no changes will be pushed")
        return 0
    
    # Check for required environment variables
    if not os.environ.get('GITHUB_TOKEN'):
        print("❌ GITHUB_TOKEN environment variable is required")
        return 1
    
    pusher = BenchmarkDataPusher(args.repo)
    
    if pusher.push_data():
        print("✅ Benchmark data push completed successfully")
        return 0
    else:
        print("❌ Benchmark data push failed")
        return 1

if __name__ == '__main__':
    sys.exit(main())
