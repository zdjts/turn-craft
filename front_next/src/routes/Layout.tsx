import { useState, useEffect } from 'react';
import { Outlet, Link, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../store/AuthContext';
import FeedbackLink from '../components/FeedbackLink';

export default function AppLayout() {
  const { username, logout } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const [theme, setTheme] = useState(() => localStorage.getItem('theme') || 'dark');

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
  }, [theme]);

  const isActive = (path: string) => location.pathname === path;

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  return (
    <div className="app-shell">
      <div className="sidebar g-card">
        <div className="sidebar-logo">
          <span className="logo-text">Turn Craft</span>
        </div>

        <div className="sidebar-menu">
          <Link
            to="/lobby"
            className={isActive('/lobby') ? 'menu-item is-active' : 'menu-item'}
          >
            <span className="menu-label">游戏大厅</span>
          </Link>
          <Link
            to="/public"
            className={isActive('/public') ? 'menu-item is-active' : 'menu-item'}
          >
            <span className="menu-label">公开房间</span>
          </Link>
          <Link
            to="/history"
            className={isActive('/history') ? 'menu-item is-active' : 'menu-item'}
          >
            <span className="menu-label">历史房间</span>
          </Link>
          <Link
            to="/profile"
            className={isActive('/profile') ? 'menu-item is-active' : 'menu-item'}
          >
            <span className="menu-label">个人主页</span>
          </Link>
          <Link
            to="/leaderboard"
            className={isActive('/leaderboard') ? 'menu-item is-active' : 'menu-item'}
          >
            <span className="menu-label">排行榜</span>
          </Link>
          <Link
            to="/about"
            className={isActive('/about') ? 'menu-item is-active' : 'menu-item'}
          >
            <span className="menu-label">关于项目</span>
          </Link>
        </div>

        <div className="sidebar-footer">
          <FeedbackLink />

          <button
            className="theme-toggle-btn"
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
          >
            {theme === 'dark' ? (
              <>
                <span className="toggle-icon">☀️</span>
                <span className="toggle-label">浅色模式</span>
              </>
            ) : (
              <>
                <span className="toggle-icon">🌙</span>
                <span className="toggle-label">深色模式</span>
              </>
            )}
          </button>

          <div className="user-profile-summary">
            <div className="user-name">{username ?? '未登录'}</div>
            <button className="logout-btn" onClick={handleLogout} title="退出登录">
              退出
            </button>
          </div>
        </div>
      </div>

      <div className="viewport">
        <div className="bg-glow bg-glow-1" />
        <div className="bg-glow bg-glow-2" />
        <div className="viewport-content">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
