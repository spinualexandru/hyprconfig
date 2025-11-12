import React, { useEffect, useState } from 'react';

function DisplayPage() {
  const [monitors, setMonitors] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    loadMonitors();
  }, []);

  const loadMonitors = async () => {
    try {
      setLoading(true);
      setError(null);

      // Check if Tauri IPC is available
      if (typeof window.__TAURI_INTERNALS__ === 'undefined') {
        throw new Error('Tauri IPC is not available. Make sure you are running inside Tauri.');
      }

      console.log('Invoking get_monitors command...');

      // Use Tauri IPC to call the backend
      const data = await window.__TAURI_INTERNALS__.invoke('get_monitors');

      console.log('Monitors received:', data);

      setMonitors(data || []);
    } catch (err) {
      console.error('Failed to load monitors:', err);
      setError(err.message || 'Failed to load monitor information');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div>
        <h1>Display</h1>
        <p className="loading">Loading monitors...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div>
        <h1>Display</h1>
        <div className="error">Failed to load monitor information: {error}</div>
      </div>
    );
  }

  if (monitors.length === 0) {
    return (
      <div>
        <h1>Display</h1>
        <p>No monitors detected.</p>
      </div>
    );
  }

  return (
    <div>
      <h1>Display</h1>
      <div className="monitors-list">
        {monitors.map((monitor) => (
          <div key={monitor.id} className="monitor-card">
            <div className="monitor-header">
              <span style={{ fontSize: '2em' }}>🖥️</span>
              <div>
                <div className="monitor-title">{monitor.name}</div>
                <div className="monitor-resolution">
                  {monitor.width}x{monitor.height} @ {monitor.refresh_rate.toFixed(2)}Hz
                </div>
              </div>
            </div>
            <div className="monitor-details">
              <div className="detail-item">
                <span className="detail-label">Description</span>
                <span className="detail-value">{monitor.description}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Resolution</span>
                <span className="detail-value">{monitor.width}x{monitor.height}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Refresh Rate</span>
                <span className="detail-value">{monitor.refresh_rate.toFixed(2)} Hz</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Position</span>
                <span className="detail-value">{monitor.x}, {monitor.y}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Scale</span>
                <span className="detail-value">{monitor.scale}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Transform</span>
                <span className="detail-value">{monitor.transform}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Active Workspace</span>
                <span className="detail-value">
                  {monitor.active_workspace_name} (#{monitor.active_workspace_id})
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default DisplayPage;
