import { lazy } from 'react';
import type { GameUIDefinition, RoomTemplate } from './types';

const LincolnLobbyCard = lazy(() => import('./lincoln/LobbyCard'));
const TexasHoldemLobbyCard = lazy(() => import('./texasHoldem/LobbyCard'));
const WerewolfLobbyCard = lazy(() => import('./werewolf/LobbyCard'));
const BlackjackLobbyCard = lazy(() => import('./blackjack/LobbyCard'));

function lincolnSlots(): string[] {
  return ['Judge', 'Pro', 'Con'];
}

function texasSlots(configs: Record<string, string>): string[] {
  return Array.from({ length: Object.keys(configs).length }, (_, i) => `player${i + 1}`);
}

function werewolfSlots(): string[] {
  return Array.from({ length: 7 }, (_, i) => `Player${i + 1}`);
}

const LINCOLN_TEMPLATES: RoomTemplate[] = [
  {
    name: '经典辩论', desc: '法官主办，AI 正反方辩论', icon: '🏛️',
    roleConfig: { Judge: 'human', Pro: 'ai', Con: 'ai' },
    myRole: 'Judge', maxRound: 8, gameConfig: null,
  },
  {
    name: '长辩论', desc: '16 轮深度辩论', icon: '📜',
    roleConfig: { Judge: 'human', Pro: 'ai', Con: 'ai' },
    myRole: 'Judge', maxRound: 16, gameConfig: null,
  },
];

const TEXAS_TEMPLATES: RoomTemplate[] = [
  {
    name: '标准德州', desc: '6 人桌，你 + 5 个 AI', icon: '🃏',
    roleConfig: (() => {
      const m: Record<string, string> = { player1: 'human' };
      for (let i = 2; i <= 6; i++) m[`player${i}`] = 'ai';
      return m;
    })(),
    myRole: 'player1', maxRound: 100,
    gameConfig: { small_blind: 10, big_blind: 20, starting_chips: 1000 },
  },
  {
    name: '快速德州', desc: '3 人对抗赛', icon: '⚡',
    roleConfig: (() => {
      const m: Record<string, string> = { player1: 'human' };
      for (let i = 2; i <= 3; i++) m[`player${i}`] = 'ai';
      return m;
    })(),
    myRole: 'player1', maxRound: 50,
    gameConfig: { small_blind: 5, big_blind: 10, starting_chips: 500 },
  },
];

const WEREWOLF_TEMPLATES: RoomTemplate[] = [
  {
    name: '7 人标准局', desc: '完美推演：2 狼 + 预言家 + 女巫 + 猎人 + 2 村民', icon: '🐺',
    roleConfig: (() => {
      const m: Record<string, string> = { Player1: 'human' };
      for (let i = 2; i <= 7; i++) m[`Player${i}`] = 'ai';
      return m;
    })(),
    myRole: 'Player1', maxRound: 50, gameConfig: null,
  },
];

function blackjackSlots(configs: Record<string, string>): string[] {
  return Object.keys(configs).sort();
}

const BLACKJACK_TEMPLATES: RoomTemplate[] = [
  {
    name: '单挑经典', desc: '你 vs 庄家，1 对 1 对决', icon: '🃏',
    roleConfig: { player1: 'human' },
    myRole: 'player1', maxRound: 1,
    gameConfig: { starting_chips: 1000, min_bet: 10, max_bet: 100 },
  },
  {
    name: '四人赌桌', desc: '4 人围桌，庄家通吃', icon: '🎲',
    roleConfig: { player1: 'human', player2: 'ai', player3: 'ai', player4: 'ai' },
    myRole: 'player1', maxRound: 1,
    gameConfig: { starting_chips: 500, min_bet: 5, max_bet: 50 },
  },
];

