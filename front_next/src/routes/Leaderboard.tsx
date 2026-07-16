import { useState, useEffect } from 'react';
import { getLeaderboard, getLeaderboardByGame } from '../api/client';
import type { LeaderboardEntry } from '../api/client';
import { useAuth } from '../store/AuthContext';
import { getGameDef } from '../games/registry';

type Tab = 'games' | 'wins' | 'experienced' | 'by-game';

export default function Leaderboard() {
  const { username } = useAuth();
  const [tab, setTab] = useState<Tab>('games');
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [gameFilter, setGameFilter] = useState('lincoln');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    const fetch = tab === 'by-game'
      ? getLeaderboardByGame(gameFilter)
      : getLeaderboard(tab, 5);
    fetch.then((res) => setEntries(res.entries ?? [])).catch(() => {}).finally(() => setLoading(false));
  }, [tab, gameFilter]);

  return (
    <div className="pg-page animate-fade-in">
      <div className="page-header">
        <h1>🏆 排行榜</h1>
        <p>查看全站玩家的对战数据和排名</p>
      </div>

      <div className="g-card" style={{ padding: 16 }}>
        <div style={{ display: 'flex', gap: 8, marginBottom: 16, flexWrap: 'wrap' }}>
          {(['games', 'wins', 'experienced', 'by-game'] as Tab[]).map((t) => (
            <button key={t} className={tab === t ? 'pg-settings-tab is-active' : 'pg-settings-tab'} onClick={() => setTab(t)}>
              {t === 'games' ? '对局数' : t === 'wins' ? '胜利榜' : t === 'experienced' ? '经验榜' : '按游戏'}
            </button>
          ))}
          {tab === 'by-game' && (
            <select value={gameFilter} onChange={(e) => setGameFilter(e.target.value)} className="g-select" style={{ marginLeft: 8 }}>
              {['lincoln', 'texas_holdem', 'werewolf'].map((gt) => {
                const def = getGameDef(gt);
                return <option key={gt} value={gt}>{def?.name ?? gt}</option>;
              })}
            </select>
          )}
        </div>

        {loading ? (
          <div className="g-skeleton-list">{[1,2,3].map((i) => <div key={i} className="g-skeleton-row" />)}</div>
        ) : entries.length === 0 ? (
          <div className="g-empty"><p>尚未有人完成对局</p></div>
        ) : (
          <div>
            {entries.map((e, i) => {
              const isMe = e.username === username;
              return (
                <div key={e.user_id} style={{
                  display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                  padding: '10px 12px', borderRadius: 6, marginBottom: 4,
                  background: isMe ? 'rgba(255,215,0,0.1)' : 'transparent',
                  border: isMe ? '1px solid rgba(255,215,0,0.3)' : '1px solid transparent',
                }}>
                  <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                    <span style={{ fontWeight: 700, fontSize: '1.1em', color: i < 3 ? 'var(--accent)' : 'var(--text-muted)', width: 24 }}>
                      {i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : `#${i + 1}`}
                    </span>
                    <span style={{ fontWeight: isMe ? 700 : 400 }}>{e.username}{isMe ? ' (你)' : ''}</span>
                  </div>
                  <span style={{ fontWeight: 600 }}>{e.value}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
