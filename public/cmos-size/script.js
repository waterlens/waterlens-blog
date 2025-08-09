// Sensor data definition
const sensorData = {
    // Phone sensors
    "1/3.2": { name: "1/3.2\"", width: 4.54, height: 3.42, area: 15.5, cropFactor: "1/22.5" },
    "1/3": { name: "1/3\"", width: 4.8, height: 3.6, area: 17.3, cropFactor: "1/21.3" },
    "1/2.8": { name: "1/2.8\"", width: 5.12, height: 3.84, area: 19.7, cropFactor: "1/20.0" },
    "1/2.7": { name: "1/2.7\"", width: 5.27, height: 3.96, area: 20.9, cropFactor: "1/19.4" },
    "1/2.6": { name: "1/2.6\"", width: 5.47, height: 4.10, area: 22.4, cropFactor: "1/18.7" },
    "1/2.55": { name: "1/2.55\"", width: 5.76, height: 4.29, area: 24.7, cropFactor: "1/17.7" },
    "1/2.4": { name: "1/2.4\"", width: 5.76, height: 4.29, area: 24.7, cropFactor: "1/17.7" },
    "1/2.3": { name: "1/2.3\"", width: 6.17, height: 4.55, area: 28.1, cropFactor: "1/16.5" },
    "1/2": { name: "1/2\"", width: 6.4, height: 4.8, area: 30.7, cropFactor: "1/15.8" },
    "1/1.9": { name: "1/1.9\"", width: 6.7, height: 5.0, area: 33.5, cropFactor: "1/15.1" },
    "1/1.8": { name: "1/1.8\"", width: 7.18, height: 5.32, area: 38.2, cropFactor: "1/14.1" },
    "1/1.7": { name: "1/1.7\"", width: 7.44, height: 5.58, area: 41.5, cropFactor: "1/13.0" },
    "1/1.6": { name: "1/1.6\"", width: 7.78, height: 5.85, area: 45.5, cropFactor: "1/12.4" },
    "1/1.5": { name: "1/1.5\"", width: 8.1, height: 6.08, area: 49.2, cropFactor: "1/11.9" },
    "1/1.4": { name: "1/1.4\"", width: 8.5, height: 6.4, area: 54.4, cropFactor: "1/11.3" },
    "1/1.33": { name: "1/1.33\"", width: 8.8, height: 6.6, area: 58.1, cropFactor: "1/10.9" },
    "1/1.28": { name: "1/1.28\"", width: 9.8, height: 7.3, area: 71.5, cropFactor: "1/10.3" },
    "1/1.2": { name: "1/1.2\"", width: 10.67, height: 8.0, area: 85.4, cropFactor: "1/9.4" },
    "1/1.1": { name: "1/1.1\"", width: 11.5, height: 8.6, area: 98.9, cropFactor: "1/8.7" },
    
    // Compact camera sensors
    "1": { name: "1\"", width: 13.2, height: 8.8, area: 116.2, cropFactor: "1/7.6" },
    "1.1": { name: "1.1\"", width: 14.5, height: 9.7, area: 140.7, cropFactor: "1/6.9" },
    "1.5": { name: "1.5\"", width: 18.7, height: 14.0, area: 261.8, cropFactor: "1/5.3" },
    
    // Mirrorless/DSLR sensors
    "4/3": { name: "4/3\"", width: 17.3, height: 13.0, area: 224.9, cropFactor: "1/5.8" },
    "APS-C Canon": { name: "APS-C Canon", width: 22.3, height: 14.9, area: 332.3, cropFactor: "1/4.5" },
    "APS-C Nikon": { name: "APS-C Nikon", width: 23.5, height: 15.6, area: 366.6, cropFactor: "1/4.3" },
    "APS-C Sony": { name: "APS-C Sony", width: 23.5, height: 15.6, area: 366.6, cropFactor: "1/4.3" },
    "APS-C Fuji": { name: "APS-C Fuji", width: 23.6, height: 15.6, area: 368.2, cropFactor: "1/4.3" },
    "APS-H": { name: "APS-H", width: 27.9, height: 18.6, area: 518.9, cropFactor: "1/3.6" },
    
    // Full frame sensors
    "Full-Frame": { name: "Full Frame", width: 36.0, height: 24.0, area: 864.0, cropFactor: "1/1" },
    "Full-Frame Canon": { name: "Full Frame Canon", width: 36.0, height: 24.0, area: 864.0, cropFactor: "1/1" },
    "Full-Frame Nikon": { name: "Full Frame Nikon", width: 35.9, height: 24.0, area: 861.6, cropFactor: "1/1" },
    "Full-Frame Sony": { name: "Full Frame Sony", width: 35.9, height: 24.0, area: 861.6, cropFactor: "1/1" },
    
    // Medium format sensors
    "Medium-Format 645": { name: "Medium Format 645", width: 43.8, height: 32.9, area: 1441.0, cropFactor: "1.67x" },
    "Medium-Format 6x6": { name: "Medium Format 6x6", width: 56.0, height: 56.0, area: 3136.0, cropFactor: "2.33x" },
    "Medium-Format 6x7": { name: "Medium Format 6x7", width: 70.0, height: 56.0, area: 3920.0, cropFactor: "2.92x" },
    "Large-Format 4x5": { name: "Large Format 4x5", width: 101.6, height: 127.0, area: 12903.2, cropFactor: "4.22x" },
    "Large-Format 8x10": { name: "Large Format 8x10", width: 203.2, height: 254.0, area: 51612.8, cropFactor: "8.44x" }
};

