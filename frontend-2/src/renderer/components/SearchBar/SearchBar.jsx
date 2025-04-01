// src/renderer/components/SearchBar/SearchBar.jsx
import React from 'react';
import './SearchBar.css';

const SearchBar = () => {
  return (
    <div className="search-container">
      <div className="search-bar">
        <div className="search-icon">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
        </div>
        <input
          type="text"
          placeholder="Search beyond files"
          className="search-input"
        />
      </div>
      <div className="search-icons">
        <button className="icon-button">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="2" y="4" width="20" height="16" rx="2" />
            <path d="M7 15h0M12 15h0M17 15h0" />
          </svg>
        </button>
        <button className="icon-button">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
          </svg>
        </button>
        <button className="icon-button">
          <div className="profile-avatar-mini">+4</div>
        </button>
      </div>
    </div>
  );
};

export default SearchBar;
