// src/renderer/components/ActivityFeed/ActivityFeed.jsx
import React from 'react';
import './ActivityFeed.css';

const ActivityFeed = () => {
  // Sample data for activity feed
  const activities = [
    {
      id: 1,
      user: { name: 'Maria', avatar: '👩‍💼' },
      action: 'commented on',
      target: 'Company Announcements',
      time: '2 hours ago',
      otherUsers: 6
    },
    {
      id: 2,
      user: { name: 'Jennifer', avatar: '👩‍💻' },
      action: 'modified',
      target: 'Bluebird Milestones',
      time: '3 hours ago',
      otherUsers: 0
    }
  ];

  return (
    <div className="activity-feed">
      <h2 className="section-title">Activity feed</h2>

      <div className="activities">
        {activities.map(activity => (
          <div key={activity.id} className="activity-item">
            <div className="activity-avatar">{activity.user.avatar}</div>
            <div className="activity-content">
              <div className="activity-header">
                <span className="user-name">{activity.user.name}</span>
                {activity.otherUsers > 0 && <span className="other-users">and {activity.otherUsers} others</span>}
                <span className="activity-action">{activity.action}</span>
                <span className="activity-target">{activity.target}</span>
              </div>

              <div className="activity-document">
                <div className="document-icon">📄</div>
                <div className="document-info">
                  <div className="document-title">{activity.target}</div>
                  <div className="document-time">Updated {activity.time}</div>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ActivityFeed;