// Global variables
let selectedSensors = [];
let canvas, ctx;
let showGrid = false;
let showLabels = false;
let scale = 1;
let offsetX = 0;
let offsetY = 0;
let isDragging = false;
let lastX = 0;
let lastY = 0;
let screenPPI = 96; // Default PPI, will detect actual value later

// Color configuration
const colors = [
    '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4', '#FFEAA7',
    '#DDA0DD', '#98D8C8', '#F7DC6F', '#BB8FCE', '#85C1E9'
];

// Initialize
$(document).ready(function() {
    detectScreenPPI();
    initializeCanvas();
    setupEventListeners();
    updateComparisonList();
    drawCanvas();
    
    // Set initial text for toggle labels button
    $('#toggle-labels').text('Show Labels');
    
    // Listen for browser zoom changes
    window.addEventListener('resize', handleZoomChange);
    window.addEventListener('orientationchange', handleZoomChange);
});

// Handle browser zoom changes
function handleZoomChange() {
    // Delay execution to ensure browser completes zoom
    setTimeout(() => {
        detectScreenPPI();
        drawCanvas();
    }, 100);
}

// Detect screen PPI
function detectScreenPPI() {
    // Create test element to detect screen PPI
    const testDiv = document.createElement('div');
    testDiv.style.width = '1in';
    testDiv.style.height = '1in';
    testDiv.style.position = 'absolute';
    testDiv.style.left = '-100%';
    testDiv.style.top = '-100%';
    document.body.appendChild(testDiv);
    
    // Get pixels per inch on screen
    const pixelsPerInch = testDiv.offsetWidth;
    document.body.removeChild(testDiv);
    
    // Get browser zoom level
    const zoomLevel = window.devicePixelRatio;
    
    // Set PPI value (considering browser zoom) - zoom is inverted, need to divide by zoom level
    screenPPI = (pixelsPerInch / zoomLevel) || 96;
    
    console.log(`Detected screen PPI: ${screenPPI}, browser zoom: ${zoomLevel}`);
    
    // 更新显示信息
    updatePPIInfo();
}

// Update PPI information display
function updatePPIInfo() {
    const mmToPixels = screenPPI / 25.4;
    const zoomLevel = window.devicePixelRatio;
    const infoText = `Screen PPI: ${screenPPI} (1mm ≈ ${mmToPixels.toFixed(1)}px, zoom: ${zoomLevel.toFixed(2)}x)`;
    
    // Update PPI info display if element exists
    const ppiInfoElement = document.getElementById('ppi-info');
    if (ppiInfoElement) {
        ppiInfoElement.textContent = infoText;
    }
    
    // Also display in console
    console.log(infoText);
}

