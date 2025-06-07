# FerroCP JSON Output

FerroCP supports JSON output format for automated performance testing and integration with other tools.

## Usage

Add the `--json` flag to any copy command to get structured JSON output:

```bash
ferrocp copy source destination --json
```

## JSON Structure

The JSON output contains comprehensive information about the copy operation:

### Example Output

```json
{
  "metadata": {
    "version": "0.2.0",
    "operation": "copy",
    "timestamp": "2025-06-07T08:50:46.183669500+00:00",
    "source_path": "c:\\test_source",
    "destination_path": "c:\\test_dest_json"
  },
  "source_device": {
    "device_type": "ssd",
    "device_type_description": "Solid State Drive (SSD)",
    "filesystem": "NTFS",
    "total_space_bytes": 2024556982272,
    "available_space_bytes": 980366196736,
    "space_usage_percent": 51.57626061797418,
    "theoretical_read_speed_mbps": 500.0,
    "theoretical_write_speed_mbps": 450.0,
    "optimal_buffer_size_bytes": 1048576,
    "technical_details": {
      "random_read_iops": 50000.0,
      "random_write_iops": 40000.0,
      "average_latency_us": 100.0,
      "queue_depth": 32,
      "supports_trim": true
    }
  },
  "destination_device": {
    "device_type": "ssd",
    "device_type_description": "Solid State Drive (SSD)",
    "filesystem": "NTFS",
    "total_space_bytes": 2024556982272,
    "available_space_bytes": 980366196736,
    "space_usage_percent": 51.57626061797418,
    "theoretical_read_speed_mbps": 500.0,
    "theoretical_write_speed_mbps": 450.0,
    "optimal_buffer_size_bytes": 1048576,
    "technical_details": {
      "random_read_iops": 50000.0,
      "random_write_iops": 40000.0,
      "average_latency_us": 100.0,
      "queue_depth": 32,
      "supports_trim": true
    }
  },
  "performance_analysis": {
    "expected_speed_mbps": 450.0,
    "bottleneck": {
      "device": "destination",
      "description": "Destination Solid State Drive (SSD) limits write speed",
      "limiting_speed_mbps": 450.0
    },
    "recommendations": [
      "Both devices have similar characteristics"
    ]
  },
  "copy_stats": {
    "files_copied": 1,
    "directories_created": 1,
    "bytes_copied": 30,
    "files_skipped": 0,
    "errors": 0,
    "duration_seconds": 0.0124875,
    "actual_transfer_rate_mbps": 0.002291109468843844,
    "zerocopy_operations": 0,
    "zerocopy_efficiency_percent": 0.0,
    "performance_efficiency_percent": 0.0005091354375208543
  },
  "result": {
    "success": true,
    "message": "Copy completed successfully but performance was lower than expected",
    "performance_rating": "poor"
  }
}
```

## Field Descriptions

### Metadata
- `version`: FerroCP version
- `operation`: Type of operation performed
- `timestamp`: ISO 8601 timestamp when operation started
- `source_path`: Source path
- `destination_path`: Destination path

### Device Information
- `device_type`: Type of storage device (ssd, hdd, network, ramdisk, unknown)
- `device_type_description`: Human-readable device type
- `filesystem`: Filesystem type (NTFS, ext4, etc.)
- `total_space_bytes`: Total storage space in bytes
- `available_space_bytes`: Available storage space in bytes
- `space_usage_percent`: Percentage of space used
- `theoretical_read_speed_mbps`: Theoretical read speed in MB/s
- `theoretical_write_speed_mbps`: Theoretical write speed in MB/s
- `optimal_buffer_size_bytes`: Optimal buffer size for this device

### Technical Details
- `random_read_iops`: Random read IOPS
- `random_write_iops`: Random write IOPS
- `average_latency_us`: Average latency in microseconds
- `queue_depth`: Queue depth
- `supports_trim`: Whether TRIM is supported

### Performance Analysis
- `expected_speed_mbps`: Expected transfer speed based on device capabilities
- `bottleneck`: Information about the performance bottleneck
- `recommendations`: List of optimization recommendations

### Copy Statistics
- `files_copied`: Number of files successfully copied
- `directories_created`: Number of directories created
- `bytes_copied`: Total bytes copied
- `files_skipped`: Number of files skipped
- `errors`: Number of errors encountered
- `duration_seconds`: Total operation duration in seconds
- `actual_transfer_rate_mbps`: Actual transfer rate in MB/s
- `zerocopy_operations`: Number of zero-copy operations performed
- `zerocopy_efficiency_percent`: Zero-copy efficiency percentage
- `performance_efficiency_percent`: Performance efficiency compared to expected

### Result
- `success`: Whether the operation was successful
- `message`: Result message
- `performance_rating`: Performance rating (excellent, good, fair, poor)

## Use Cases

### Performance Testing
Use JSON output for automated performance testing:

```bash
# Run multiple tests and collect results
ferrocp copy test1 dest1 --json > result1.json
ferrocp copy test2 dest2 --json > result2.json
ferrocp copy test3 dest3 --json > result3.json

# Analyze results with your preferred tool
python analyze_performance.py result*.json
```

### CI/CD Integration
Integrate FerroCP performance monitoring into your CI/CD pipeline:

```yaml
- name: Performance Test
  run: |
    ferrocp copy source dest --json > performance.json
    python check_performance.py performance.json
```

### Monitoring and Alerting
Monitor copy operations and set up alerts based on performance metrics:

```bash
# Check if efficiency is below threshold
EFFICIENCY=$(ferrocp copy source dest --json | jq '.copy_stats.performance_efficiency_percent')
if (( $(echo "$EFFICIENCY < 50" | bc -l) )); then
  echo "Performance alert: Efficiency is $EFFICIENCY%"
fi
```

### Data Analysis
Export data for analysis in spreadsheets or data analysis tools:

```python
import json
import pandas as pd

# Load JSON results
with open('performance.json') as f:
    data = json.load(f)

# Extract key metrics
metrics = {
    'files_copied': data['copy_stats']['files_copied'],
    'bytes_copied': data['copy_stats']['bytes_copied'],
    'duration': data['copy_stats']['duration_seconds'],
    'efficiency': data['copy_stats']['performance_efficiency_percent'],
    'device_type': data['source_device']['device_type']
}

# Create DataFrame for analysis
df = pd.DataFrame([metrics])
```

## Performance Testing Script

A complete performance testing script is available at `scripts/performance_test.py`. This script:

- Creates test files of various sizes
- Runs FerroCP with different configurations
- Analyzes performance results
- Exports data to CSV for further analysis

Run it with:

```bash
python scripts/performance_test.py
```

## Tips for Performance Testing

1. **Consistent Environment**: Run tests in a consistent environment to get reliable results
2. **Multiple Runs**: Run multiple iterations and average the results
3. **Different Scenarios**: Test with different file sizes and types
4. **Baseline Comparison**: Compare against other tools like robocopy or rsync
5. **Monitor System Resources**: Check CPU, memory, and disk usage during tests

## Troubleshooting

### Empty JSON Output
If you get empty JSON output, check:
- FerroCP version supports JSON output
- No errors in stderr
- Proper command syntax

### Invalid JSON
If JSON parsing fails:
- Check for any non-JSON output mixed in
- Verify the command completed successfully
- Look for error messages in stderr

### Performance Issues
If performance is consistently poor:
- Check disk health
- Monitor system load
- Verify optimal buffer sizes
- Consider different copy modes
