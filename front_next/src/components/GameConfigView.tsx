import { Suspense } from 'react';
import type { GameConfigProps as GProps, GameUIDefinition } from '../games/types';

interface Props {
  gameDef: GameUIDefinition;
  config: GProps;
  creating: boolean;
  onCreate: () => void;
  onBack: () => void;
}

export default function GameConfigView({ gameDef, config, creating, onCreate, onBack }: Props) {
  const LobbyCard = gameDef.LobbyCard;

  return (
    <div className="pg-lobby-config-panel g-card">
      <button className="pg-lobby-config-back" onClick={onBack}>
        ← 返回游戏列表
      </button>

      <h3>⚙️ 配置: {gameDef.name}</h3>

      {gameDef.tier === 'experimental' && (
        <div className="g-warning" style={{ marginBottom: 12, padding: '8px 12px', borderRadius: 6, background: 'rgba(255,200,0,0.1)', color: '#c90', fontSize: '0.85em' }}>
          ⚠️ 此游戏处于实验阶段，体验可能不稳定。
        </div>
      )}

      <div className="pg-lobby-config">
        <div className="g-field pg-lobby-inline">
          <label>公开房间</label>
          <input
            type="checkbox"
            className="g-toggle"
            checked={config.isPublic}
            onChange={(e) => config.onChange?.({ isPublic: e.target.checked })}
          />
          <span className="pg-lobby-hint">允许此房间展示在公开列表中</span>
        </div>

        <Suspense fallback={<div className="g-skeleton-list">{[1,2].map(i => <div key={i} className="g-skeleton-row" />)}</div>}>
          <LobbyCard
            roleConfig={config.roleConfig}
            myRole={config.myRole}
            maxRound={config.maxRound}
            gameConfig={config.gameConfig}
            isPublic={config.isPublic}
            onChange={(patch) => config.onChange?.(patch)}
          />
        </Suspense>

        <button
          className={creating ? 'pg-lobby-create is-loading' : 'pg-lobby-create'}
          onClick={onCreate}
          disabled={creating}
        >
          {creating ? <span className="g-spinner" /> : '🏟️ 创建房间并进入'}
        </button>
      </div>
    </div>
  );
}
