import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { getRoom, createRoom, getAiInsights } from '../api/rooms';
import type { RoomSnapshot, AiInsight } from '../api/rooms';
import { getGameDef } from '../games/registry';

function suitSymbol(suit: string): string {
  switch (suit) {
    case 'Hearts': return '♥';
    case 'Diamonds': return '♦';
    case 'Clubs': return '♣';
    case 'Spades': return '♠';
    default: return '?';
  }
}

function rankDisplay(rank: string): string {
  const map: Record<string, string> = {
    Two: '2', Three: '3', Four: '4', Five: '5', Six: '6',
    Seven: '7', Eight: '8', Nine: '9', Ten: '10',
    Jack: 'J', Queen: 'Q', King: 'K', Ace: 'A',
  };
  return map[rank] ?? rank;
}

export default function Replay() {
  const { roomId } = useParams<{ roomId: string }>();
  const navigate = useNavigate();

  const [room, setRoom] = useState<RoomSnapshot | null>(null);
  const [insights, setInsights] = useState<AiInsight[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    if (!roomId) return;
    setLoading(true);
    Promise.all([
      getRoom(roomId),
      getAiInsights(roomId).catch(() => ({ insights: [] as AiInsight[] })),
    ])
      .then(([roomRes, insightsRes]) => {
        setRoom(roomRes.room);
        setInsights(insightsRes.insights);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [roomId]);

  function pickBestInsight(): string {
    if (insights.length === 0) return '';
    for (const ins of insights) {
      if (ins.highlights.length > 0) return ins.highlights[0];
    }
    for (const ins of insights) {
      if (ins.mistakes.length > 0) return ins.mistakes[0];
    }
    for (const ins of insights) {
      if (ins.overall_assessment) return ins.overall_assessment;
    }
    return '';
  }

  function buildShareText(): string {
    if (!room || !roomId) return '';
    const def = getGameDef(room.game_type);
    const name = def?.name ?? '?';
    const engine = room.engine_state as Record<string, unknown>;
    const done = (engine?.finished as boolean) ?? false;
    const rnd = (engine?.round as number) ?? 0;
    const pot = (engine?.pot as number) ?? 0;
    const actors = (engine?.actors as Array<Record<string, unknown>>) ?? [];
    const aiActors = actors.filter((a) => (a.kind as string)?.toLowerCase() === 'ai');
    const insight = pickBestInsight();

    if (insight) {
      const header = room.game_type === 'lincoln' ? '【林肯辩论】' : room.game_type === 'texas_holdem' ? '【德州扑克】' : `【${name}】`;
      return `${header}${insight}\n🏛️ 查看对局: ${roomId}`;
    }

    if (room.game_type === 'lincoln') {
      const pro = aiActors.find((a) => a.role === 'Pro');
      const con = aiActors.find((a) => a.role === 'Con');
      const proStyle = (pro?.style as string) ?? 'default';
      const conStyle = (con?.style as string) ?? 'default';
      const styleLabel = (s: string) =>
        s === 'rational' ? '理性' : s === 'aggressive' ? '激进' : s === 'deceptive' ? '狡猾' : s === 'chaotic' ? '混乱' : s === 'creative' ? '创意' : '';
      return `【林肯辩论】AI ${styleLabel(proStyle) || ''}正方 vs AI ${styleLabel(conStyle) || ''}反方\n共${rnd}轮 · ${done ? '已结算' : '未完成'}\n🏛️ 查看对局: ${roomId}`;
    }

    if (room.game_type === 'texas_holdem') {
      return `【德州扑克】奖池💰${pot} · ${(room.actor_slots as Array<unknown>)?.length ?? 0}人桌${done ? ' ✅' : ' ⏳'}\n🃏 查看对局: ${roomId}`;
    }

    const slots = (room.actor_slots as Array<unknown>)?.length ?? 0;
    const extra = `共 ${rnd} 轮 | ${slots} 人`;
    return `Turn Craft | ${name} ${done ? '✅' : '⏳'}\n${extra}\n房间: ${roomId}`;
  }

  const handleShare = () => {
    const text = buildShareText();
    if (text && navigator.clipboard) {
      navigator.clipboard.writeText(text);
    }
  };

  const handlePlayAgain = async () => {
    if (!room || creating) return;
    setCreating(true);
    const slots = (room.actor_slots as Array<Record<string, string>>) ?? [];
    const slotNames = slots.map((s) => s.slot_name).filter(Boolean);
    const configs: Record<string, string> = {};
    for (const s of slots) {
      configs[s.slot_name] = s.occupant === 'Ai' ? 'ai' : 'human';
    }
    const firstEmpty = slots.find((s) => s.occupant === 'Empty' || s.occupant === 'Human');
    const mySlot = firstEmpty?.slot_name ?? 'spectator';
    const engine = room.engine_state as Record<string, unknown>;

    try {
      const res = await createRoom({
        game_type: room.game_type,
        max_round: room.max_round,
        my_slot: mySlot,
        slots: slotNames,
        slot_configs: configs,
        game_config: engine?.game_config ?? undefined,
        is_public: true,
      });
      if (res.status === 'success' && res.room_id && res.actor_id) {
        navigate(`/game/${res.room_id}/${res.actor_id}`);
      }
    } catch {
      // silently fail
    } finally {
      setCreating(false);
    }
  };

  if (loading) {
    return (
      <div className="pg-replay animate-fade-in">
        <div className="pg-arena-loading g-card">
          <span className="g-spinner" />
          <p>正在读取对局记录...</p>
        </div>
      </div>
    );
  }

  if (!room) {
    return (
      <div className="pg-replay animate-fade-in">
        <div className="pg-replay-error g-card">
          <p>未能加载到该对局数据。</p>
        </div>
      </div>
    );
  }

  const def = getGameDef(room.game_type);
  const engine = room.engine_state as Record<string, unknown>;
  const finished = (engine?.finished as boolean) ?? false;
  const round = (engine?.round as number) ?? 0;
  const pot = (engine?.pot as number) ?? 0;
  const history = (engine?.history as Array<Record<string, unknown>>) ?? [];

  return (
    <div className="pg-replay animate-fade-in">
      <div className="page-header">
        <div className="header-left">
          <h1>🎞️ 对局回放记录</h1>
          <p>这里是该房间历史状态与局内对话的静态复盘记录。</p>
        </div>
        <div className="header-right">
          <button className="pg-replay-back g-card-subtle" onClick={() => navigate('/history')}>
            ⬅️ 返回历史列表
          </button>
          <button className="pg-replay-share g-card-subtle" style={{ marginLeft: 8 }} onClick={handleShare}>
            📋 分享结果
          </button>
        </div>
      </div>

      <div className="pg-replay-detail g-card">
        <div className="pg-replay-meta-header">
          <div className="pg-replay-meta-badge">{def?.icon ?? '❓'} {def?.name ?? '未知游戏'}</div>
          <div className="meta-item">房间 ID: {room.room_id}</div>
          <div className="meta-item">总局数: {room.max_round} 轮</div>
          <div className="meta-item">创建时间: {room.created_at.replace('T', ' ')}</div>
        </div>

        <div className="pg-replay-actions g-card-subtle">
          <button className="pg-replay-play-again" onClick={handlePlayAgain} disabled={creating}>
            🔄 再来一局（相同配置）
          </button>
        </div>

        {insights.length > 0 && (
          <div className="pg-replay-insights g-card" style={{ marginBottom: 16, padding: 16 }}>
            <h4 style={{ marginBottom: 12 }}>🤖 AI 策略深度评价</h4>
            {insights.map((ins) => {
              const styleLabel: Record<string, string> = {
                default: '默认', aggressive: '激进', conservative: '保守',
                creative: '创意', deceptive: '狡猾', rational: '理性', chaotic: '混乱',
              };
              return (
                <div key={ins.actor_id} style={{
                  border: '1px solid var(--border-subtle)', borderRadius: 8, padding: 14, marginBottom: 12,
                  background: 'var(--bg-secondary)',
                }}>
                  <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 8, flexWrap: 'wrap' }}>
                    <strong style={{ fontSize: '1.05em' }}>{ins.actor_id}</strong>
                    <span style={{ background: 'var(--accent-dim)', padding: '2px 8px', borderRadius: 4, fontSize: '0.8em' }}>{ins.role}</span>
                    <span style={{ background: 'var(--accent-dim)', padding: '2px 8px', borderRadius: 4, fontSize: '0.8em' }}>{styleLabel[ins.style] ?? ins.style}</span>
                  </div>

                  {ins.overall_assessment ? (
                    <div style={{ marginBottom: 10, padding: '8px 12px', background: 'var(--bg-card)', borderRadius: 6, borderLeft: '3px solid var(--accent)', fontStyle: 'italic' }}>
                      💬 {ins.overall_assessment}
                    </div>
                  ) : (
                    <div style={{ marginBottom: 10, fontSize: '0.85em', color: 'var(--text-muted)' }}>
                      策略评价暂不可用
                    </div>
                  )}

                  {ins.key_actions.length > 0 && (
                    <div style={{ marginBottom: 8 }}>
                      <div style={{ fontSize: '0.85em', color: 'var(--text-muted)', marginBottom: 4 }}>关键行动:</div>
                      {ins.key_actions.map((a, i) => (
                        <div key={i} style={{ fontSize: '0.85em', padding: '3px 0', borderBottom: i < ins.key_actions.length - 1 ? '1px solid var(--border-subtle)' : 'none' }}>
                          <span style={{ color: 'var(--accent)', fontWeight: 600 }}>第{a.round}轮</span>
                          <span style={{ marginLeft: 6 }}>{a.action}</span>
                          {a.reason && <span style={{ display: 'block', marginLeft: 14, color: 'var(--text-muted)', fontSize: '0.9em' }}>原因: {a.reason}</span>}
                          <span className={a.impact === 'high' ? 'g-badge-success' : 'g-badge-info'} style={{ marginLeft: 8, fontSize: '0.75em' }}>
                            {a.impact === 'high' ? '高影响' : '中影响'}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}

                  {ins.highlights.length > 0 && (
                    <div style={{ marginTop: 6, marginBottom: 4 }}>
                      {ins.highlights.map((h, i) => (
                        <div key={i} style={{ background: 'rgba(0,180,0,0.08)', borderLeft: '3px solid #0a0', padding: '6px 10px', borderRadius: 4, marginBottom: 4, fontSize: '0.85em', color: '#0a0' }}>
                          ⭐ {h}
                        </div>
                      ))}
                    </div>
                  )}

                  {ins.mistakes.length > 0 && (
                    <div style={{ marginTop: 6 }}>
                      {ins.mistakes.map((m, i) => (
                        <div key={i} style={{ background: 'rgba(220,0,0,0.06)', borderLeft: '3px solid #c00', padding: '6px 10px', borderRadius: 4, marginBottom: 4, fontSize: '0.85em', color: '#c00' }}>
                          ❌ {m}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}

        <div className="pg-replay-body">
          <div className="pg-replay-stats" style={{ display: 'flex', gap: 16, marginBottom: 16 }}>
            <div className="pg-replay-stat g-card-subtle" style={{ flex: 1, textAlign: 'center', padding: 12 }}>
              <div style={{ fontSize: '1.5em', fontWeight: 'bold' }}>{finished ? '✅' : '⏳'}</div>
              <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>{finished ? '已结算' : '未完成'}</div>
            </div>
            <div className="pg-replay-stat g-card-subtle" style={{ flex: 1, textAlign: 'center', padding: 12 }}>
              <div style={{ fontSize: '1.5em', fontWeight: 'bold' }}>{round}</div>
              <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>轮数</div>
            </div>
            {room.game_type === 'texas_holdem' && (
              <div className="pg-replay-stat g-card-subtle" style={{ flex: 1, textAlign: 'center', padding: 12 }}>
                <div style={{ fontSize: '1.5em', fontWeight: 'bold' }}>💰 {pot}</div>
                <div style={{ fontSize: '0.85em', color: 'var(--text-muted)' }}>总奖池</div>
              </div>
            )}
          </div>

          {room.game_type === 'lincoln' && (
            <div className="lincoln-replay-view">
              <div className="pg-replay-round">
                <span>🏛️ 林肯辩论历史辩词 (共 {round} 轮)</span>
              </div>
              <div className="timeline-scroll pg-replay-timeline">
                {history.length === 0 ? (
                  <p style={{ color: 'var(--text-muted)', textAlign: 'center', padding: 20 }}>没有发言记录</p>
                ) : (
                  history.map((entry, idx) => {
                    const role = entry.role as string;
                    const aId = entry.actor_id as string;
                    const content = entry.content as string;
                    const [roleCls, icon, label] = role === 'Judge' ? ['judge', '👑', '裁判'] : role === 'Pro' ? ['pro', '🟢', '正方'] : role === 'Con' ? ['con', '🔴', '反方'] : ['', '❓', '未知'];
                    return (
                      <div key={idx} className="gm-timeline-item">
                        <div className={`gm-timeline-avatar ${roleCls}`}>{icon}</div>
                        <div className="gm-timeline-body">
                          <div className="gm-timeline-meta">
                            <span className="gm-timeline-author">{aId}</span>
                            <span className={`gm-timeline-tag ${roleCls}`}>{label}</span>
                          </div>
                          <div className={`gm-timeline-content ${roleCls}`}>{content}</div>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          )}

          {room.game_type === 'texas_holdem' && (
            <div className="texas-replay-view">
              <div className="community-cards-section">
                <h4>🃏 公共牌</h4>
                <div className="community-cards-list" style={{ display: 'flex', gap: 4 }}>
                  {(engine?.community_cards as Array<Record<string, string>> ?? []).length === 0 ? (
                    <p className="empty-lbl">无公共牌</p>
                  ) : (
                    (engine?.community_cards as Array<Record<string, string>> ?? []).map((c, idx) => (
                      <div key={idx} className={`poker-card-mini ${c.suit === 'Hearts' || c.suit === 'Diamonds' ? 'card-red' : 'card-black'}`}>
                        <span className="rank">{rankDisplay(c.rank)}</span>
                        <span className="suit">{suitSymbol(c.suit)}</span>
                      </div>
                    ))
                  )}
                </div>
              </div>

              {(engine?.showdown_results as Array<Record<string, unknown>> ?? []).length > 0 && (
                <div className="showdown-replay-section">
                  <h4>🏆 摊牌与结算结果</h4>
                  <div className="showdown-list">
                    {(engine?.showdown_results as Array<Record<string, unknown>> ?? []).map((res, idx) => {
                      const pId = res.player_id as string;
                      const rankDesc = res.hand_rank as string;
                      const winner = res.is_winner as boolean;
                      return (
                        <div key={idx} className={winner ? 'pg-replay-showdown winner g-card-subtle' : 'pg-replay-showdown'}>
                          <span className="showdown-winner-icon">{winner ? '👑' : '👤'}</span>
                          <span className="showdown-player">{pId}</span>
                          <span className="showdown-rank">{rankDesc}</span>
                          {winner && <span className="winner-tag">获胜者</span>}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
