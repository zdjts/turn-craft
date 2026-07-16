import { useState, useEffect } from 'react';
import type { GameConfigProps } from '../types';

const PLAYER_COUNTS = [2, 3, 4, 5, 6];

export default function TexasHoldemLobbyCard(props: GameConfigProps) {
  const { roleConfig, myRole, onChange } = props;

  const [smallBlind, setSmallBlind] = useState('10');
  const [bigBlind, setBigBlind] = useState('20');
  const [startingChips, setStartingChips] = useState('1000');
  const [playerCount, setPlayerCountState] = useState(6);
  const [spectatorMode, setSpectatorMode] = useState(false);

  useEffect(() => {
    if (!myRole || (!myRole.startsWith('player') && myRole !== 'spectator')) {
      const sb = parseInt(smallBlind) || 10;
      const bb = parseInt(bigBlind) || 20;
      const sc = parseInt(startingChips) || 1000;
      const count = playerCount;
      const modes: Record<string, string> = { player1: 'human' };
      for (let i = 2; i <= count; i++) modes[`player${i}`] = 'ai';
      onChange({
        myRole: 'player1',
        roleConfig: modes,
        maxRound: 100,
        gameConfig: { small_blind: sb, big_blind: bb, starting_chips: sc },
      });
    }
  }, []);

  const updateGameConfig = (sb: string, bb: string, sc: string) => {
    onChange({
      gameConfig: {
        small_blind: parseInt(sb) || 10,
        big_blind: parseInt(bb) || 20,
        starting_chips: parseInt(sc) || 1000,
      },
    });
  };

  const setPlayerCount = (count: number) => {
    setPlayerCountState(count);
    const modes: Record<string, string> = {};
    const isSpec = spectatorMode;
    if (isSpec) {
      for (let i = 1; i <= count; i++) modes[`player${i}`] = 'ai';
      onChange({ myRole: 'spectator', roleConfig: modes });
    } else {
      modes.player1 = 'human';
      for (let i = 2; i <= count; i++) modes[`player${i}`] = 'ai';
      onChange({ myRole: 'player1', roleConfig: modes });
    }
  };

  const setSpectator = (spec: boolean) => {
    setSpectatorMode(spec);
    const count = playerCount;
    const modes: Record<string, string> = {};
    if (spec) {
      for (let i = 1; i <= count; i++) modes[`player${i}`] = 'ai';
      onChange({ myRole: 'spectator', roleConfig: modes });
    } else {
      modes.player1 = 'human';
      for (let i = 2; i <= count; i++) {
        modes[`player${i}`] = roleConfig[`player${i}`] ?? 'ai';
      }
      onChange({ myRole: 'player1', roleConfig: modes });
    }
  };

  const toggleSeat = (slot: string) => {
    const modes = { ...roleConfig };
    if (modes[slot] === 'human') {
      modes[slot] = 'ai';
    } else {
      modes[slot] = 'human';
    }
    onChange({ roleConfig: modes });
  };

  return (
    <>
      <div className="g-field">
        <label>游戏人数</label>
        <div className="player-count-grid">
          {PLAYER_COUNTS.map((c) => (
            <button
              key={c}
              className={playerCount === c ? 'count-btn selected' : 'count-btn'}
              onClick={() => setPlayerCount(c)}
            >
              {c} 人
            </button>
          ))}
        </div>
      </div>

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
            {Array.from({ length: playerCount - 1 }, (_, i) => i + 2).map((i) => {
              const slot = `player${i}`;
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

      <div className="g-field">
        <label>德州扑克配置</label>
        <div className="texas-config">
          <div className="config-field">
            <label>小盲注</label>
            <input
              type="number"
              value={smallBlind}
              onChange={(e) => {
                setSmallBlind(e.target.value);
                updateGameConfig(e.target.value, bigBlind, startingChips);
              }}
            />
          </div>
          <div className="config-field">
            <label>大盲注</label>
            <input
              type="number"
              value={bigBlind}
              onChange={(e) => {
                setBigBlind(e.target.value);
                updateGameConfig(smallBlind, e.target.value, startingChips);
              }}
            />
          </div>
        </div>
        <div className="config-field">
          <label>起始筹码</label>
          <input
            type="number"
            value={startingChips}
            onChange={(e) => {
              setStartingChips(e.target.value);
              updateGameConfig(smallBlind, bigBlind, e.target.value);
            }}
          />
        </div>
      </div>
    </>
  );
}