// Initialize canvas
function initializeCanvas() {
    canvas = document.getElementById('sensor-canvas');
    if (!canvas) {
        console.error('Canvas element not found: sensor-canvas');
        return;
    }
    
    ctx = canvas.getContext('2d');
    if (!ctx) {
        console.error('Cannot get 2D context from canvas');
        return;
    }
    
    // Set canvas style
    canvas.style.cursor = 'grab';
    
    // Set high DPI support
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    
    // If canvas has no size yet, use default size
    const width = rect.width || 800;
    const height = rect.height || 600;
    
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    
    canvas.style.width = width + 'px';
    canvas.style.height = height + 'px';
}

// Setup event listeners
function setupEventListeners() {
    // Sensor button clicks
    $('.sensor-btn').click(function() {
        const sensorKey = $(this).data('sensor');
        toggleSensor(sensorKey);
        $(this).toggleClass('selected');
    });
    
    // Add custom sensor
    $('#add-custom').click(addCustomSensor);
    
    // Canvas controls
    $('#reset-view').click(resetView);
    $('#toggle-labels').click(toggleLabels);
    
    // Remove button event delegation
    $(document).on('click', '.remove-btn', function() {
        const index = parseInt($(this).data('index'));
        if (index >= 0 && index < selectedSensors.length) {
            const sensorKey = selectedSensors[index];
            selectedSensors.splice(index, 1);
            $(`.sensor-btn[data-sensor="${sensorKey}"]`).removeClass('selected');
            updateSensorInfo();
            updateComparisonList();
            drawCanvas();
        }
    });
    
    // Mouse events
    canvas.addEventListener('mousedown', startDrag);
    canvas.addEventListener('mousemove', drag);
    canvas.addEventListener('mouseup', endDrag);
    canvas.addEventListener('wheel', handleWheel);
    
    // Touch events
    canvas.addEventListener('touchstart', handleTouchStart);
    canvas.addEventListener('touchmove', handleTouchMove);
    canvas.addEventListener('touchend', handleTouchEnd);
}

// Toggle sensor selection
function toggleSensor(sensorKey) {
    const index = selectedSensors.indexOf(sensorKey);
    if (index > -1) {
        selectedSensors.splice(index, 1);
    } else {
        selectedSensors.push(sensorKey);
    }
    
    // Auto-adjust scale for small sensors
    if (selectedSensors.length > 0) {
        const smallestSensor = selectedSensors.reduce((smallest, key) => {
            const sensor = sensorData[key];
            return sensor && sensor.area < smallest.area ? sensor : smallest;
        }, { area: Infinity });
        
        if (smallestSensor.area < 50) { // Small sensors (phone sensors typically < 50mm²)
            // Auto-zoom to make small sensors more visible
            const mmToPixels = screenPPI / 25.4;
            const canvasWidth = canvas.width / window.devicePixelRatio;
            const canvasHeight = canvas.height / window.devicePixelRatio;
            
            // Calculate scale to make smallest sensor at least 50px wide
            const targetWidth = 50;
            const autoScale = targetWidth / (smallestSensor.width * mmToPixels);
            
            // Apply auto-scale if it's reasonable (between 0.5 and 10)
            if (autoScale >= 0.5 && autoScale <= 10) {
                scale = autoScale;
                offsetX = 0;
                offsetY = 0;
            }
        }
    }
    
    updateSensorInfo();
    updateComparisonList();
    drawCanvas();
}

// Add custom sensor
function addCustomSensor() {
    const width = parseFloat($('#custom-width').val());
    const height = parseFloat($('#custom-height').val());
    
    if (isNaN(width) || isNaN(height) || width <= 0 || height <= 0) {
        alert('Please enter valid width and height values');
        return;
    }
    
    const area = width * height;
    const cropFactor = calculateCropFactor(area);
    const customKey = `custom_${Date.now()}`;
    
    sensorData[customKey] = {
        name: `Custom (${width}×${height})`,
        width: width,
        height: height,
        area: area,
        cropFactor: cropFactor
    };
    
    selectedSensors.push(customKey);
    updateSensorInfo();
    updateComparisonList();
    drawCanvas();
    
    // Clear input fields
    $('#custom-width').val('');
    $('#custom-height').val('');
}

// Calculate crop factor
function calculateCropFactor(area) {
    const fullFrameArea = 864.0; // Full frame area
    const ratio = fullFrameArea / area;
    
    if (ratio > 1) {
        return `1/${ratio.toFixed(1)}`;
    } else {
        return `${(1/ratio).toFixed(2)}x`;
    }
}

