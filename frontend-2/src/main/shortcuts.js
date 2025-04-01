// main/shortcuts.js
const { globalShortcut, BrowserWindow } = require('electron');

function registerShortcuts(createWindowCallback) {
  // Register Command+E to show the application (changed from Command+/)
  const shortcutSuccess = globalShortcut.register('CommandOrControl+/', () => {
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

  if (!shortcutSuccess) {
    console.error('Failed to register Command+E shortcut');
  } else {
    console.log('Command+E shortcut registered successfully');
  }
}

function unregisterShortcuts() {
  // Unregister all shortcuts when the app quits
  globalShortcut.unregisterAll();
  console.log('All shortcuts unregistered');
}

module.exports = { registerShortcuts, unregisterShortcuts };
