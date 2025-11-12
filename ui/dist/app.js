// Wait for DOM to be ready
document.addEventListener('DOMContentLoaded', () => {
    console.log('DOM loaded');
    console.log('Tauri available:', typeof window.__TAURI_INTERNALS__ !== 'undefined');

    initializeApp();
    loadMonitors();
});

function initializeApp() {
    // Navigation handling
    const navItems = document.querySelectorAll('.nav-item');
    const pages = document.querySelectorAll('.page');

    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const pageName = item.getAttribute('data-page');

            // Update active nav item
            navItems.forEach(nav => nav.classList.remove('active'));
            item.classList.add('active');

            // Show corresponding page
            pages.forEach(page => {
                page.style.display = 'none';
            });

            const targetPage = document.getElementById(`${pageName}-page`);
            if (targetPage) {
                targetPage.style.display = 'block';
            }

            // Load page-specific data
            if (pageName === 'display') {
                loadMonitors();
            }
        });
    });
}

// Load monitors data from backend
async function loadMonitors() {
    const container = document.getElementById('monitors-container');

    console.log('loadMonitors called');

    try {
        container.innerHTML = '<p class="loading">Loading monitors...</p>';

        // Check if Tauri IPC is available
        if (typeof window.__TAURI_INTERNALS__ === 'undefined') {
            throw new Error('Tauri IPC is not available. Make sure you are running inside Tauri.');
        }

        console.log('Invoking get_monitors command...');

        // Use Tauri IPC to call the backend
        const monitors = await window.__TAURI_INTERNALS__.invoke('get_monitors');

        console.log('Monitors received:', monitors);

        if (!monitors || monitors.length === 0) {
            container.innerHTML = '<p>No monitors detected.</p>';
            return;
        }

        const monitorsList = document.createElement('div');
        monitorsList.className = 'monitors-list';

        monitors.forEach(monitor => {
            const card = createMonitorCard(monitor);
            monitorsList.appendChild(card);
        });

        container.innerHTML = '';
        container.appendChild(monitorsList);
    } catch (error) {
        console.error('Failed to load monitors:', error);
        container.innerHTML = `<div class="error">Failed to load monitor information: ${error}</div>`;
    }
}

// Create a monitor card element
function createMonitorCard(monitor) {
    const card = document.createElement('div');
    card.className = 'monitor-card';

    card.innerHTML = `
        <div class="monitor-header">
            <span style="font-size: 2em;">🖥️</span>
            <div>
                <div class="monitor-title">${escapeHtml(monitor.name)}</div>
                <div class="monitor-resolution">${monitor.width}x${monitor.height} @ ${monitor.refresh_rate.toFixed(2)}Hz</div>
            </div>
        </div>
        <div class="monitor-details">
            <div class="detail-item">
                <span class="detail-label">Description</span>
                <span class="detail-value">${escapeHtml(monitor.description)}</span>
            </div>
            <div class="detail-item">
                <span class="detail-label">Resolution</span>
                <span class="detail-value">${monitor.width}x${monitor.height}</span>
            </div>
            <div class="detail-item">
                <span class="detail-label">Refresh Rate</span>
                <span class="detail-value">${monitor.refresh_rate.toFixed(2)} Hz</span>
            </div>
            <div class="detail-item">
                <span class="detail-label">Position</span>
                <span class="detail-value">${monitor.x}, ${monitor.y}</span>
            </div>
            <div class="detail-item">
                <span class="detail-label">Scale</span>
                <span class="detail-value">${monitor.scale}</span>
            </div>
            <div class="detail-item">
                <span class="detail-label">Transform</span>
                <span class="detail-value">${escapeHtml(monitor.transform)}</span>
            </div>
            <div class="detail-item">
                <span class="detail-label">Active Workspace</span>
                <span class="detail-value">${escapeHtml(monitor.active_workspace_name)} (#${monitor.active_workspace_id})</span>
            </div>
        </div>
    `;

    return card;
}

// Helper function to escape HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
