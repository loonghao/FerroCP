/**
 * Modern Rez GUI - JavaScript Application
 * Handles navigation, interactions, and state management
 */

class RezGUI {
    constructor() {
        this.currentPage = 'overview';
        this.theme = localStorage.getItem('rez-theme') || 'light';
        this.init();
    }

    init() {
        this.setupEventListeners();
        this.applyTheme();
        this.loadInitialData();
    }

    setupEventListeners() {
        // Navigation
        document.querySelectorAll('.nav-link').forEach(link => {
            link.addEventListener('click', (e) => {
                e.preventDefault();
                const page = link.dataset.page;
                this.navigateToPage(page);
            });
        });

        // Search functionality
        const searchInput = document.querySelector('.search-input');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.handleSearch(e.target.value);
            });
        }

        // Test connection
        const greetButton = document.querySelector('.btn:has(.fa-paper-plane)');
        if (greetButton) {
            greetButton.addEventListener('click', () => {
                this.testConnection();
            });
        }

        // Quick action buttons
        document.querySelectorAll('.btn').forEach(btn => {
            if (btn.textContent.includes('Browse Packages')) {
                btn.addEventListener('click', () => this.navigateToPage('packages'));
            } else if (btn.textContent.includes('Create Context')) {
                btn.addEventListener('click', () => this.navigateToPage('contexts'));
            } else if (btn.textContent.includes('View Dependencies')) {
                btn.addEventListener('click', () => this.navigateToPage('dependencies'));
            }
        });

        // Start Sidecar button
        const startSidecarBtn = document.querySelector('.btn:has(.fa-play)');
        if (startSidecarBtn) {
            startSidecarBtn.addEventListener('click', () => {
                this.startPythonSidecar();
            });
        }

        // Refresh button
        const refreshBtn = document.querySelector('.btn:has(.fa-sync-alt)');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', () => {
                this.refreshData();
            });
        }
    }

    navigateToPage(pageName) {
        // Update navigation
        document.querySelectorAll('.nav-link').forEach(link => {
            link.classList.remove('active');
        });
        document.querySelector(`[data-page="${pageName}"]`).classList.add('active');

        // Update page content
        document.querySelectorAll('.page').forEach(page => {
            page.style.display = 'none';
        });
        document.getElementById(`${pageName}-page`).style.display = 'block';

        // Update page title
        const titles = {
            overview: 'Welcome to Rez GUI',
            packages: 'Package Browser',
            contexts: 'Context Manager',
            dependencies: 'Dependency Graph',
            settings: 'Settings'
        };
        document.querySelector('.page-title').textContent = titles[pageName] || 'Rez GUI';

        this.currentPage = pageName;
        this.loadPageData(pageName);
    }

    loadPageData(pageName) {
        switch (pageName) {
            case 'packages':
                this.loadPackages();
                break;
            case 'contexts':
                this.loadContexts();
                break;
            case 'dependencies':
                this.loadDependencies();
                break;
            case 'settings':
                this.loadSettings();
                break;
        }
    }

    loadPackages() {
        // Simulate loading packages
        console.log('Loading packages...');
        // In a real app, this would fetch from the backend
    }

    loadContexts() {
        // Simulate loading contexts
        console.log('Loading contexts...');
        // In a real app, this would fetch from the backend
    }

    loadDependencies() {
        // Simulate loading dependency graph
        console.log('Loading dependency graph...');
        // In a real app, this would fetch from the backend
    }

    loadSettings() {
        // Simulate loading settings
        console.log('Loading settings...');
        // In a real app, this would fetch from the backend
    }

    handleSearch(query) {
        console.log('Searching for:', query);
        // Implement search functionality
        // This would filter packages, contexts, etc. based on the query
    }

    testConnection() {
        const input = document.querySelector('.form-input');
        const name = input.value.trim() || 'World';
        
        // Simulate API call
        this.showNotification(`Hello, ${name}! Connection test successful.`, 'success');
        input.value = '';
    }

    startPythonSidecar() {
        // Simulate starting Python sidecar
        this.showNotification('Starting Python Sidecar...', 'info');
        
        // Update UI to show loading state
        const card = document.querySelector('.card:has(.fa-play)');
        const badge = card.querySelector('.card-badge');
        const statusIndicator = card.querySelector('.status-indicator');
        const button = card.querySelector('.btn');
        
        badge.textContent = 'Starting...';
        badge.className = 'card-badge warning';
        statusIndicator.innerHTML = '<div class="status-dot"></div>Starting...';
        statusIndicator.className = 'status-indicator warning';
        button.disabled = true;
        
        // Simulate async operation
        setTimeout(() => {
            badge.textContent = 'Running';
            badge.className = 'card-badge success';
            statusIndicator.innerHTML = '<div class="status-dot"></div>Connected';
            statusIndicator.className = 'status-indicator success';
            button.innerHTML = '<i class="fas fa-stop"></i>Stop Sidecar';
            button.disabled = false;
            
            this.showNotification('Python Sidecar started successfully!', 'success');
        }, 2000);
    }

    refreshData() {
        this.showNotification('Refreshing data...', 'info');
        
        // Simulate data refresh
        setTimeout(() => {
            this.loadInitialData();
            this.showNotification('Data refreshed successfully!', 'success');
        }, 1000);
    }

    loadInitialData() {
        // Simulate loading initial application data
        console.log('Loading initial data...');
        
        // Update last updated time
        const now = new Date().toLocaleTimeString();
        const lastUpdatedElements = document.querySelectorAll('[data-last-updated]');
        lastUpdatedElements.forEach(el => {
            el.textContent = now;
        });
    }

    applyTheme() {
        document.documentElement.setAttribute('data-theme', this.theme);
        
        // Update theme toggle button icon
        const themeBtn = document.querySelector('.btn:has(.fa-moon)');
        if (themeBtn) {
            const icon = themeBtn.querySelector('i');
            icon.className = this.theme === 'light' ? 'fas fa-moon' : 'fas fa-sun';
        }
    }

    toggleTheme() {
        this.theme = this.theme === 'light' ? 'dark' : 'light';
        localStorage.setItem('rez-theme', this.theme);
        this.applyTheme();
        
        this.showNotification(`Switched to ${this.theme} theme`, 'info');
    }

    showNotification(message, type = 'info') {
        // Create notification element
        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 12px 16px;
            background: var(--bg-primary);
            border: 1px solid var(--border-primary);
            border-radius: var(--radius-md);
            box-shadow: var(--shadow-lg);
            z-index: 1000;
            max-width: 300px;
            font-size: 14px;
            color: var(--text-primary);
            transform: translateX(100%);
            transition: transform var(--transition-normal);
        `;
        
        // Add type-specific styling
        if (type === 'success') {
            notification.style.borderLeftColor = 'var(--success-500)';
            notification.style.borderLeftWidth = '4px';
        } else if (type === 'error') {
            notification.style.borderLeftColor = 'var(--error-500)';
            notification.style.borderLeftWidth = '4px';
        } else if (type === 'warning') {
            notification.style.borderLeftColor = 'var(--warning-500)';
            notification.style.borderLeftWidth = '4px';
        }
        
        notification.textContent = message;
        document.body.appendChild(notification);
        
        // Animate in
        setTimeout(() => {
            notification.style.transform = 'translateX(0)';
        }, 10);
        
        // Auto remove after 3 seconds
        setTimeout(() => {
            notification.style.transform = 'translateX(100%)';
            setTimeout(() => {
                document.body.removeChild(notification);
            }, 300);
        }, 3000);
    }
}

// Global theme toggle function
function toggleTheme() {
    if (window.rezGUI) {
        window.rezGUI.toggleTheme();
    }
}

// Initialize the application when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.rezGUI = new RezGUI();
});

// Handle responsive sidebar toggle (for mobile)
function toggleSidebar() {
    const sidebar = document.querySelector('.sidebar');
    sidebar.classList.toggle('open');
}

// Add mobile menu button if needed
if (window.innerWidth <= 768) {
    const header = document.querySelector('.main-header');
    const menuButton = document.createElement('button');
    menuButton.className = 'btn btn-secondary btn-sm';
    menuButton.innerHTML = '<i class="fas fa-bars"></i>';
    menuButton.onclick = toggleSidebar;
    header.insertBefore(menuButton, header.firstChild);
}
