// AGI Testbed - Main JavaScript
document.addEventListener('DOMContentLoaded', function() {
    // Initialize dropdown menus
    initializeDropdowns();
    
    // Set up API utilities
    const api = setupApiUtils();
    
    // Set up system control buttons
    setupSystemControls(api);
    
    // Refresh system data periodically
    startDataRefresh(api);
    
    // Set up message modal if present
    setupMessageModal(api);
});

// Initialize dropdown menus
function initializeDropdowns() {
    const dropdowns = document.querySelectorAll('.dropdown');
    
    dropdowns.forEach(dropdown => {
        const link = dropdown.querySelector('a');
        const content = dropdown.querySelector('.dropdown-content');
        
        if (link && content) {
            // Make dropdown accessible via keyboard
            link.addEventListener('keydown', (e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    content.classList.toggle('show');
                }
            });
        }
    });
}

// Set up API utilities
function setupApiUtils() {
    return {
        // Make a GET request to an API endpoint
        get: async function(endpoint) {
            try {
                const response = await fetch(`/api/${endpoint}`);
                
                if (!response.ok) {
                    const errorData = await response.json();
                    throw new Error(errorData.error || 'API request failed');
                }
                
                return await response.json();
            } catch (error) {
                console.error(`API GET Error (${endpoint}):`, error);
                showNotification('error', `API request failed: ${error.message}`);
                throw error;
            }
        },
        
        // Make a POST request to an API endpoint
        post: async function(endpoint, data) {
            try {
                const response = await fetch(`/api/${endpoint}`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify(data)
                });
                
                if (!response.ok) {
                    const errorData = await response.json();
                    throw new Error(errorData.error || 'API request failed');
                }
                
                return await response.json();
            } catch (error) {
                console.error(`API POST Error (${endpoint}):`, error);
                showNotification('error', `API request failed: ${error.message}`);
                throw error;
            }
        }
    };
}

// Set up system control buttons
function setupSystemControls(api) {
    const startButton = document.getElementById('start-system');
    const stopButton = document.getElementById('stop-system');
    const refreshButton = document.getElementById('refresh-data');
    
    if (startButton) {
        startButton.addEventListener('click', async () => {
            try {
                startButton.disabled = true;
                startButton.textContent = 'Starting...';
                
                const result = await api.post('orchestrator/start', {});
                
                if (result.success) {
                    showNotification('success', 'System started successfully');
                    refreshSystemData(api);
                }
            } catch (error) {
                console.error('Failed to start system:', error);
            } finally {
                startButton.disabled = false;
                startButton.textContent = 'Start All Components';
            }
        });
    }
    
    if (stopButton) {
        stopButton.addEventListener('click', async () => {
            try {
                stopButton.disabled = true;
                stopButton.textContent = 'Stopping...';
                
                const result = await api.post('orchestrator/stop', {});
                
                if (result.success) {
                    showNotification('success', 'System stopped successfully');
                    refreshSystemData(api);
                }
            } catch (error) {
                console.error('Failed to stop system:', error);
            } finally {
                stopButton.disabled = false;
                stopButton.textContent = 'Stop All Components';
            }
        });
    }
    
    if (refreshButton) {
        refreshButton.addEventListener('click', () => {
            refreshSystemData(api);
            showNotification('info', 'Refreshing system data...');
        });
    }
}

// Start periodic data refresh
function startDataRefresh(api) {
    // Immediately refresh data once
    refreshSystemData(api);
    
    // Then set up interval for periodic refresh
    setInterval(() => {
        refreshSystemData(api);
    }, 10000); // Refresh every 10 seconds
}

// Refresh system data from the API
async function refreshSystemData(api) {
    try {
        // Fetch system status
        const statusData = await api.get('system/status');
        updateSystemStatus(statusData);
        
        // Fetch component metrics
        const metricsData = await api.get('system/metrics');
        updateMetrics(metricsData);
        
        // If on dashboard, update component information
        if (document.querySelector('.component-grid')) {
            updateComponentCards(api);
        }
        
        // If metrics charts are present, update them
        updateCharts(metricsData);
        
    } catch (error) {
        console.error('Error refreshing system data:', error);
    }
}

// Update system status display
function updateSystemStatus(data) {
    const statusElement = document.querySelector('.system-status .status-value');
    const activeComponentsElement = document.getElementById('active-components');
    
    if (statusElement) {
        statusElement.textContent = data.status;
        
        // Update the status class
        const statusClasses = ['status-Running', 'status-Paused', 'status-Error', 
                              'status-Initializing', 'status-ShuttingDown', 'status-unknown'];
        
        statusClasses.forEach(cls => {
            statusElement.classList.remove(cls);
        });
        
        statusElement.classList.add(`status-${data.status}`);
    }
    
    if (activeComponentsElement) {
        activeComponentsElement.textContent = data.active_components;
    }
}

// Update metrics display
function updateMetrics(data) {
    // This function will be expanded in dashboard.js to update charts and metrics displays
    console.log('Metrics updated:', data);
}