// Update sensor information display
function updateSensorInfo() {
    if (selectedSensors.length === 0) {
        $('#current-sensor').text('None Selected');
        $('#sensor-size').text('-');
        $('#sensor-area').text('-');
        $('#crop-factor').text('-');
        return;
    }
    
    const lastSensor = sensorData[selectedSensors[selectedSensors.length - 1]];
    if (!lastSensor) {
        console.error('Sensor information not found:', selectedSensors[selectedSensors.length - 1]);
        $('#current-sensor').text('Data Error');
        $('#sensor-size').text('-');
        $('#sensor-area').text('-');
        $('#crop-factor').text('-');
        return;
    }
    $('#current-sensor').text(lastSensor.name);
    $('#sensor-size').text(`${lastSensor.width} × ${lastSensor.height} mm`);
    $('#sensor-area').text(`${lastSensor.area.toFixed(1)} mm²`);
    $('#crop-factor').text(lastSensor.cropFactor);
}

// Update sensor table


// Update comparison list
function updateComparisonList() {
    const container = $('#comparison-list');
    
    if (selectedSensors.length === 0) {
        container.html('<p>Select sensors to start comparison</p>');
        return;
    }
    
    let html = '';
    selectedSensors.forEach((sensorKey, index) => {
        const sensor = sensorData[sensorKey];
        if (!sensor) {
            console.error('Sensor not found when updating comparison list:', sensorKey);
            return;
        }
        const color = colors[index % colors.length];
        html += `
            <div style="color: ${color}; margin-bottom: 8px; display: flex; align-items: center; justify-content: space-between;">
                <span><strong>${sensor.name}</strong>: ${sensor.width}×${sensor.height}mm (${sensor.area.toFixed(1)}mm²)</span>
                <button class="remove-btn" data-index="${index}" style="background: transparent; color: #dc3545; border: none; padding: 2px 6px; font-size: 14px; font-weight: bold; cursor: pointer; margin-left: 8px;">×</button>
            </div>
        `;
    });
    
    container.html(html);
}

// Draw canvas
function drawCanvas() {
    const width = canvas.width / window.devicePixelRatio;
    const height = canvas.height / window.devicePixelRatio;
    
    // Clear canvas
    ctx.clearRect(0, 0, width, height);
    
    // Draw background
    ctx.fillStyle = '#f8f9fa';
    ctx.fillRect(0, 0, width, height);
    
    // Draw sensors - all sensors centered
    selectedSensors.forEach((sensorKey, index) => {
        const sensor = sensorData[sensorKey];
        if (!sensor) {
            console.error('Sensor data not found:', sensorKey);
            return;
        }
        const color = colors[index % colors.length];
        drawSensor(sensor, color, index);
    });
    
    // Draw scale bar
    drawScaleBar(width, height);
}



// Draw sensor
function drawSensor(sensor, color, index) {
    const canvasWidth = canvas.width / window.devicePixelRatio;
    const canvasHeight = canvas.height / window.devicePixelRatio;
    
    // Calculate sensor size on canvas - using PPI-based real size
    // 1 inch = 25.4 mm, so 1 mm = screenPPI/25.4 pixels
    const mmToPixels = screenPPI / 25.4;
    const baseScale = mmToPixels * scale; // PPI-based scale
    let displayWidth = sensor.width * baseScale;
    let displayHeight = sensor.height * baseScale;
    
    // Ensure minimum display size for very small sensors
    const minDisplaySize = 8; // Minimum 8 pixels for visibility
    if (displayWidth < minDisplaySize && displayHeight < minDisplaySize) {
        const scaleFactor = minDisplaySize / Math.min(displayWidth, displayHeight);
        displayWidth *= scaleFactor;
        displayHeight *= scaleFactor;
    }
    
    // Calculate sensor position - simple center display, let user drag and zoom to view
    const x = (canvasWidth / 2) + offsetX - displayWidth / 2;
    const y = (canvasHeight / 2) + offsetY - displayHeight / 2;
    
    // Draw sensor rectangle
    ctx.fillStyle = color + '40'; // Semi-transparent
    ctx.fillRect(x, y, displayWidth, displayHeight);
    
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.strokeRect(x, y, displayWidth, displayHeight);
    
    // Draw labels
    if (showLabels) {
        // Fixed font size, not affected by zoom
        const fontSize = 12;
        ctx.fillStyle = color;
        ctx.font = `${fontSize}px Oxygen`;
        ctx.textAlign = 'center';
        
        const labelX = x + displayWidth / 2;
        const labelY = y - 10;
        
        // Draw text (no background)
        ctx.fillStyle = color;
        ctx.fillText(sensor.name, labelX, labelY);
        
        // Draw size information (no background)
        const sizeText = `${sensor.width}×${sensor.height}mm`;
        const sizeY = y + displayHeight + 15;
        
        ctx.fillStyle = color;
        ctx.fillText(sizeText, labelX, sizeY);
    }
}

