#!/usr/bin/env python3
"""
FerroCP Benchmark Utilities

Common utilities and helper functions for benchmark processing and analysis.
"""

import json
import os
import re
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from datetime import datetime

def extract_platform_from_filename(filename: str) -> str:
    """Extract platform information from benchmark filename"""
    filename_lower = filename.lower()
    # Check for specific patterns in order of specificity
    if 'ubuntu' in filename_lower or 'linux' in filename_lower:
        return 'ubuntu'
    elif 'windows' in filename_lower:
        return 'windows'
    elif 'win' in filename_lower and 'darwin' not in filename_lower:
        return 'windows'
    elif 'macos' in filename_lower or 'darwin' in filename_lower:
        return 'macos'
    return 'unknown'

def extract_python_version_from_filename(filename: str) -> str:
    """Extract Python version from benchmark filename"""
    # Look for patterns like python-3.11, py311, python3.11, etc.
    patterns = [
        r'python-(\d+\.\d+)',
        r'py(\d)(\d+)',
        r'python(\d+\.\d+)',
        r'(\d+\.\d+)'
    ]
    
    for pattern in patterns:
        match = re.search(pattern, filename)
        if match:
            if len(match.groups()) == 1:
                return match.group(1)
            elif len(match.groups()) == 2:
                # Handle py311 format
                return f"{match.group(1)}.{match.group(2)}"
    
    return 'unknown'

def load_benchmark_json(file_path: Path) -> List[Dict]:
    """Load benchmark data from a JSON file"""
    try:
        with open(file_path, 'r') as f:
            data = json.load(f)
            return data.get('benchmarks', [])
    except (json.JSONDecodeError, FileNotFoundError) as e:
        print(f"Error loading {file_path}: {e}")
        return []

def get_github_metadata() -> Dict[str, str]:
    """Get GitHub Actions metadata from environment variables"""
    return {
        'timestamp': datetime.now().isoformat(),
        'commit_sha': os.environ.get('GITHUB_SHA', 'unknown'),
        'ref': os.environ.get('GITHUB_REF', 'unknown'),
        'ref_name': os.environ.get('GITHUB_REF_NAME', 'unknown'),
        'run_id': os.environ.get('GITHUB_RUN_ID', 'unknown'),
        'run_number': os.environ.get('GITHUB_RUN_NUMBER', 'unknown'),
        'event_name': os.environ.get('GITHUB_EVENT_NAME', 'unknown'),
        'actor': os.environ.get('GITHUB_ACTOR', 'unknown'),
        'repository': os.environ.get('GITHUB_REPOSITORY', 'unknown')
    }

def format_duration(seconds: float) -> str:
    """Format duration in a human-readable way"""
    if seconds < 0.001:
        return f"{seconds * 1_000_000:.1f}μs"
    elif seconds < 1:
        return f"{seconds * 1000:.1f}ms"
    else:
        return f"{seconds:.3f}s"

def format_change_percentage(change_ratio: float) -> str:
    """Format change percentage with appropriate sign and color indicators"""
    percentage = change_ratio * 100
    if percentage > 0:
        return f"+{percentage:.2f}%"
    else:
        return f"{percentage:.2f}%"

def get_severity_emoji(severity: str) -> str:
    """Get emoji for regression severity"""
    severity_map = {
        'critical': '🔴',
        'major': '🟠', 
        'minor': '🟡',
        'none': '✅'
    }
    return severity_map.get(severity.lower(), '⚠️')

def create_benchmark_summary(benchmarks: List[Dict]) -> Dict:
    """Create a summary of benchmark results"""
    if not benchmarks:
        return {}
    
    # Group by benchmark name
    grouped = {}
    for benchmark in benchmarks:
        name = benchmark['name']
        if name not in grouped:
            grouped[name] = []
        grouped[name].append(benchmark['mean'])
    
    summary = {}
    for name, times in grouped.items():
        summary[name] = {
            'count': len(times),
            'mean': sum(times) / len(times),
            'min': min(times),
            'max': max(times),
            'std': calculate_std(times) if len(times) > 1 else 0.0
        }
    
    return summary

