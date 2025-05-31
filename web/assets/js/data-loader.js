/**
 * FerroCP Benchmarks Data Loader
 * Handles loading and caching of benchmark data from various sources
 */

class DataLoader {
    constructor() {
        this.cache = new Map();
        this.baseUrl = this.getBaseUrl();
        this.retryAttempts = 3;
        this.retryDelay = 1000;
    }

    /**
     * Get the base URL for data requests
     */
    getBaseUrl() {
        // Check if we're running on GitHub Pages
        if (window.location.hostname.includes('github.io')) {
            return window.location.origin + window.location.pathname.replace(/\/[^\/]*$/, '');
        }
        // Local development - get the directory containing the HTML file
        const path = window.location.pathname;
        const directory = path.substring(0, path.lastIndexOf('/'));
        return window.location.origin + directory;
    }

    /**
     * Generic fetch with retry logic
     */
    async fetchWithRetry(url, options = {}, attempt = 1) {
        try {
            const response = await fetch(url, {
                ...options,
                headers: {
                    'Accept': 'application/json',
                    ...options.headers
                }
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            return response;
        } catch (error) {
            if (attempt < this.retryAttempts) {
                console.warn(`Fetch attempt ${attempt} failed for ${url}, retrying...`, error);
                await this.delay(this.retryDelay * attempt);
                return this.fetchWithRetry(url, options, attempt + 1);
            }
            throw error;
        }
    }

    /**
     * Delay utility for retry logic
     */
    delay(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    /**
     * Load and cache JSON data
     */
    async loadJson(path, useCache = true) {
        const cacheKey = `json:${path}`;
        
        if (useCache && this.cache.has(cacheKey)) {
            return this.cache.get(cacheKey);
        }

        try {
            const url = `${this.baseUrl}/${path}`;
            const response = await this.fetchWithRetry(url);
            const data = await response.json();
            
            if (useCache) {
                this.cache.set(cacheKey, data);
            }
            
            return data;
        } catch (error) {
            console.error(`Failed to load JSON from ${path}:`, error);
            throw new Error(`Failed to load data: ${error.message}`);
        }
    }

    /**
     * Load CSV data and convert to JSON
     */
    async loadCsv(path, useCache = true) {
        const cacheKey = `csv:${path}`;
        
        if (useCache && this.cache.has(cacheKey)) {
            return this.cache.get(cacheKey);
        }

        try {
            const url = `${this.baseUrl}/${path}`;
            const response = await this.fetchWithRetry(url);
            const csvText = await response.text();
            const data = this.parseCsv(csvText);
            
            if (useCache) {
                this.cache.set(cacheKey, data);
            }
            
            return data;
        } catch (error) {
            console.error(`Failed to load CSV from ${path}:`, error);
            throw new Error(`Failed to load CSV data: ${error.message}`);
        }
    }

    /**
     * Parse CSV text to JSON array
     */
    parseCsv(csvText) {
        const lines = csvText.trim().split('\n');
        if (lines.length < 2) {
            return [];
        }

        const headers = lines[0].split(',').map(h => h.trim());
        const data = [];

        for (let i = 1; i < lines.length; i++) {
            const values = lines[i].split(',').map(v => v.trim());
            if (values.length === headers.length) {
                const row = {};
                headers.forEach((header, index) => {
                    let value = values[index];
                    // Try to parse numbers
                    if (!isNaN(value) && value !== '') {
                        value = parseFloat(value);
                    }
                    row[header] = value;
                });
                data.push(row);
            }
        }

        return data;
    }

    /**
     * Load main index data
     */
    async loadIndex() {
        return this.loadJson('index.json');
    }

    /**
     * Load summary data
     */
    async loadSummary() {
        return this.loadJson('summary.json');
    }

    /**
     * Load statistics
     */
    async loadStatistics() {
        return this.loadJson('statistics.json');
    }

    /**
     * Load latest benchmark data
     */
    async loadLatestData() {
        try {
            return await this.loadCsv('latest/latest-detailed.csv');
        } catch (error) {
            // Fallback to index data if latest CSV is not available
            console.warn('Latest CSV not available, falling back to index data');
            const index = await this.loadIndex();
            if (index.latest_run && index.latest_run.files) {
                const csvFile = Object.values(index.latest_run.files).find(f => f.endsWith('.csv'));
                if (csvFile) {
                    return this.loadCsv(csvFile);
                }
            }
            return [];
        }
    }

    /**
     * Load historical data for trends
     */
    async loadHistoricalData(timeRange = 30) {
        const index = await this.loadIndex();
        const cutoffDate = new Date();
        cutoffDate.setDate(cutoffDate.getDate() - timeRange);

        const recentRuns = index.runs.filter(run => {
            const runDate = new Date(run.timestamp);
            return runDate >= cutoffDate;
        });

        const historicalData = [];
        
        // Load data from recent runs (limit to avoid too many requests)
        const maxRuns = Math.min(recentRuns.length, 20);
        for (let i = 0; i < maxRuns; i++) {
            const run = recentRuns[i];
            try {
                const csvFile = Object.values(run.files || {}).find(f => f.endsWith('.csv'));
                if (csvFile) {
                    const data = await this.loadCsv(csvFile, false);
                    historicalData.push({
                        run_number: run.run_number,
                        timestamp: run.timestamp,
                        commit_sha: run.commit_sha,
                        data: data
                    });
                }
            } catch (error) {
                console.warn(`Failed to load data for run ${run.run_number}:`, error);
            }
        }

        return historicalData;
    }

    /**
     * Load platform-specific data
     */
    async loadPlatformData() {
        const data = await this.loadLatestData();
        const platforms = {};

        data.forEach(item => {
            const platform = item.platform || 'unknown';
            if (!platforms[platform]) {
                platforms[platform] = [];
            }
            platforms[platform].push(item);
        });

        return platforms;
    }

    /**
     * Load reports list
     */
    async loadReports(searchTerm = '') {
        const index = await this.loadIndex();
        const reports = [];

        for (const run of index.runs) {
            const reportFile = Object.values(run.files || {}).find(f => f.endsWith('.md'));
            if (reportFile) {
                const report = {
                    run_number: run.run_number,
                    timestamp: run.timestamp,
                    commit_sha: run.commit_sha,
                    ref_name: run.ref_name,
                    file: reportFile,
                    title: `Performance Report #${run.run_number}`,
                    date: new Date(run.timestamp).toLocaleDateString(),
                    time: new Date(run.timestamp).toLocaleTimeString()
                };

                if (!searchTerm || 
                    report.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
                    report.commit_sha.includes(searchTerm) ||
                    report.run_number.toString().includes(searchTerm)) {
                    reports.push(report);
                }
            }
        }

        return reports;
    }

    /**
     * Load specific report content
     */
    async loadReport(reportPath) {
        try {
            const url = `${this.baseUrl}/${reportPath}`;
            const response = await this.fetchWithRetry(url);
            return await response.text();
        } catch (error) {
            console.error(`Failed to load report from ${reportPath}:`, error);
            throw new Error(`Failed to load report: ${error.message}`);
        }
    }

    /**
     * Clear cache
     */
    clearCache() {
        this.cache.clear();
    }

    /**
     * Get cache statistics
     */
    getCacheStats() {
        return {
            size: this.cache.size,
            keys: Array.from(this.cache.keys())
        };
    }

    /**
     * Preload essential data
     */
    async preloadEssentialData() {
        try {
            await Promise.all([
                this.loadSummary(),
                this.loadStatistics(),
                this.loadLatestData()
            ]);
            console.log('Essential data preloaded successfully');
        } catch (error) {
            console.error('Failed to preload essential data:', error);
            throw error;
        }
    }
}

// Create global instance
window.dataLoader = new DataLoader();