// Draw scale bar
function drawScaleBar(canvasWidth, canvasHeight) {
    // Scale bar position (bottom right, fixed position)
    const margin = 30;
    const barHeight = 20;
    
    // Use same calculation as sensors
    const mmToPixels = screenPPI / 25.4;
    const baseScale = mmToPixels * scale; // Same scale as sensors
    
    // Choose appropriate scale bar length - reference sensor size
    let barLengthMm;
    if (scale >= 2) {
        barLengthMm = 5; // Show 5mm
    } else if (scale >= 1) {
        barLengthMm = 10; // Show 10mm
    } else if (scale >= 0.5) {
        barLengthMm = 25; // Show 25mm
    } else {
        barLengthMm = 50; // Show 50mm
    }
    
    // Calculate scale bar pixel width - use same calculation as sensors
    const barWidth = barLengthMm * baseScale;
    
    // Scale bar position (bottom right, fixed position)
    const x = canvasWidth - margin - barWidth;
    const y = canvasHeight - margin - barHeight;
    
    // Draw scale bar background (transparent) - compress height to avoid overlapping with text
    ctx.fillStyle = 'rgba(255, 255, 255, 0.3)';
    ctx.fillRect(x - 8, y - 4, barWidth + 16, barHeight + 8);
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.5)';
    ctx.lineWidth = 1;
    ctx.strokeRect(x - 8, y - 4, barWidth + 16, barHeight + 8);
    
    // Draw scale bar line
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(x, y + barHeight / 2);
    ctx.lineTo(x + barWidth, y + barHeight / 2);
    ctx.stroke();
    

    
    // Draw length annotation
    ctx.fillStyle = '#333';
    ctx.font = 'bold 14px Oxygen';
    ctx.textAlign = 'center';
    ctx.fillText(`${barLengthMm}mm`, x + barWidth / 2, y + barHeight + 18);
    
    // Draw scale information
    const scaleText = `1:${(1 / scale).toFixed(1)}`;
    ctx.font = '12px Oxygen';
    ctx.fillText(scaleText, x + barWidth / 2, y - 8);
}



// Reset view
function resetView() {
    // Reset zoom and offset
    scale = 1;
    offsetX = 0;
    offsetY = 0;
    
    // Force redraw canvas
    drawCanvas();
}



// Toggle label display
function toggleLabels() {
    showLabels = !showLabels;
    $('#toggle-labels').text(showLabels ? 'Hide Labels' : 'Show Labels');
    drawCanvas();
}

// Mouse drag event
function startDrag(e) {
    isDragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    canvas.style.cursor = 'grabbing';
}

function drag(e) {
    if (!isDragging) return;
    
    const deltaX = e.clientX - lastX;
    const deltaY = e.clientY - lastY;
    
    // Calculate new offset
    const newOffsetX = offsetX + deltaX;
    const newOffsetY = offsetY + deltaY;
    
    // Get canvas size
    const canvasWidth = canvas.width / window.devicePixelRatio;
    const canvasHeight = canvas.height / window.devicePixelRatio;
    
    // Calculate actual display size of sensors on canvas
    let maxSensorWidth = 0;
    let maxSensorHeight = 0;
    
    if (selectedSensors.length > 0) {
        const mmToPixels = screenPPI / 25.4;
        const baseScale = mmToPixels * scale;
        
        selectedSensors.forEach(sensorKey => {
            const sensor = sensorData[sensorKey];
            if (sensor) {
                const displayWidth = sensor.width * baseScale;
                const displayHeight = sensor.height * baseScale;
                maxSensorWidth = Math.max(maxSensorWidth, displayWidth);
                maxSensorHeight = Math.max(maxSensorHeight, displayHeight);
            }
        });
    }
    
    // Calculate drag range limit, ensure sensors don't move completely out of canvas
    const maxOffsetX = Math.max(0, (canvasWidth - maxSensorWidth) / 2);
    const maxOffsetY = Math.max(0, (canvasHeight - maxSensorHeight) / 2);
    const minOffsetX = -Math.max(0, (canvasWidth - maxSensorWidth) / 2);
    const minOffsetY = -Math.max(0, (canvasHeight - maxSensorHeight) / 2);
    
    // Apply range limit
    offsetX = Math.max(minOffsetX, Math.min(maxOffsetX, newOffsetX));
    offsetY = Math.max(minOffsetY, Math.min(maxOffsetY, newOffsetY));
    
    lastX = e.clientX;
    lastY = e.clientY;
    
    drawCanvas();
}

