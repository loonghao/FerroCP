import React, { useState, useEffect } from 'react';
import { Search, Home, Package, Layers, GitBranch, Settings, Play, RefreshCw, Plus, Moon, Sun } from 'lucide-react';

interface RezConfig {
  version: string;
  platform: string;
  pythonSidecar: {
    status: 'running' | 'stopped' | 'starting';
    port: number;
  };
}

interface NavigationItem {
  id: string;
  label: string;
  icon: React.ReactNode;
}

const RezGUI: React.FC = () => {
  const [currentPage, setCurrentPage] = useState('overview');
  const [theme, setTheme] = useState<'light' | 'dark'>('light');
  const [searchQuery, setSearchQuery] = useState('');
  const [config, setConfig] = useState<RezConfig>({
    version: '0.1.0',
    platform: 'win32',
    pythonSidecar: {
      status: 'stopped',
      port: 8080
    }
  });

  const navigationItems: NavigationItem[] = [
    { id: 'overview', label: 'Overview', icon: <Home size={20} /> },
    { id: 'packages', label: 'Package Browser', icon: <Package size={20} /> },
    { id: 'contexts', label: 'Context Manager', icon: <Layers size={20} /> },
    { id: 'dependencies', label: 'Dependency Graph', icon: <GitBranch size={20} /> },
    { id: 'settings', label: 'Settings', icon: <Settings size={20} /> },
  ];

  useEffect(() => {
    // Load theme from localStorage
    const savedTheme = localStorage.getItem('rez-theme') as 'light' | 'dark' || 'light';
    setTheme(savedTheme);
    document.documentElement.setAttribute('data-theme', savedTheme);
  }, []);

  const toggleTheme = () => {
    const newTheme = theme === 'light' ? 'dark' : 'light';
    setTheme(newTheme);
    localStorage.setItem('rez-theme', newTheme);
    document.documentElement.setAttribute('data-theme', newTheme);
  };

  const startPythonSidecar = async () => {
    setConfig(prev => ({
      ...prev,
      pythonSidecar: { ...prev.pythonSidecar, status: 'starting' }
    }));

    // Simulate API call
    setTimeout(() => {
      setConfig(prev => ({
        ...prev,
        pythonSidecar: { ...prev.pythonSidecar, status: 'running' }
      }));
    }, 2000);
  };

  const stopPythonSidecar = () => {
    setConfig(prev => ({
      ...prev,
      pythonSidecar: { ...prev.pythonSidecar, status: 'stopped' }
    }));
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running': return 'success';
      case 'starting': return 'warning';
      case 'stopped': return 'error';
      default: return 'error';
    }
  };

  const renderOverviewPage = () => (
    <div className="space-y-6">
      {/* Status Cards */}
      <div className="card-grid">
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Application</h3>
            <div className="card-badge">v{config.version}</div>
          </div>
          <div className="card-content">
            <p><strong>Platform:</strong> {config.platform}</p>
            <p><strong>Status:</strong> <span className="status-indicator success">
              <div className="status-dot"></div>Running
            </span></p>
            <p><strong>Last Updated:</strong> Just now</p>
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Python Sidecar</h3>
            <div className={`card-badge ${getStatusColor(config.pythonSidecar.status)}`}>
              {config.pythonSidecar.status}
            </div>
          </div>
          <div className="card-content">
            <p><strong>Status:</strong> <span className={`status-indicator ${getStatusColor(config.pythonSidecar.status)}`}>
              <div className="status-dot"></div>
              {config.pythonSidecar.status === 'running' ? 'Connected' : 
               config.pythonSidecar.status === 'starting' ? 'Starting...' : 'Not connected'}
            </span></p>
            <p><strong>Port:</strong> {config.pythonSidecar.port}</p>
            <button 
              className="btn btn-primary btn-sm mt-3"
              onClick={config.pythonSidecar.status === 'running' ? stopPythonSidecar : startPythonSidecar}
              disabled={config.pythonSidecar.status === 'starting'}
            >
              {config.pythonSidecar.status === 'running' ? (
                <>Stop Sidecar</>
              ) : (
                <><Play size={16} />Start Sidecar</>
              )}
            </button>
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <h3 className="card-title">Quick Actions</h3>
          </div>
          <div className="card-content">
            <div className="flex flex-col gap-3">
              <button className="btn btn-secondary" onClick={() => setCurrentPage('packages')}>
                <Package size={16} />Browse Packages
              </button>
              <button className="btn btn-secondary" onClick={() => setCurrentPage('contexts')}>
                <Layers size={16} />Create Context
              </button>
              <button className="btn btn-secondary" onClick={() => setCurrentPage('dependencies')}>
                <GitBranch size={16} />View Dependencies
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Test Connection */}
      <div className="card">
        <div className="card-header">
          <h3 className="card-title">Test Connection</h3>
        </div>
        <div className="card-content">
          <p className="mb-4 text-secondary">
            Test the connection between frontend and backend
          </p>
          <div className="flex gap-3 items-center">
            <input 
              type="text" 
              className="form-input flex-1" 
              placeholder="Enter a name..."
            />
            <button className="btn btn-primary">
              Greet
            </button>
          </div>
        </div>
      </div>

      {/* Feature Cards */}
      <div className="card-grid">
        <div className="card">
          <div className="card-header">
            <h3 className="card-title">
              <Package size={20} className="text-primary mr-2" />
              Package Browser
            </h3>
          </div>
          <div className="card-content">
            <p>Browse, search, and manage Rez packages with advanced filtering and sorting capabilities.</p>
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <h3 className="card-title">
              <Layers size={20} className="text-primary mr-2" />
              Context Manager
            </h3>
          </div>
          <div className="card-content">
            <p>Create, resolve, and manage Rez contexts with dependency tracking and environment configuration.</p>
          </div>
        </div>

        <div className="card">
          <div className="card-header">
            <h3 className="card-title">
              <GitBranch size={20} className="text-primary mr-2" />
              Dependency Graph
            </h3>
          </div>
          <div className="card-content">
            <p>Visualize package dependencies with interactive graphs and conflict resolution tools.</p>
          </div>
        </div>
      </div>
    </div>
  );

  const renderPage = () => {
    switch (currentPage) {
      case 'overview':
        return renderOverviewPage();
      case 'packages':
        return <div><h2>Package Browser</h2><p>Package browser content would go here...</p></div>;
      case 'contexts':
        return <div><h2>Context Manager</h2><p>Context manager content would go here...</p></div>;
      case 'dependencies':
        return <div><h2>Dependency Graph</h2><p>Dependency graph content would go here...</p></div>;
      case 'settings':
        return <div><h2>Settings</h2><p>Settings content would go here...</p></div>;
      default:
        return renderOverviewPage();
    }
  };

  const getPageTitle = () => {
    const titles = {
      overview: 'Welcome to Rez GUI',
      packages: 'Package Browser',
      contexts: 'Context Manager',
      dependencies: 'Dependency Graph',
      settings: 'Settings'
    };
    return titles[currentPage as keyof typeof titles] || 'Rez GUI';
  };

  return (
    <div className="app-container">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="logo">
            <div className="logo-icon">R</div>
            <div className="logo-text">
              <h1>Rez GUI</h1>
              <p>Package Management</p>
            </div>
          </div>
          
          <div className="search-box">
            <Search size={16} className="search-icon" />
            <input 
              type="text" 
              className="search-input" 
              placeholder="Search packages..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </div>
        
        <nav className="nav-section">
          <ul className="nav-list">
            {navigationItems.map((item) => (
              <li key={item.id} className="nav-item">
                <a 
                  href="#" 
                  className={`nav-link ${currentPage === item.id ? 'active' : ''}`}
                  onClick={(e) => {
                    e.preventDefault();
                    setCurrentPage(item.id);
                  }}
                >
                  <span className="nav-icon">{item.icon}</span>
                  <span>{item.label}</span>
                </a>
              </li>
            ))}
          </ul>
        </nav>
        
        <div className="sidebar-footer" style={{ padding: 'var(--space-4)', borderTop: '1px solid var(--border-primary)' }}>
          <div className="status-indicator success">
            <div className="status-dot"></div>
            <span>Connected</span>
          </div>
        </div>
      </aside>
      
      {/* Main Content */}
      <main className="main-content">
        <header className="main-header">
          <h1 className="page-title">{getPageTitle()}</h1>
          <div className="header-actions">
            <button className="btn btn-secondary btn-sm">
              <RefreshCw size={16} />
              Refresh
            </button>
            <button className="btn btn-primary btn-sm">
              <Plus size={16} />
              New Context
            </button>
            <button className="btn btn-secondary btn-sm" onClick={toggleTheme}>
              {theme === 'light' ? <Moon size={16} /> : <Sun size={16} />}
            </button>
          </div>
        </header>
        
        <div className="content-area">
          {renderPage()}
        </div>
      </main>
    </div>
  );
};

export default RezGUI;
