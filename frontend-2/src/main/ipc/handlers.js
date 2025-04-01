// src/main/ipc/handlers.js
const { ipcMain } = require('electron');
const { fetchRecentFiles, fetchActivityFeed, searchFiles } = require('../api/backendApi');

function setupIpcHandlers() {
  ipcMain.handle('get-recent-files', async () => {
    return await fetchRecentFiles();
  });

  ipcMain.handle('get-activity-feed', async () => {
    return await fetchActivityFeed();
  });

  ipcMain.handle('search-files', async (event, query) => {
    return await searchFiles(query);
  });
}

module.exports = { setupIpcHandlers };
