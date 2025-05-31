# FerroCP Performance Benchmarks Web Dashboard

A modern, responsive web dashboard for visualizing FerroCP performance benchmark results.

## Features

### 📊 Real-time Performance Monitoring
- Live performance metrics and statistics
- Interactive charts and visualizations
- Historical trend analysis
- Cross-platform performance comparison

### 🎯 Key Sections

#### Overview
- Performance summary statistics
- Latest benchmark results
- Platform distribution
- Quick performance metrics

#### Trends
- Historical performance analysis
- Trend visualization over time
- Performance regression detection
- Improvement tracking

#### Platforms
- Ubuntu, Windows, and macOS comparison
- Platform-specific performance metrics
- Cross-platform benchmark analysis

#### Reports
- Searchable benchmark reports
- Detailed performance analysis
- Historical report archive
- Direct links to raw data

### 🛠 Technical Stack

- **Frontend**: Pure HTML5, CSS3, JavaScript (ES6+)
- **Charts**: Chart.js + Plotly.js for interactive visualizations
- **Styling**: Modern CSS with CSS Grid and Flexbox
- **Icons**: Font Awesome 6
- **Data**: JSON APIs with CSV fallback
- **Hosting**: GitHub Pages compatible

### 📱 Responsive Design

- Mobile-first responsive design
- Touch-friendly interface
- Optimized for all screen sizes
- Dark mode support (system preference)

## File Structure

```
web/
├── index.html              # Main application page
├── assets/
│   ├── css/
│   │   └── style.css       # Main stylesheet
│   ├── js/
│   │   ├── app.js          # Main application logic
│   │   ├── charts.js       # Chart management
│   │   └── data-loader.js  # Data loading utilities
│   └── favicon.ico         # Site favicon
├── latest/                 # Latest benchmark results
│   └── latest-detailed.csv # Latest CSV data
├── data/                   # Historical data (organized by date)
├── reports/                # Performance reports
├── charts/                 # Generated charts
├── index.json              # Main data index
├── summary.json            # Quick summary data
├── statistics.json         # Detailed statistics
└── README.md              # This file
```

## Data Sources

The dashboard loads data from multiple sources:

1. **index.json** - Main index of all benchmark runs
2. **summary.json** - Quick summary for fast loading
3. **statistics.json** - Detailed statistics and metrics
4. **latest/latest-detailed.csv** - Most recent benchmark data
5. **Historical CSV files** - Time-series data for trends

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

## Performance Features

- **Lazy Loading**: Charts and data loaded on demand
- **Caching**: Intelligent data caching for faster navigation
- **Progressive Enhancement**: Core functionality works without JavaScript
- **Optimized Assets**: Minified CSS and efficient data loading

## Customization

### Colors and Theming
The dashboard uses CSS custom properties for easy theming:

```css
:root {
    --primary-color: #2563eb;
    --secondary-color: #64748b;
    --success-color: #10b981;
    --warning-color: #f59e0b;
    --error-color: #ef4444;
}
```

### Chart Configuration
Charts can be customized in `assets/js/charts.js`:

```javascript
this.colors = {
    primary: '#2563eb',
    ubuntu: '#E95420',
    windows: '#0078D4',
    macos: '#000000'
};
```

## Development

### Local Development
1. Serve the `web/` directory with any HTTP server
2. For example: `python -m http.server 8000`
3. Open `http://localhost:8000` in your browser

### Data Updates
The dashboard automatically refreshes data based on the configured refresh interval. Data is loaded from:
- GitHub Pages (production)
- Local files (development)

### Adding New Charts
1. Add chart creation method to `ChartManager` class
2. Call the method from `BenchmarkApp.updateCharts()`
3. Add corresponding HTML container in `index.html`

## Deployment

### GitHub Pages
The dashboard is designed to work seamlessly with GitHub Pages:

1. Push the `web/` directory to your repository
2. Enable GitHub Pages in repository settings
3. Set source to the branch containing the web files
4. The dashboard will be available at `https://username.github.io/repository-name/`

### Custom Domain
For custom domains, update the `getBaseUrl()` method in `data-loader.js`.

## API Reference

### Data Loader Methods

```javascript
// Load main index
await dataLoader.loadIndex()

// Load summary data
await dataLoader.loadSummary()

// Load latest benchmark data
await dataLoader.loadLatestData()

// Load historical data (last N days)
await dataLoader.loadHistoricalData(30)

// Load platform-specific data
await dataLoader.loadPlatformData()

// Search reports
await dataLoader.loadReports('search term')
```

### Chart Manager Methods

```javascript
// Create overview chart
chartManager.createOverviewChart(data)

// Create platform comparison
chartManager.createPlatformChart(platformData)

// Create interactive timeline
chartManager.createTimelineChart(historicalData)

// Download chart as image
chartManager.downloadChart('chart-id')
```

## Contributing

1. Follow the existing code style and structure
2. Test on multiple browsers and devices
3. Ensure accessibility compliance
4. Update documentation for new features

## License

This dashboard is part of the FerroCP project and follows the same license terms.
