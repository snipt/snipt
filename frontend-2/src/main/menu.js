// main/shortcuts.js
const { globalShortcut, BrowserWindow } = require('electron');

function registerShortcuts(createWindowCallback) {
  // Register Command+/ to show the application
  const commandSlashSuccess = globalShortcut.register('CommandOrControl+/', () => {
    const windows = BrowserWindow.getAllWindows();

    if (windows.length > 0) {
      // If there are existing windows, focus the first one
      const win = windows[0];
      if (win.isMinimized()) win.restore();
      win.show();
      win.focus();
    } else {
      // No windows exist, create a new one
      createWindowCallback();
    }
  });

  if (!commandSlashSuccess) {
    console.error('Failed to register Command+/ shortcut');
  }
}

module.exports = { registerShortcuts };
