// src/renderer/pages/Home/Home.jsx
import React, { useState, useEffect } from 'react';
import './Home.css';
import SearchBar from '../../components/SearchBar/SearchBar';
import RecentlyAccessed from '../../components/RecentlyAccessed/RecentlyAccessed';
import ActivityFeed from '../../components/ActivityFeed/ActivityFeed';
import { getRecentFiles, getActivityFeed } from '../../utils/api';

const Home = () => {
  const [recentFiles, setRecentFiles] = useState([]);
  const [activities, setActivities] = useState([]);

  useEffect(() => {
    // Fetch data from backend API
    getRecentFiles().then(data => setRecentFiles(data));
    getActivityFeed().then(data => setActivities(data));
  }, []);

  const handleSearch = (query) => {
    console.log('Searching for:', query);
    // Implement search logic
  };

  return (
    <div className="home-page">
      <SearchBar onSearch={handleSearch} />

      <div className="content">
        <RecentlyAccessed files={recentFiles} />
        <ActivityFeed activities={activities} />
      </div>
    </div>
  );
};

export default Home;
