// src/renderer/utils/api.js
import { API_BASE_URL } from '../shared/constants';

// Use the Electron IPC for secure API calls
export const getRecentFiles = async () => {
  try {
    const response = await window.api.invoke('get-recent-files');
    return response;
  } catch (error) {
    console.error('Error fetching recent files:', error);
    return [];
  }
};

export const getActivityFeed = async () => {
  try {
    const response = await window.api.invoke('get-activity-feed');
    return response;
  } catch (error) {
    console.error('Error fetching activity feed:', error);
    return [];
  }
};
