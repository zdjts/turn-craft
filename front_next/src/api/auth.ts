import { post } from './client';

export interface AuthResponse {
  status: string;
  token?: string;
  message?: string;
}

export interface AuthRequest {
  username: string;
  password: string;
}

export function register(req: AuthRequest): Promise<AuthResponse> {
  return post<AuthResponse>('/register', req);
}

export function login(req: AuthRequest): Promise<AuthResponse> {
  return post<AuthResponse>('/login', req);
}
