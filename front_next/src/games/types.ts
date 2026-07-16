import type { ComponentType } from 'react';

export interface GameConfigProps {
  roleConfig: Record<string, string>;
  myRole: string;
  maxRound: number;
  gameConfig: unknown;
  isPublic: boolean;
  onChange: (props: Partial<GameConfigProps>) => void;
}

export interface RoomTemplate {
  name: string;
  desc: string;
  icon: string;
  roleConfig: Record<string, string>;
  myRole: string;
  maxRound: number;
  gameConfig: unknown;
}

export interface DefaultGameConfig {
  roleConfig: Record<string, string>;
  myRole: string;
  maxRound: number;
  gameConfig: unknown;
}

export type GameTier = 'main' | 'experimental';

export interface GameUIDefinition {
  gameType: string;
  name: string;
  icon: string;
  description: string;
  minPlayers: number;
  maxPlayers: number;
  tier: GameTier;
  LobbyCard: ComponentType<GameConfigProps>;
  defaultConfig: () => DefaultGameConfig;
  generateSlots: (configs: Record<string, string>) => string[];
  helpText: string[];
  templates: RoomTemplate[];
}
