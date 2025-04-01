const { app, BrowserWindow } = require('electron');
const path = require('path');
const { registerShortcuts, unregisterShortcuts } = require('./src/main/shortcuts'); // Update path if needed

let mainWindow;

function createWindow() {
  console.log('Creating main window...');

  mainWindow = new BrowserWindow({
    width: 1024,
    height: 768,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }, 
    frame: false
  });

  mainWindow.loadFile(path.join(__dirname, 'index.html'));

  // Open DevTools during development
  // mainWindow.webContents.openDevTools();

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// Wait until the app is ready
app.whenReady().then(() => {
  createWindow();

  // Register shortcuts AFTER the app is ready
  registerShortcuts(createWindow);

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

// Unregister shortcuts before quitting
app.on('will-quit', () => {
  unregisterShortcuts();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
