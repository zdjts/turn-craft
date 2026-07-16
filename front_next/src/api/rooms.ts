import { del, get, post, put } from './client';

export interface CreateRoomRequest {
  game_type: string;
  max_round: number;
  my_slot: string;
  slots: string[];
  slot_configs: Record<string, string>;
  game_config?: unknown;
  is_public: boolean;
}

export interface CreateRoomResponse {
  status: string;
  room_id?: string;
  actor_id?: string;
  message?: string;
}

export interface RoomSnapshot {
  room_id: string;
  owner_id: string;
  game_type: string;
  engine_state: unknown;
  actor_slots: unknown;
  ai_configs: unknown;
  max_round: number;
  created_at: string;
  is_public: boolean;
}

export function createRoom(req: CreateRoomRequest): Promise<CreateRoomResponse> {
  return post<CreateRoomResponse>('/rooms', req);
}

export function getPublicRooms(): Promise<{ rooms: RoomSnapshot[] }> {
  return get<{ rooms: RoomSnapshot[] }>('/rooms/public');
}

export function getHistoryRooms(): Promise<{ rooms: RoomSnapshot[] }> {
  return get<{ rooms: RoomSnapshot[] }>('/rooms/history');
}

export function getRoom(roomId: string): Promise<{ room: RoomSnapshot }> {
  return get<{ room: RoomSnapshot }>(`/rooms/${roomId}`);
}

export function joinRoom(roomId: string, slotName: string): Promise<void> {
  return post<void>(`/rooms/${roomId}/join`, { slot_name: slotName });
}

export function deleteRoom(roomId: string): Promise<void> {
  return del<void>(`/rooms/${roomId}`);
}

export function setRoomPublic(roomId: string, isPublic: boolean): Promise<void> {
  return put<void>(`/rooms/${roomId}/public`, { is_public: isPublic });
}

export interface AiInsightAction {
  round: number;
  action: string;
  impact: string;
  reason?: string;
}

export interface AiInsight {
  actor_id: string;
  role: string;
  style: string;
  overall_assessment?: string;
  key_actions: AiInsightAction[];
  highlights: string[];
  mistakes: string[];
}

export function getAiInsights(roomId: string): Promise<{ insights: AiInsight[] }> {
  return get<{ insights: AiInsight[] }>(`/rooms/${roomId}/ai-insights`);
}