function endDrag() {
    isDragging = false;
    canvas.style.cursor = 'grab';
}

// Mouse wheel zoom - centered on mouse position
function handleWheel(e) {
    e.preventDefault();
    
    const zoomFactor = 0.1;
    const delta = e.deltaY > 0 ? -zoomFactor : zoomFactor;
    
    // Get mouse position on canvas
    const rect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;
    
    // Calculate new scale - increased max zoom for small sensors
    const newScale = Math.max(0.1, Math.min(20, scale + delta));
    const scaleRatio = newScale / scale;
    
    // Zoom centered on mouse position
    // Calculate mouse position offset relative to canvas center
    const canvasCenterX = rect.width / 2;
    const canvasCenterY = rect.height / 2;
    
    // Calculate mouse offset relative to center
    const mouseOffsetX = mouseX - canvasCenterX;
    const mouseOffsetY = mouseY - canvasCenterY;
    
    // Update offset, so zoom is centered on mouse position
    const newOffsetX = offsetX - mouseOffsetX * (scaleRatio - 1);
    const newOffsetY = offsetY - mouseOffsetY * (scaleRatio - 1);
    
    // Get canvas size
    const canvasWidth = canvas.width / window.devicePixelRatio;
    const canvasHeight = canvas.height / window.devicePixelRatio;
    
    // Calculate actual display size of sensors at new scale
    let maxSensorWidth = 0;
    let maxSensorHeight = 0;
    
    if (selectedSensors.length > 0) {
        const mmToPixels = screenPPI / 25.4;
        const newBaseScale = mmToPixels * newScale;
        
        selectedSensors.forEach(sensorKey => {
            const sensor = sensorData[sensorKey];
            if (sensor) {
                const displayWidth = sensor.width * newBaseScale;
                const displayHeight = sensor.height * newBaseScale;
                maxSensorWidth = Math.max(maxSensorWidth, displayWidth);
                maxSensorHeight = Math.max(maxSensorHeight, displayHeight);
            }
        });
    }
    
    // Calculate drag range limit, ensure sensors don't move completely out of canvas
    const maxOffsetX = Math.max(0, (canvasWidth - maxSensorWidth) / 2);
    const maxOffsetY = Math.max(0, (canvasHeight - maxSensorHeight) / 2);
    const minOffsetX = -Math.max(0, (canvasWidth - maxSensorWidth) / 2);
    const minOffsetY = -Math.max(0, (canvasHeight - maxSensorHeight) / 2);
    
    // Apply range limit
    offsetX = Math.max(minOffsetX, Math.min(maxOffsetX, newOffsetX));
    offsetY = Math.max(minOffsetY, Math.min(maxOffsetY, newOffsetY));
    
    scale = newScale;
    drawCanvas();
}

// Touch event handling
function handleTouchStart(e) {
    e.preventDefault();
    if (e.touches.length === 1) {
        const touch = e.touches[0];
        startDrag({
            clientX: touch.clientX,
            clientY: touch.clientY
        });
    }
}

function handleTouchMove(e) {
    e.preventDefault();
    if (e.touches.length === 1) {
        const touch = e.touches[0];
        drag({
            clientX: touch.clientX,
            clientY: touch.clientY
        });
    }
}

function handleTouchEnd(e) {
    e.preventDefault();
    endDrag();
}

// When window size changes, re-adjust canvas
$(window).resize(function() {
    initializeCanvas();
    drawCanvas();
});
