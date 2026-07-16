import { useState, useEffect } from 'react';
import type { GameConfigProps } from '../types';

const PLAYER_COUNTS = [1, 2, 3, 4, 5, 6];

export default function BlackjackLobbyCard(props: GameConfigProps) {
  const { myRole, onChange } = props;
  const [playerCount, setPlayerCountState] = useState(1);
  const [startingChips, setStartingChips] = useState('1000');

  useEffect(() => {
    if (!myRole) {
      const modes: Record<string, string> = { player1: 'human' };
      onChange({
        myRole: 'player1',
        roleConfig: modes,
        maxRound: 1,
        gameConfig: { starting_chips: 1000, min_bet: 10, max_bet: 100 },
      });
    }
  }, []);

  const setPlayerCount = (count: number) => {
    setPlayerCountState(count);
    const modes: Record<string, string> = { player1: 'human' };
    for (let i = 2; i <= count; i++) modes[`player${i}`] = 'ai';
    onChange({ myRole: 'player1', roleConfig: modes });
  };

  return (
    <>
      <div className="g-field">
        <label>游戏人数</label>
        <div className="player-count-grid">
          {PLAYER_COUNTS.map((c) => (
            <button key={c} className={playerCount === c ? 'count-btn selected' : 'count-btn'} onClick={() => setPlayerCount(c)}>
              {c} 人
            </button>
          ))}
        </div>
      </div>
      <div className="g-field">
        <label>起始筹码</label>
        <input type="number" value={startingChips} onChange={(e) => {
          setStartingChips(e.target.value);
          onChange({ gameConfig: { starting_chips: parseInt(e.target.value) || 1000, min_bet: 10, max_bet: 100 } });
        }} />
      </div>
    </>
  );
}
