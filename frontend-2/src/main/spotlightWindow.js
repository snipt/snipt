// src/main/spotlightWindow.js
const { BrowserWindow, screen } = require('electron');
const path = require('path');

let spotlightWindow = null;

function createSpotlightWindow() {
  if (spotlightWindow) {
    return spotlightWindow;
  }

  const { width, height } = screen.getPrimaryDisplay().workAreaSize;

  spotlightWindow = new BrowserWindow({
    width: 600,
    height: 80,
    x: Math.round((width - 600) / 2),
    y: Math.round(height * 0.2),
    frame: false,
    transparent: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    show: false, // Start hidden
    resizable: false,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, '../../preload.js'),
    },
  });

  // If using your existing Webpack build, make sure the file path is correct:
  // You might need to load your compiled HTML or your dev server URL.
  spotlightWindow.loadURL(`file://${path.join(__dirname, '../../index.html')}?spotlight=true`);

  // Hide the window when it loses focus (optional).
  spotlightWindow.on('blur', () => {
    spotlightWindow.hide();
  });

  return spotlightWindow;
}

function getSpotlightWindow() {
  return spotlightWindow;
}

module.exports = {
  createSpotlightWindow,
  getSpotlightWindow,
};
