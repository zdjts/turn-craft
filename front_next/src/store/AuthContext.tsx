import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import { getToken, setToken, removeToken } from '../api/client';
import * as authApi from '../api/auth';

interface AuthState {
  token: string | null;
  username: string | null;
}

interface AuthContextValue extends AuthState {
  isAuthenticated: boolean;
  login: (username: string, password: string) => Promise<void>;
  register: (username: string, password: string) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AuthState>(() => ({
    token: getToken(),
    username: localStorage.getItem('username'),
  }));

  const login = useCallback(async (username: string, password: string) => {
    const res = await authApi.login({ username, password });
    if (res.status !== 'success' || !res.token) {
      throw new Error(res.message || '登录失败');
    }
    setToken(res.token);
    localStorage.setItem('username', username);
    setState({ token: res.token, username });
  }, []);

  const register = useCallback(async (username: string, password: string) => {
    const res = await authApi.register({ username, password });
    if (res.status !== 'success' || !res.token) {
      throw new Error(res.message || '注册失败');
    }
    setToken(res.token);
    localStorage.setItem('username', username);
    setState({ token: res.token, username });
  }, []);

  const logout = useCallback(() => {
    removeToken();
    localStorage.removeItem('username');
    setState({ token: null, username: null });
  }, []);

  return (
    <AuthContext.Provider
      value={{
        ...state,
        isAuthenticated: state.token !== null,
        login,
        register,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
