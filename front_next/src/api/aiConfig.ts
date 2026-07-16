import { get, put } from './client';

export interface AiConfigData {
  api_key: string;
  base_url: string;
  model: string;
  max_tokens: number;
  prompt: string;
  style: string;
}

export function getAiConfigs(roomId: string): Promise<{ configs: Record<string, AiConfigData> }> {
  return get<{ configs: Record<string, AiConfigData> }>(`/rooms/${roomId}/ai-config`);
}

export function updateAiConfig(roomId: string, actorId: string, config: Partial<AiConfigData>): Promise<void> {
  return put<void>(`/rooms/${roomId}/ai-config/${actorId}`, config);
}