def calculate_std(values: List[float]) -> float:
    """Calculate standard deviation"""
    if len(values) < 2:
        return 0.0
    
    mean = sum(values) / len(values)
    variance = sum((x - mean) ** 2 for x in values) / (len(values) - 1)
    return variance ** 0.5

def validate_benchmark_data(data: List[Dict]) -> Tuple[bool, List[str]]:
    """Validate benchmark data structure and return validation results"""
    errors = []
    
    if not data:
        errors.append("No benchmark data provided")
        return False, errors
    
    required_fields = ['name', 'mean', 'stddev', 'min', 'max']
    
    for i, benchmark in enumerate(data):
        for field in required_fields:
            if field not in benchmark:
                errors.append(f"Benchmark {i}: Missing required field '{field}'")
            elif not isinstance(benchmark[field], (int, float)) and field != 'name':
                errors.append(f"Benchmark {i}: Field '{field}' must be numeric")
        
        # Validate logical constraints
        if 'mean' in benchmark and 'min' in benchmark and 'max' in benchmark:
            if benchmark['min'] > benchmark['mean']:
                errors.append(f"Benchmark {i} ({benchmark.get('name', 'unknown')}): min > mean")
            if benchmark['mean'] > benchmark['max']:
                errors.append(f"Benchmark {i} ({benchmark.get('name', 'unknown')}): mean > max")
    
    return len(errors) == 0, errors

def filter_benchmarks_by_pattern(benchmarks: List[Dict], patterns: List[str]) -> List[Dict]:
    """Filter benchmarks by name patterns"""
    if not patterns:
        return benchmarks
    
    filtered = []
    for benchmark in benchmarks:
        name = benchmark.get('name', '')
        for pattern in patterns:
            # Simple wildcard matching
            pattern_regex = pattern.replace('*', '.*')
            if re.match(pattern_regex, name):
                filtered.append(benchmark)
                break
    
    return filtered

def ensure_directory_exists(path: Path) -> None:
    """Ensure a directory exists, creating it if necessary"""
    path.mkdir(parents=True, exist_ok=True)

def safe_divide(numerator: float, denominator: float, default: float = 0.0) -> float:
    """Safely divide two numbers, returning default if denominator is zero"""
    if denominator == 0:
        return default
    return numerator / denominator

def get_benchmark_category(benchmark_name: str, categories: Dict[str, Dict]) -> str:
    """Determine the category of a benchmark based on its name"""
    for category, config in categories.items():
        patterns = config.get('patterns', [])
        for pattern in patterns:
            pattern_regex = pattern.replace('*', '.*')
            if re.match(pattern_regex, benchmark_name):
                return category
    return 'standard'  # Default category

def create_data_index(data_dir: Path) -> Dict:
    """Create an index of available benchmark data files"""
    index = {
        'files': [],
        'last_updated': datetime.now().isoformat(),
        'total_files': 0
    }
    
    if not data_dir.exists():
        return index
    
    for json_file in data_dir.glob("**/*.json"):
        try:
            stat = json_file.stat()
            file_info = {
                'path': str(json_file.relative_to(data_dir)),
                'size': stat.st_size,
                'modified': datetime.fromtimestamp(stat.st_mtime).isoformat(),
                'platform': extract_platform_from_filename(json_file.name),
                'python_version': extract_python_version_from_filename(json_file.name)
            }
            index['files'].append(file_info)
        except Exception as e:
            print(f"Error indexing {json_file}: {e}")
    
    index['total_files'] = len(index['files'])
    return index

def save_json_safely(data: Dict, file_path: Path) -> bool:
    """Safely save JSON data to file with error handling"""
    try:
        ensure_directory_exists(file_path.parent)
        with open(file_path, 'w') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        return True
    except Exception as e:
        print(f"Error saving JSON to {file_path}: {e}")
        return False

def load_json_safely(file_path: Path) -> Optional[Dict]:
    """Safely load JSON data from file with error handling"""
    try:
        with open(file_path, 'r') as f:
            return json.load(f)
    except Exception as e:
        print(f"Error loading JSON from {file_path}: {e}")
        return None
