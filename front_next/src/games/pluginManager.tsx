import { lazy, Suspense, type ComponentType } from 'react';

export interface GamePluginProps {
  state: Record<string, unknown>;
  onAction: (action: unknown) => void;
  actorId: string;
  isMyTurn: boolean;
  streamingText: Record<string, string>;
}

const LincolnGameView = lazy(() => import('./lincoln/GameView'));
const TexasHoldemGameView = lazy(() => import('./texasHoldem/GameView'));
const WerewolfGameView = lazy(() => import('./werewolf/GameView'));
const BlackjackGameView = lazy(() => import('./blackjack/GameView'));

const GAME_VIEWS: Record<string, ComponentType<GamePluginProps>> = {
  lincoln: LincolnGameView,
  texas_holdem: TexasHoldemGameView,
  werewolf: WerewolfGameView,
  blackjack: BlackjackGameView,
};

interface PluginManagerProps extends GamePluginProps {
  gameType: string;
}

const FALLBACK = (
  <div className="loading-canvas g-card" style={{ minHeight: 200, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
    <span className="g-spinner" />
  </div>
);

export default function GamePluginManager({ gameType, ...props }: PluginManagerProps) {
  const View = GAME_VIEWS[gameType];
  if (!View) {
    return (
      <div className="unknown-game">
        <div className="unknown-game-icon">🎮</div>
        <h2>未知游戏类型</h2>
        <p>game_type: "{gameType}"</p>
      </div>
    );
  }
  return (
    <Suspense fallback={FALLBACK}>
      <View {...props} />
    </Suspense>
  );
}
