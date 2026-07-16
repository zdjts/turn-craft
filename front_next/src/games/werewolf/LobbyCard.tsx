import { useState, useEffect } from 'react';
import type { GameConfigProps } from '../types';

export default function WerewolfLobbyCard(props: GameConfigProps) {
  const { roleConfig, myRole, onChange } = props;
  const [spectatorMode, setSpectatorMode] = useState(false);

  useEffect(() => {
    if (!myRole) {
      const defaults: Record<string, string> = { Player1: 'human' };
      for (let i = 2; i <= 7; i++) defaults[`Player${i}`] = 'ai';
      onChange({ myRole: 'Player1', roleConfig: defaults });
    }
  }, []);

  const setSpectator = (spec: boolean) => {
    setSpectatorMode(spec);
    const modes: Record<string, string> = {};
    if (spec) {
      for (let i = 1; i <= 7; i++) modes[`Player${i}`] = 'ai';
      onChange({ myRole: 'spectator', roleConfig: modes });
    } else {
      modes.Player1 = 'human';
      for (let i = 2; i <= 7; i++) {
        modes[`Player${i}`] = roleConfig[`Player${i}`] ?? 'ai';
      }
      onChange({ myRole: 'Player1', roleConfig: modes });
    }
  };

  const toggleSeat = (slot: string) => {
    const modes = { ...roleConfig };
    if (modes[slot] === 'human') modes[slot] = 'ai';
    else modes[slot] = 'human';
    onChange({ roleConfig: modes });
  };

  return (
    <>
      <div className="g-field">
        <label>游戏模式</label>
        <div className="mode-toggle">
          <button
            className={!spectatorMode ? 'mode-btn selected' : 'mode-btn'}
            onClick={() => setSpectator(false)}
          >
            <div className="mode-icon">🎮</div>
            <div className="mode-label">亲自上阵</div>
            <div className="mode-desc">你可以设置多个座位为真人联机</div>
          </button>
          <button
            className={spectatorMode ? 'mode-btn selected' : 'mode-btn'}
            onClick={() => setSpectator(true)}
          >
            <div className="mode-icon">👀</div>
            <div className="mode-label">观战模式</div>
            <div className="mode-desc">观看全 AI 之间的对局</div>
          </button>
        </div>
      </div>

      {!spectatorMode && (
        <div className="g-field">
          <label>联机席位配置</label>
          <div className="seats-toggle-grid">
            {Array.from({ length: 6 }, (_, i) => i + 2).map((i) => {
              const slot = `Player${i}`;
              const isHuman = roleConfig[slot] === 'human';
              return (
                <button
                  key={slot}
                  className={isHuman ? 'seat-btn human' : 'seat-btn ai'}
                  onClick={() => toggleSeat(slot)}
                >
                  <div className="seat-icon">{isHuman ? '👤' : '🤖'}</div>
                  <div className="seat-label">Player {i}</div>
                  <div className="seat-status">{isHuman ? '开放联机' : 'AI 接管'}</div>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </>
  );
}
