import React from 'react';
import './styles/global.css';
import Navbar from './components/Navbar/Navbar';
import SearchBar from './components/SearchBar/SearchBar';
import RecentlyAccessed from './components/RecentlyAccessed/RecentlyAccessed';
import ActivityFeed from './components/ActivityFeed/ActivityFeed';

const App = () => {
  return (
    <div className="app-container" style={{ display: 'flex', height: '100vh' }}>
      <Navbar />
      <div className="content-area" style={{ flex: 1, padding: '20px', overflow: 'auto' }}>
        <SearchBar />
        <RecentlyAccessed />
        <ActivityFeed />
      </div>
    </div>
  );
};

export default App;