const GAMES: Record<string, GameUIDefinition> = {
  lincoln: {
    gameType: 'lincoln',
    name: '林肯辩论',
    icon: '🏛️',
    description: '经典英式辩论 · 法官裁判 · 正反方交锋',
    tier: 'main',
    minPlayers: 3,
    maxPlayers: 3,
    LobbyCard: LincolnLobbyCard,
    defaultConfig: () => ({
      roleConfig: { Judge: 'human', Pro: 'ai', Con: 'ai' },
      myRole: 'Judge',
      maxRound: 16,
      gameConfig: null,
    }),
    generateSlots: lincolnSlots,
    helpText: [
      '🎯 目标：通过辩论说服裁判。正方(Pro)支持辩题，反方(Con)反对辩题。',
      '👨‍⚖️ 法官(Judge)：开局给出辩题，最后裁决胜负。',
      '💬 发言顺序：法官开题 → 正方 → 反方 → 正方 → 反方 → 法官总结',
      '🤖 AI 玩家会自动发言。你发言后等待 AI 回应即可。',
    ],
    templates: LINCOLN_TEMPLATES,
  },
  texas_holdem: {
    gameType: 'texas_holdem',
    name: '德州扑克',
    icon: '🃏',
    description: '2-6 人经典德扑 · 盲注博弈 · 心理对抗',
    tier: 'main',
    minPlayers: 2,
    maxPlayers: 6,
    LobbyCard: TexasHoldemLobbyCard,
    defaultConfig: () => ({
      roleConfig: (() => {
        const m: Record<string, string> = { player1: 'human' };
        for (let i = 2; i <= 6; i++) m[`player${i}`] = 'ai';
        return m;
      })(),
      myRole: 'player1',
      maxRound: 100,
      gameConfig: { small_blind: 10, big_blind: 20, starting_chips: 1000 },
    }),
    generateSlots: texasSlots,
    helpText: [
      '🎯 目标：赢取所有筹码。通过下注、加注、弃牌等策略击败对手。',
      '🃏 每局开始每位玩家获得两张底牌，然后依次发公共牌。',
      '💰 下注轮次：Pre-Flop → Flop → Turn → River → 摊牌',
      '🤖 AI 玩家会自动行动。轮到你时，底牌会显示在界面中。',
    ],
    templates: TEXAS_TEMPLATES,
  },
  blackjack: {
    gameType: 'blackjack',
    name: '二十一点',
    icon: '🃏',
    description: '经典 Blackjack · 庄家对赌 · 策略博弈',
    tier: 'main',
    minPlayers: 1,
    maxPlayers: 6,
    LobbyCard: BlackjackLobbyCard,
    defaultConfig: () => ({
      roleConfig: { player1: 'human' },
      myRole: 'player1',
      maxRound: 1,
      gameConfig: { starting_chips: 1000, min_bet: 10, max_bet: 100 },
    }),
    generateSlots: blackjackSlots,
    helpText: [
      '🎯 目标：手牌点数接近 21 点但不超过，击败庄家。',
      '🃏 A 算 1 或 11 点，J/Q/K 算 10 点，其余按面值。',
      '👆 要牌(Hit)：再拿一张。✋ 停牌(Stand)：停止拿牌。💰 加倍(Double)：赌注翻倍，只拿一张。',
      '🤖 AI 玩家根据风格决策：保守型 12 点停，激进型 15 点还冲。',
    ],
    templates: BLACKJACK_TEMPLATES,
  },
  werewolf: {
    gameType: 'werewolf',
    name: '狼人杀',
    icon: '🐺',
    description: '7 人社交推理 · 狼人暗杀 · 好人投票',
    tier: 'experimental',
    minPlayers: 7,
    maxPlayers: 7,
    LobbyCard: WerewolfLobbyCard,
    defaultConfig: () => ({
      roleConfig: (() => {
        const m: Record<string, string> = { Player1: 'human' };
        for (let i = 2; i <= 7; i++) m[`Player${i}`] = 'ai';
        return m;
      })(),
      myRole: 'Player1',
      maxRound: 50,
      gameConfig: null,
    }),
    generateSlots: werewolfSlots,
    helpText: [
      '🎯 目标：狼人阵营 vs 好人阵营。狼人隐藏身份，好人找出狼人。',
      '🌙 夜晚阶段：狼人击杀、预言家查验、女巫救人/毒人。',
      '☀️ 白天阶段：存活玩家发言讨论，然后投票放逐。',
      '🤖 AI 玩家会自动行动。请关注私密消息查看你的身份和能力。',
    ],
    templates: WEREWOLF_TEMPLATES,
  },
};

export function getGameDef(gameType: string): GameUIDefinition | undefined {
  return GAMES[gameType];
}

export function getAllGames(): GameUIDefinition[] {
  return Object.values(GAMES).sort((a, b) => a.gameType.localeCompare(b.gameType));
}
