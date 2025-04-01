// src/renderer/components/RecentlyAccessed/RecentlyAccessed.jsx
import React from 'react';
import './RecentlyAccessed.css';

const RecentlyAccessed = () => {
  // Sample data for recently accessed files
  const recentFiles = [
    { id: 1, title: 'Marketing Launch', type: 'presentation', icon: '🔴' },
    { id: 2, title: 'Client Brief', type: 'document', icon: '🔵' },
    { id: 3, title: 'Budget Overview', type: 'spreadsheet', icon: '🟢' },
    { id: 4, title: 'Project Specs', type: 'document', icon: '📝' }
  ];

  return (
    <div className="recently-accessed">
      <h2 className="section-title">Recently accessed</h2>
      <div className="file-grid">
        {recentFiles.map(file => (
          <div key={file.id} className="file-card">
            <div className="file-icon">{file.icon}</div>
            <div className="file-title">{file.title}</div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default RecentlyAccessed;