import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../store/AuthContext';

export default function Login() {
  const { login, register, isAuthenticated } = useAuth();
  const navigate = useNavigate();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  if (isAuthenticated) {
    navigate('/', { replace: true });
    return null;
  }

  const handleSubmit = async (e: FormEvent, mode: 'login' | 'register') => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      if (mode === 'login') {
        await login(username, password);
      } else {
        await register(username, password);
      }
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : '操作失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-page">
      <div className="login-card g-card">
        <h1>Turn Craft</h1>
        <p className="login-subtitle">回合制博弈游戏平台</p>

        <form onSubmit={(e) => handleSubmit(e, 'login')}>
          <div className="g-field">
            <label>用户名</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="输入用户名"
              required
            />
          </div>

          <div className="g-field">
            <label>密码</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="输入密码"
              required
            />
          </div>

          {error && <div className="g-error">{error}</div>}

          <div className="login-actions">
            <button type="submit" className="g-btn g-btn-primary" disabled={loading}>
              {loading ? '处理中...' : '登录'}
            </button>
            <button
              type="button"
              className="g-btn g-btn-secondary"
              disabled={loading}
              onClick={(e) => handleSubmit(e, 'register')}
            >
              注册
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