// Update component cards on the dashboard
async function updateComponentCards(api) {
    try {
        // Time Scaling Component
        const tsData = await api.get('time-scaling/status');
        updateComponentCard('time-scaling-component', tsData.status, {
            'ts-mode': tsData.mode || 'Standard',
            'ts-tasks': tsData.tasks_processed || '0',
            'ts-avg-time': `${Math.round(tsData.avg_processing_time || 0)} ms`
        });
        
        // Memory Management Component
        const memData = await api.get('memory/status');
        updateComponentCard('memory-component', memData.status, {
            'mem-stm-alloc': `${Math.round((memData.stm_allocation || 0.2) * 100)}%`,
            'mem-stm-util': `${Math.round((memData.stm_utilization || 0) * 100)}%`,
            'mem-ltm-items': memData.ltm_items || '0'
        });
        
        // Reasoning Component
        const reasonData = await api.get('reasoning/status');
        updateComponentCard('reasoning-component', reasonData.status, {
            'reason-nodes': reasonData.total_nodes || '0',
            'reason-inferences': reasonData.inferences || '0',
            'reason-confidence': `${Math.round((reasonData.avg_confidence || 0) * 100)}%`
        });
        
        // Operational State Component
        const opData = await api.get('operational/status');
        updateComponentCard('operational-component', opData.status, {
            'op-mode': opData.current_mode || 'Unknown',
            'op-goals': opData.active_goals || '0',
            'op-tasks': opData.running_tasks || '0'
        });
    } catch (error) {
        console.error('Error updating component cards:', error);
    }
}

// Update a specific component card
function updateComponentCard(cardId, status, metrics) {
    const card = document.getElementById(cardId);
    if (!card) return;
    
    // Update status
    const statusElement = card.querySelector('.component-status');
    if (statusElement) {
        statusElement.textContent = status;
        
        // Update status class
        const statusClasses = ['status-running', 'status-paused', 'status-error', 
                              'status-initializing', 'status-unknown'];
        
        statusClasses.forEach(cls => {
            statusElement.classList.remove(cls);
        });
        
        statusElement.classList.add(`status-${status.toLowerCase()}`);
    }
    
    // Update metrics
    for (const [id, value] of Object.entries(metrics)) {
        const element = document.getElementById(id);
        if (element) {
            element.textContent = value;
        }
    }
}

// Update charts if present
function updateCharts(metricsData) {
    // This will be implemented in dashboard.js for specific charts
}

// Set up the message modal
function setupMessageModal(api) {
    const modal = document.getElementById('message-modal');
    if (!modal) return;
    
    const openModalButton = document.getElementById('send-message');
    const closeButton = modal.querySelector('.close');
    const cancelButton = document.getElementById('cancel-message');
    const form = document.getElementById('message-form');
    
    // Open modal button
    if (openModalButton) {
        openModalButton.addEventListener('click', () => {
            modal.style.display = 'block';
        });
    }
    
    // Close button
    if (closeButton) {
        closeButton.addEventListener('click', () => {
            modal.style.display = 'none';
        });
    }
    
    // Cancel button
    if (cancelButton) {
        cancelButton.addEventListener('click', () => {
            modal.style.display = 'none';
        });
    }
    
    // Close when clicking outside the modal
    window.addEventListener('click', (event) => {
        if (event.target === modal) {
            modal.style.display = 'none';
        }
    });
    
    // Form submission
    if (form) {
        form.addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const targetComponent = document.getElementById('target-component').value;
            const messageType = document.getElementById('message-type').value;
            const messagePayload = document.getElementById('message-payload').value;
            const messagePriority = document.getElementById('message-priority').value;
            
            // Validate and parse payload
            let payload;
            try {
                payload = messagePayload ? JSON.parse(messagePayload) : {};
            } catch (error) {
                showNotification('error', 'Invalid JSON payload');
                return;
            }
            
            // Send message
            try {
                const result = await api.post('orchestrator/message', {
                    target: targetComponent || null,
                    message_type: messageType,
                    payload: payload,
                    priority: parseInt(messagePriority, 10)
                });
                
                if (result.success) {
                    showNotification('success', 'Message sent successfully');
                    modal.style.display = 'none';
                    
                    // Clear form
                    form.reset();
                }
            } catch (error) {
                console.error('Error sending message:', error);
            }
        });
    }
}

// Show a notification
function showNotification(type, message) {
    // Check if notification container exists, create if it doesn't
    let container = document.querySelector('.notification-container');
    
    if (!container) {
        container = document.createElement('div');
        container.className = 'notification-container';
        document.body.appendChild(container);
    }
    
    // Create notification element
    const notification = document.createElement('div');
    notification.className = `notification ${type}`;
    notification.textContent = message;
    
    // Add close button
    const closeButton = document.createElement('button');
    closeButton.className = 'notification-close';
    closeButton.innerHTML = '&times;';
    closeButton.addEventListener('click', () => {
        container.removeChild(notification);
    });
    
    notification.appendChild(closeButton);
    container.appendChild(notification);
    
    // Remove notification after 5 seconds
    setTimeout(() => {
        if (notification.parentNode === container) {
            container.removeChild(notification);
        }
    }, 5000);
}

// Format a date string
function formatDate(dateString) {
    if (!dateString) return 'N/A';
    
    const date = new Date(dateString);
    
    // Check if date is valid
    if (isNaN(date.getTime())) return 'Invalid Date';
    
    return date.toLocaleString();
}

// Helper function to get status class
function getStatusClass(status) {
    if (!status) return 'status-unknown';
    
    status = status.toLowerCase();
    
    if (status.includes('run') || status.includes('start')) {
        return 'status-running';
    } else if (status.includes('pause')) {
        return 'status-paused';
    } else if (status.includes('error') || status.includes('fail')) {
        return 'status-error';
    } else if (status.includes('init')) {
        return 'status-initializing';
    } else if (status.includes('shut')) {
        return 'status-shuttingdown';
    } else {
        return 'status-unknown';
    }
}

// Register Handlebars helpers if available
if (typeof Handlebars !== 'undefined') {
    Handlebars.registerHelper('eq', function(a, b) {
        return a === b;
    });
    
    Handlebars.registerHelper('format_date', function(date) {
        return formatDate(date);
    });
    
    Handlebars.registerHelper('system_status_class', function(status) {
        return getStatusClass(status);
    });
}