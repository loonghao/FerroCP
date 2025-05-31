/**
 * FerroCP Benchmarks Main Application
 * Handles application initialization, navigation, and data coordination
 */

class BenchmarkApp {
    constructor() {
        this.currentSection = 'overview';
        this.data = {
            summary: null,
            statistics: null,
            latest: null,
            historical: null,
            platforms: null
        };
        this.isLoading = false;
        this.timeRange = 30; // Default to 30 days
    }

    /**
     * Initialize the application
     */
    async init() {
        try {
            console.log('🚀 Initializing FerroCP Benchmarks app...');
            this.showLoading(true);

            console.log('📋 Setting up event listeners...');
            this.setupEventListeners();

            console.log('📊 Loading initial data...');
            await this.loadInitialData();

            console.log('🎨 Updating UI...');
            this.updateUI();

            this.showLoading(false);
            console.log('✅ FerroCP Benchmarks app initialized successfully');
        } catch (error) {
            console.error('❌ Failed to initialize app:', error);
            this.showError(`Initialization failed: ${error.message}`);
        }
    }

    /**
     * Setup event listeners
     */
    setupEventListeners() {
        // Navigation
        document.querySelectorAll('.nav-link').forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const section = link.dataset.section;
                this.navigateToSection(section);
            });
        });

        // Time range filter
        const timeRangeSelect = document.getElementById('time-range');
        if (timeRangeSelect) {
            timeRangeSelect.addEventListener('change', (e) => {
                this.timeRange = parseInt(e.target.value) || 30;
                this.refreshData();
            });
        }

        // Refresh button
        const refreshBtn = document.getElementById('refresh-btn');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', () => {
                this.refreshData();
            });
        }

        // Benchmark filter
        const benchmarkFilter = document.getElementById('benchmark-filter');
        if (benchmarkFilter) {
            benchmarkFilter.addEventListener('change', (e) => {
                this.updateTimelineChart(e.target.value);
            });
        }

        // Search reports
        const searchReports = document.getElementById('search-reports');
        if (searchReports) {
            searchReports.addEventListener('input', (e) => {
                this.searchReports(e.target.value);
            });
        }
    }

    /**
     * Load initial data
     */
    async loadInitialData() {
        try {
            console.log('📥 Loading essential data...');

            // Load essential data in parallel
            const [summary, statistics, latest] = await Promise.all([
                window.dataLoader.loadSummary().catch(e => {
                    console.warn('Failed to load summary:', e);
                    return null;
                }),
                window.dataLoader.loadStatistics().catch(e => {
                    console.warn('Failed to load statistics:', e);
                    return null;
                }),
                window.dataLoader.loadLatestData().catch(e => {
                    console.warn('Failed to load latest data:', e);
                    return [];
                })
            ]);

            this.data.summary = summary;
            this.data.statistics = statistics;
            this.data.latest = latest;

            console.log('📊 Loaded data:', {
                summary: !!summary,
                statistics: !!statistics,
                latest: latest?.length || 0
            });

            // Load additional data
            console.log('📈 Loading additional data...');
            await this.loadAdditionalData();
        } catch (error) {
            console.error('❌ Failed to load initial data:', error);
            throw error;
        }
    }

    /**
     * Load additional data (historical, platforms)
     */
    async loadAdditionalData() {
        try {
            const [historical, platforms] = await Promise.all([
                window.dataLoader.loadHistoricalData(this.timeRange),
                window.dataLoader.loadPlatformData()
            ]);

            this.data.historical = historical;
            this.data.platforms = platforms;
        } catch (error) {
            console.warn('Failed to load additional data:', error);
        }
    }

    /**
     * Update the UI with loaded data
     */
    updateUI() {
        this.updateStats();
        this.updateCharts();
        this.updateLastUpdated();
    }

    /**
     * Update statistics cards
     */
    updateStats() {
        const { summary, statistics, latest } = this.data;

        // Total runs
        const totalRunsEl = document.getElementById('total-runs');
        if (totalRunsEl && summary) {
            totalRunsEl.textContent = summary.total_runs || 0;
        }

        // Latest run
        const latestRunEl = document.getElementById('latest-run');
        if (latestRunEl && summary && summary.latest_run) {
            const date = new Date(summary.latest_run.timestamp);
            latestRunEl.textContent = date.toLocaleDateString();
        }

        // Platforms count
        const platformsCountEl = document.getElementById('platforms-count');
        if (platformsCountEl && this.data.platforms) {
            platformsCountEl.textContent = Object.keys(this.data.platforms).length;
        }

        // Average performance
        const avgPerformanceEl = document.getElementById('avg-performance');
        if (avgPerformanceEl && latest && latest.length > 0) {
            const avgTime = latest.reduce((sum, item) => sum + (item.mean || 0), 0) / latest.length;
            avgPerformanceEl.textContent = `${avgTime.toFixed(3)}s`;
        }
    }

    /**
     * Update all charts
     */
    updateCharts() {
        const { latest, platforms, historical } = this.data;

        if (latest && latest.length > 0) {
            window.chartManager.createOverviewChart(latest);
            window.chartManager.updateBenchmarkFilter(latest);
        }

        if (platforms) {
            window.chartManager.createPlatformChart(platforms);
            window.chartManager.createPlatformComparisonChart(platforms);
            this.updatePlatformStats(platforms);
        }

        if (historical && historical.length > 0) {
            window.chartManager.createTimelineChart(historical);
            window.chartManager.createTrendsChart(historical);
        }
    }

    /**
     * Update platform statistics
     */
    updatePlatformStats(platforms) {
        Object.entries(platforms).forEach(([platform, data]) => {
            const statsEl = document.getElementById(`${platform}-stats`);
            if (statsEl && data.length > 0) {
                const avgTime = data.reduce((sum, item) => sum + (item.mean || 0), 0) / data.length;
                const benchmarkCount = new Set(data.map(item => item.name)).size;
                
                statsEl.innerHTML = `
                    <div class="stat-item">
                        <strong>Avg Time:</strong> ${avgTime.toFixed(3)}s
                    </div>
                    <div class="stat-item">
                        <strong>Benchmarks:</strong> ${benchmarkCount}
                    </div>
                    <div class="stat-item">
                        <strong>Runs:</strong> ${data.length}
                    </div>
                `;
            }
        });
    }

    /**
     * Update timeline chart with selected benchmark
     */
    updateTimelineChart(selectedBenchmark) {
        if (this.data.historical) {
            window.chartManager.createTimelineChart(this.data.historical, selectedBenchmark);
        }
    }

    /**
     * Navigate to a specific section
     */
    navigateToSection(sectionName) {
        // Update navigation
        document.querySelectorAll('.nav-link').forEach(link => {
            link.classList.remove('active');
        });
        document.querySelector(`[data-section="${sectionName}"]`).classList.add('active');

        // Update sections
        document.querySelectorAll('.section').forEach(section => {
            section.classList.remove('active');
        });
        document.getElementById(`${sectionName}-section`).classList.add('active');

        this.currentSection = sectionName;

        // Load section-specific data
        this.loadSectionData(sectionName);
    }

    /**
     * Load section-specific data
     */
    async loadSectionData(sectionName) {
        try {
            switch (sectionName) {
                case 'reports':
                    await this.loadReports();
                    break;
                case 'trends':
                    if (!this.data.historical) {
                        await this.loadAdditionalData();
                        this.updateCharts();
                    }
                    break;
                case 'platforms':
                    if (!this.data.platforms) {
                        await this.loadAdditionalData();
                        this.updateCharts();
                    }
                    break;
            }
        } catch (error) {
            console.error(`Failed to load data for section ${sectionName}:`, error);
        }
    }

    /**
     * Load and display reports
     */
    async loadReports() {
        try {
            const reports = await window.dataLoader.loadReports();
            this.displayReports(reports);
        } catch (error) {
            console.error('Failed to load reports:', error);
        }
    }

    /**
     * Display reports list
     */
    displayReports(reports) {
        const reportsListEl = document.getElementById('reports-list');
        if (!reportsListEl) return;

        if (reports.length === 0) {
            reportsListEl.innerHTML = '<p class="text-muted">No reports found.</p>';
            return;
        }

        reportsListEl.innerHTML = reports.map(report => `
            <div class="report-item">
                <div class="report-header">
                    <h4 class="report-title">${report.title}</h4>
                    <div class="report-meta">
                        <span><i class="fas fa-calendar"></i> ${report.date}</span>
                        <span><i class="fas fa-clock"></i> ${report.time}</span>
                        <span><i class="fas fa-code-branch"></i> ${report.commit_sha.substring(0, 8)}</span>
                    </div>
                </div>
                <div class="report-summary">
                    Performance report for run #${report.run_number} on ${report.ref_name} branch
                </div>
                <div class="report-actions">
                    <a href="${report.file}" target="_blank" class="btn btn-sm btn-primary">
                        <i class="fas fa-external-link-alt"></i> View Report
                    </a>
                </div>
            </div>
        `).join('');
    }

    /**
     * Search reports
     */
    async searchReports(searchTerm) {
        try {
            const reports = await window.dataLoader.loadReports(searchTerm);
            this.displayReports(reports);
        } catch (error) {
            console.error('Failed to search reports:', error);
        }
    }

    /**
     * Refresh all data
     */
    async refreshData() {
        if (this.isLoading) return;

        try {
            this.isLoading = true;
            this.showLoading(true);
            
            // Clear cache and reload data
            window.dataLoader.clearCache();
            await this.loadInitialData();
            this.updateUI();
            
            this.showLoading(false);
            this.isLoading = false;
        } catch (error) {
            console.error('Failed to refresh data:', error);
            this.showError('Failed to refresh data');
            this.isLoading = false;
        }
    }

    /**
     * Update last updated timestamp
     */
    updateLastUpdated() {
        const lastUpdatedEl = document.getElementById('last-updated');
        if (lastUpdatedEl) {
            lastUpdatedEl.textContent = new Date().toLocaleString();
        }
    }

    /**
     * Show/hide loading indicator
     */
    showLoading(show) {
        const loadingEl = document.getElementById('loading');
        const errorEl = document.getElementById('error');
        
        if (loadingEl) {
            loadingEl.style.display = show ? 'flex' : 'none';
        }
        if (errorEl) {
            errorEl.classList.add('hidden');
        }
    }

    /**
     * Show error message
     */
    showError(message) {
        const loadingEl = document.getElementById('loading');
        const errorEl = document.getElementById('error');
        const errorMessageEl = document.getElementById('error-message');
        
        if (loadingEl) {
            loadingEl.style.display = 'none';
        }
        if (errorEl) {
            errorEl.classList.remove('hidden');
        }
        if (errorMessageEl) {
            errorMessageEl.textContent = message;
        }
    }
}

// Initialize app when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.benchmarkApp = new BenchmarkApp();
    window.benchmarkApp.init();
});
