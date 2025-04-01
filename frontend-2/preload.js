// preload.js
const { contextBridge, ipcRenderer } = require('electron');

// Expose protected methods that allow the renderer process to use
// the ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld(
  'api', {
    invoke: (channel, data) => {
      // Whitelist channels
      const validChannels = [
        'get-recent-files',
        'get-activity-feed',
        'search-files'
      ];
      if (validChannels.includes(channel)) {
        return ipcRenderer.invoke(channel, data);
      }
    }
  }
);
