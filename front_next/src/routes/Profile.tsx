import { useState, useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { getHistoryRooms } from '../api/rooms';
import { getAchievements } from '../api/client';
import type { Achievement } from '../api/client';
import type { RoomSnapshot } from '../api/rooms';
import { getGameDef } from '../games/registry';
import { useAuth } from '../store/AuthContext';

const ACHIEVEMENT_ICONS: Record<string, string> = {
  first_game: '🌱', lincoln_5: '🏛️', texas_10: '🃏', werewolf_3_good: '🔍',
  total_50: '🎖️', all_styles: '🤖', streak_5: '🔥', spectate_10: '👀', invite_friend: '🔗',
};

export default function Profile() {
  const { username } = useAuth();
  const [rooms, setRooms] = useState<RoomSnapshot[]>([]);
  const [achievements, setAchievements] = useState<Achievement[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      getHistoryRooms(),
      getAchievements().catch(() => ({ achievements: [] as Achievement[] })),
    ]).then(([roomsRes, achRes]) => {
      setRooms(roomsRes.rooms ?? []);
      setAchievements(achRes.achievements);
    }).catch(() => {}).finally(() => setLoading(false));
  }, [username]);

  const stats = useMemo(() => {
    const total = rooms.length;
    const finished = rooms.filter((r) => {
      const e = r.engine_state as Record<string, unknown>;
      return (e?.finished as boolean) ?? false;
    });
    const byGame: Record<string, { total: number; finished: number }> = {};
    for (const r of rooms) {
      if (!byGame[r.game_type]) byGame[r.game_type] = { total: 0, finished: 0 };
      byGame[r.game_type].total++;
      const e = r.engine_state as Record<string, unknown>;
      if ((e?.finished as boolean) ?? false) byGame[r.game_type].finished++;
    }
    return { total, finished: finished.length, byGame };
  }, [rooms]);

  if (loading) {
    return <div className="pg-page"><div className="g-skeleton-list">{[1,2,3].map(i => <div key={i} className="g-skeleton-row" />)}</div></div>;
  }

  return (
    <div className="pg-profile animate-fade-in">
      <div className="page-header">
        <h1>👤 {username ?? '未登录'}</h1>
        <p>博弈数据统计与个人成就</p>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 12, marginBottom: 16 }}>
        <div className="g-card" style={{ padding: 16, textAlign: 'center' }}>
          <div style={{ fontSize: '2em', fontWeight: 700 }}>{stats.total}</div>
          <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>总对局数</div>
        </div>
        <div className="g-card" style={{ padding: 16, textAlign: 'center' }}>
          <div style={{ fontSize: '2em', fontWeight: 700 }}>{stats.finished}</div>
          <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>已完成</div>
        </div>
        <div className="g-card" style={{ padding: 16, textAlign: 'center' }}>
          <div style={{ fontSize: '2em', fontWeight: 700 }}>{stats.total > 0 ? Math.round(stats.finished / stats.total * 100) : 0}%</div>
          <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>完成率</div>
        </div>
        <div className="g-card" style={{ padding: 16, textAlign: 'center' }}>
          <div style={{ fontSize: '2em', fontWeight: 700 }}>{Object.keys(stats.byGame).length}</div>
          <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>参与游戏数</div>
        </div>
      </div>

      <div className="g-card" style={{ padding: 16, marginBottom: 16 }}>
        <h3 style={{ marginBottom: 12 }}>🎮 各游戏统计</h3>
        {Object.keys(stats.byGame).length === 0 ? (
          <p style={{ color: 'var(--text-muted)' }}>开始你的第一局对局吧！</p>
        ) : (
          Object.entries(stats.byGame).map(([gt, s]) => {
            const def = getGameDef(gt);
            const rate = s.total > 0 ? Math.round(s.finished / s.total * 100) : 0;
            return (
              <div key={gt} className="g-card-subtle" style={{ padding: 10, marginBottom: 6, display: 'flex', justifyContent: 'space-between' }}>
                <span>{def?.icon} {def?.name ?? gt}</span>
                <span>{s.total}局 · 完成{s.finished}局 · {rate}%</span>
              </div>
            );
          })
        )}
      </div>

      <div className="g-card" style={{ padding: 16, marginBottom: 16 }}>
        <h3 style={{ marginBottom: 12 }}>⏱️ 最近对局</h3>
        {rooms.length === 0 ? (
          <p style={{ color: 'var(--text-muted)' }}>暂无对局记录</p>
        ) : (
          rooms.slice(0, 5).map((r) => {
            const def = getGameDef(r.game_type);
            const e = r.engine_state as Record<string, unknown>;
            const done = (e?.finished as boolean) ?? false;
            return (
              <div key={r.room_id} className="g-card-subtle" style={{ padding: 10, marginBottom: 6, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <span>{def?.icon ?? '❓'} {def?.name ?? '?'}</span>
                  <span style={{ marginLeft: 8, fontSize: '0.85em', color: 'var(--text-muted)' }}>{r.created_at.slice(0, 10)}</span>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{ fontSize: '0.85em' }}>{done ? '✅ 已结算' : '⏳ 未完成'}</span>
                  <Link to={`/replay/${r.room_id}`} className="g-card-subtle" style={{ padding: '4px 8px', fontSize: '0.8em' }}>复盘</Link>
                </div>
              </div>
            );
          })
        )}
      </div>

      <div className="g-card" style={{ padding: 16 }}>
        <h3 style={{ marginBottom: 12 }}>🏅 成就</h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(160px, 1fr))', gap: 8 }}>
          {achievements.map((a) => (
            <div key={a.id} className="g-card-subtle" style={{
              padding: 10, textAlign: 'center',
              opacity: a.unlocked ? 1 : 0.4,
              filter: a.unlocked ? 'none' : 'grayscale(1)',
            }}>
              <div style={{ fontSize: '1.5em' }}>{a.unlocked ? (ACHIEVEMENT_ICONS[a.id] ?? '🏅') : '❓'}</div>
              <div style={{ fontSize: '0.85em', fontWeight: 600 }}>{a.unlocked ? a.name : '???'}</div>
              <div style={{ fontSize: '0.75em', color: 'var(--text-muted)' }}>
                {a.unlocked ? a.description : '未解锁'}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
