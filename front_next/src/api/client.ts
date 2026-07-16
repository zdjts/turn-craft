const TOKEN_KEY = 'token';

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function removeToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

const BACKEND_ORIGIN = '';

async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const token = getToken();
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string>),
  };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const res = await fetch(`${BACKEND_ORIGIN}${path}`, {
    ...options,
    headers,
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(body || `HTTP ${res.status}`);
  }

  return res.json();
}

export function get<T>(path: string): Promise<T> {
  return request<T>(path, { method: 'GET' });
}

export function post<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method: 'POST',
    body: body ? JSON.stringify(body) : undefined,
  });
}

export function put<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method: 'PUT',
    body: body ? JSON.stringify(body) : undefined,
  });
}

export function del<T>(path: string): Promise<T> {
  return request<T>(path, { method: 'DELETE' });
}

export interface LeaderboardEntry {
  user_id: string;
  username: string;
  value: number;
}

export function getLeaderboard(type: 'games' | 'wins' | 'experienced', minGames?: number): Promise<{ entries: LeaderboardEntry[] }> {
  let path = `/leaderboard/${type}`;
  if (type === 'experienced' && minGames) path += `?min_games=${minGames}`;
  return get<{ entries: LeaderboardEntry[] }>(path);
}

export function getLeaderboardByGame(gameType: string): Promise<{ entries: LeaderboardEntry[] }> {
  return get<{ entries: LeaderboardEntry[] }>(`/leaderboard/by-game/${gameType}`);
}

export interface Achievement {
  id: string;
  name: string;
  description: string;
  unlocked: boolean;
}

export function getAchievements(): Promise<{ achievements: Achievement[] }> {
  return get<{ achievements: Achievement[] }>('/users/me/achievements');
}
