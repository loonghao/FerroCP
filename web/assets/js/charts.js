/**
 * FerroCP Benchmarks Chart Manager
 * Handles creation and management of all charts and visualizations
 */

class ChartManager {
    constructor() {
        this.charts = new Map();
        this.colors = {
            primary: '#2563eb',
            secondary: '#64748b',
            success: '#10b981',
            warning: '#f59e0b',
            error: '#ef4444',
            ubuntu: '#E95420',
            windows: '#0078D4',
            macos: '#000000'
        };
        this.chartDefaults = {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    position: 'top',
                },
                tooltip: {
                    mode: 'index',
                    intersect: false,
                }
            },
            scales: {
                x: {
                    display: true,
                    grid: {
                        display: false
                    }
                },
                y: {
                    display: true,
                    beginAtZero: true,
                    grid: {
                        color: 'rgba(0, 0, 0, 0.1)'
                    }
                }
            }
        };
    }

    /**
     * Create overview performance chart
     */
    createOverviewChart(data) {
        const ctx = document.getElementById('overview-chart');
        if (!ctx || !data || data.length === 0) return;

        // Group data by benchmark name
        const benchmarkGroups = {};
        data.forEach(item => {
            const name = item.name || 'Unknown';
            if (!benchmarkGroups[name]) {
                benchmarkGroups[name] = [];
            }
            benchmarkGroups[name].push(item.mean || 0);
        });

        // Calculate averages
        const labels = Object.keys(benchmarkGroups);
        const averages = labels.map(label => {
            const values = benchmarkGroups[label];
            return values.reduce((sum, val) => sum + val, 0) / values.length;
        });

        const config = {
            type: 'bar',
            data: {
                labels: labels,
                datasets: [{
                    label: 'Average Performance (seconds)',
                    data: averages,
                    backgroundColor: this.colors.primary,
                    borderColor: this.colors.primary,
                    borderWidth: 1
                }]
            },
            options: {
                ...this.chartDefaults,
                plugins: {
                    ...this.chartDefaults.plugins,
                    title: {
                        display: true,
                        text: 'Benchmark Performance Overview'
                    }
                },
                scales: {
                    ...this.chartDefaults.scales,
                    y: {
                        ...this.chartDefaults.scales.y,
                        title: {
                            display: true,
                            text: 'Time (seconds)'
                        }
                    }
                }
            }
        };

        this.destroyChart('overview-chart');
        this.charts.set('overview-chart', new Chart(ctx, config));
    }

    /**
     * Create platform comparison chart
     */
    createPlatformChart(platformData) {
        const ctx = document.getElementById('platform-chart');
        if (!ctx || !platformData) return;

        const platforms = Object.keys(platformData);
        const datasets = [];

        // Get unique benchmark names
        const allBenchmarks = new Set();
        Object.values(platformData).forEach(data => {
            data.forEach(item => allBenchmarks.add(item.name));
        });

        const benchmarkNames = Array.from(allBenchmarks).slice(0, 5); // Limit to top 5

        platforms.forEach((platform, index) => {
            const data = benchmarkNames.map(benchmark => {
                const items = platformData[platform].filter(item => item.name === benchmark);
                if (items.length === 0) return 0;
                return items.reduce((sum, item) => sum + (item.mean || 0), 0) / items.length;
            });

            datasets.push({
                label: platform.charAt(0).toUpperCase() + platform.slice(1),
                data: data,
                backgroundColor: this.colors[platform] || this.colors.secondary,
                borderColor: this.colors[platform] || this.colors.secondary,
                borderWidth: 1
            });
        });

        const config = {
            type: 'bar',
            data: {
                labels: benchmarkNames,
                datasets: datasets
            },
            options: {
                ...this.chartDefaults,
                plugins: {
                    ...this.chartDefaults.plugins,
                    title: {
                        display: true,
                        text: 'Platform Performance Comparison'
                    }
                },
                scales: {
                    ...this.chartDefaults.scales,
                    y: {
                        ...this.chartDefaults.scales.y,
                        title: {
                            display: true,
                            text: 'Time (seconds)'
                        }
                    }
                }
            }
        };

        this.destroyChart('platform-chart');
        this.charts.set('platform-chart', new Chart(ctx, config));
    }

    /**
     * Create interactive timeline chart using Plotly
     */
    createTimelineChart(historicalData, selectedBenchmark = 'all') {
        const container = document.getElementById('timeline-chart');
        if (!container || !historicalData || historicalData.length === 0) return;

        // Prepare data for Plotly
        const traces = {};

        historicalData.forEach(run => {
            run.data.forEach(item => {
                const benchmarkName = item.name;
                
                if (selectedBenchmark !== 'all' && benchmarkName !== selectedBenchmark) {
                    return;
                }

                if (!traces[benchmarkName]) {
                    traces[benchmarkName] = {
                        x: [],
                        y: [],
                        type: 'scatter',
                        mode: 'lines+markers',
                        name: benchmarkName,
                        line: { width: 2 },
                        marker: { size: 6 }
                    };
                }

                traces[benchmarkName].x.push(new Date(run.timestamp));
                traces[benchmarkName].y.push(item.mean || 0);
            });
        });

        const data = Object.values(traces);

        const layout = {
            title: 'Performance Timeline',
            xaxis: {
                title: 'Date',
                type: 'date'
            },
            yaxis: {
                title: 'Time (seconds)',
                type: 'linear'
            },
            hovermode: 'x unified',
            showlegend: true,
            margin: { t: 50, r: 50, b: 50, l: 50 }
        };

        const config = {
            responsive: true,
            displayModeBar: true,
            modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d'],
            displaylogo: false
        };

        Plotly.newPlot(container, data, layout, config);
    }

    /**
     * Create trends analysis chart
     */
    createTrendsChart(historicalData) {
        const container = document.getElementById('trends-chart');
        if (!container || !historicalData || historicalData.length === 0) return;

        // Calculate performance trends
        const benchmarkTrends = {};

        historicalData.forEach(run => {
            run.data.forEach(item => {
                const name = item.name;
                if (!benchmarkTrends[name]) {
                    benchmarkTrends[name] = [];
                }
                benchmarkTrends[name].push({
                    timestamp: new Date(run.timestamp),
                    value: item.mean || 0,
                    run_number: run.run_number
                });
            });
        });

        // Create traces for top benchmarks
        const traces = Object.entries(benchmarkTrends)
            .slice(0, 8) // Limit to top 8 benchmarks
            .map(([name, data]) => {
                data.sort((a, b) => a.timestamp - b.timestamp);
                
                return {
                    x: data.map(d => d.timestamp),
                    y: data.map(d => d.value),
                    type: 'scatter',
                    mode: 'lines+markers',
                    name: name,
                    line: { width: 2 },
                    marker: { size: 4 },
                    hovertemplate: '<b>%{fullData.name}</b><br>' +
                                 'Time: %{y:.6f}s<br>' +
                                 'Date: %{x}<br>' +
                                 '<extra></extra>'
                };
            });

        const layout = {
            title: 'Performance Trends Over Time',
            xaxis: {
                title: 'Date',
                type: 'date'
            },
            yaxis: {
                title: 'Time (seconds)',
                type: 'linear'
            },
            hovermode: 'x unified',
            showlegend: true,
            margin: { t: 50, r: 50, b: 50, l: 50 }
        };

        const config = {
            responsive: true,
            displayModeBar: true,
            modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d'],
            displaylogo: false
        };

        Plotly.newPlot(container, traces, layout, config);
    }

    /**
     * Create platform comparison chart using Plotly
     */
    createPlatformComparisonChart(platformData) {
        const container = document.getElementById('platform-comparison-chart');
        if (!container || !platformData) return;

        const platforms = Object.keys(platformData);
        const benchmarks = new Set();

        // Collect all unique benchmark names
        Object.values(platformData).forEach(data => {
            data.forEach(item => benchmarks.add(item.name));
        });

        const benchmarkNames = Array.from(benchmarks).slice(0, 10);

        const traces = platforms.map(platform => {
            const data = benchmarkNames.map(benchmark => {
                const items = platformData[platform].filter(item => item.name === benchmark);
                if (items.length === 0) return 0;
                return items.reduce((sum, item) => sum + (item.mean || 0), 0) / items.length;
            });

            return {
                x: benchmarkNames,
                y: data,
                type: 'bar',
                name: platform.charAt(0).toUpperCase() + platform.slice(1),
                marker: {
                    color: this.colors[platform] || this.colors.secondary
                }
            };
        });

        const layout = {
            title: 'Cross-Platform Performance Comparison',
            xaxis: {
                title: 'Benchmarks',
                tickangle: -45
            },
            yaxis: {
                title: 'Time (seconds)'
            },
            barmode: 'group',
            showlegend: true,
            margin: { t: 50, r: 50, b: 100, l: 50 }
        };

        const config = {
            responsive: true,
            displayModeBar: true,
            modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d'],
            displaylogo: false
        };

        Plotly.newPlot(container, traces, layout, config);
    }

    /**
     * Update benchmark filter options
     */
    updateBenchmarkFilter(data) {
        const select = document.getElementById('benchmark-filter');
        if (!select || !data) return;

        const benchmarks = new Set();
        data.forEach(item => benchmarks.add(item.name));

        // Clear existing options except "All"
        select.innerHTML = '<option value="all">All Benchmarks</option>';

        // Add benchmark options
        Array.from(benchmarks).sort().forEach(benchmark => {
            const option = document.createElement('option');
            option.value = benchmark;
            option.textContent = benchmark;
            select.appendChild(option);
        });
    }

    /**
     * Download chart as image
     */
    downloadChart(chartId) {
        const chart = this.charts.get(chartId);
        if (chart) {
            const link = document.createElement('a');
            link.download = `${chartId}-${new Date().toISOString().split('T')[0]}.png`;
            link.href = chart.toBase64Image();
            link.click();
        }
    }

    /**
     * Destroy a specific chart
     */
    destroyChart(chartId) {
        const chart = this.charts.get(chartId);
        if (chart) {
            chart.destroy();
            this.charts.delete(chartId);
        }
    }

    /**
     * Destroy all charts
     */
    destroyAllCharts() {
        this.charts.forEach(chart => chart.destroy());
        this.charts.clear();
    }

    /**
     * Resize all charts
     */
    resizeCharts() {
        this.charts.forEach(chart => chart.resize());
    }
}

// Create global instance
window.chartManager = new ChartManager();

// Handle window resize
window.addEventListener('resize', () => {
    if (window.chartManager) {
        window.chartManager.resizeCharts();
    }
});

// Global function for downloading charts
window.downloadChart = function(chartId) {
    if (window.chartManager) {
        window.chartManager.downloadChart(chartId);
    }
};
