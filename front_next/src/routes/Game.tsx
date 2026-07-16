import { useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { get } from '../api/client';
import { useWebSocket } from '../ws/useWebSocket';
import ConnectionStatus from '../components/ConnectionStatus';
import GamePluginManager from '../games/pluginManager';

export default function Game() {
  const { roomId, actorId } = useParams<{ roomId: string; actorId: string }>();
  const navigate = useNavigate();

  const ws = useWebSocket(roomId ?? '', actorId ?? '');

  const connected = ws.state.kind === 'connected';
  const isClosed = ws.state.kind === 'closed';
  const isSpectator = actorId === 'spectator';

  const state = ws.lastState as Record<string, unknown> | null;
  const isFinished = (state?.finished as boolean) ?? false;
  const gameType = (state?.game_type as string) ?? 'unknown';

  const activeActor = useMemo(() => {
    return (state?.active_actor as string) ?? (state?.active_player as string) ?? null;
  }, [state]);

  const phase = useMemo(() => {
    return (state?.phase as string) ?? (state?.cur_role as string) ?? (state?.phase_hint as string) ?? null;
  }, [state]);

  const round = useMemo(() => {
    return (state?.round as number | undefined) ?? null;
  }, [state]);

  const isMyTurn = activeActor === actorId;

  const rosterSlots = useMemo(() => {
    const list: { id: string; role: string; kind: string }[] = [];
    const actors = state?.actors as Array<Record<string, unknown>> | undefined;
    if (actors) {
      for (const a of actors) {
        const id = a.id as string | undefined;
        const role = a.role as string | undefined;
        const kind = a.kind as string | undefined;
        if (id && role && kind) list.push({ id, role, kind });
      }
    } else {
      const players = state?.players as Array<Record<string, unknown>> | undefined;
      if (players) {
        for (const p of players) {
          const id = p.id as string | undefined;
          const kind = p.kind as string | undefined;
          if (id && kind) {
            const role = (p.position as string) ?? (p.role as string) ?? '未知';
            list.push({ id, role, kind });
          }
        }
      }
    }
    return list;
  }, [state]);

  const handleCopyRoomId = async () => {
    if (roomId && navigator.clipboard) {
      await navigator.clipboard.writeText(roomId);
    }
  };

  const handleLeave = () => {
    ws.disconnect();
    navigate('/lobby');
  };

  return (
    <div className="pg-arena">
      <div className="pg-arena-sidebar g-card">
        <div className="pg-arena-info">
          <div className="room-row-top">
            <span className="room-label">游戏房间</span>
            <button className="copy-btn" onClick={handleCopyRoomId} title="复制房间ID">📋</button>
          </div>
          <div className="room-id-mono">{roomId}</div>

          <button
            className="g-card-subtle invite-btn"
            style={{ width: '100%', marginBottom: 8, padding: '6px 12px', fontSize: '0.85em' }}
            onClick={async () => {
              try {
                const res = await get<{ status: string; invite_code: string; invite_link: string }>(`/rooms/${roomId}/invite`);
                if (res.status === 'success' && navigator.clipboard) {
                  const link = `${window.location.origin}${res.invite_link}`;
                  await navigator.clipboard.writeText(link);
                }
              } catch { /* ignore */ }
            }}
          >
            🔗 邀请好友
          </button>

          <ConnectionStatus
            connState={ws.state}
            actionError={ws.actionError}
            canRetry={ws.canRetry}
            streamingText={ws.streamingText}
            onRetry={ws.retry}
            onSkip={ws.skip}
          />

          {connected && (
            <div className="pg-arena-turn-info">
              <div className="pg-arena-turn-info-row">
                {activeActor && phase ? (
                  <>
                    <span className="turn-label">
                      {isMyTurn ? '🎯 你的回合' : `⏳ ${activeActor} 的回合`}
                    </span>
                    <span className="phase-label">{phase}</span>
                  </>
                ) : activeActor ? (
                  <span className="turn-label">⏳ {activeActor} 的回合</span>
                ) : (
                  <span className="turn-label">等待开始...</span>
                )}
              </div>
              {round != null && <div className="round-label">第 {round} 轮</div>}
            </div>
          )}
        </div>

        <div className="pg-arena-actions">
          <h4 className="roster-title">⚙️ AI 助手配置</h4>
          <div className="ai-config-buttons-list">
            {rosterSlots.filter((s) => s.kind.toLowerCase() === 'ai').length === 0 ? (
              <div className="empty-ai-configs-hint">本局无 AI 参与者</div>
            ) : (
              rosterSlots
                .filter((s) => s.kind.toLowerCase() === 'ai')
                .map((s) => (
                  <button
                    key={s.id}
                    className="pg-arena-jump g-card-subtle"
                    style={{ marginBottom: 8 }}
                    onClick={() => navigate(`/settings/${roomId}/${s.id}`)}
                  >
                    配置 {s.role} AI
                  </button>
                ))
            )}
          </div>
        </div>

        <div className="pg-arena-roster">
          <h4 className="roster-title">👥 对局参与者</h4>
          <div className="pg-arena-roster-list">
            {rosterSlots.length === 0 ? (
              <div className="pg-arena-empty">等待玩家信息...</div>
            ) : (
              rosterSlots.map((s) => {
                const isActive = activeActor === s.id;
                const isAi = s.kind.toLowerCase() === 'ai';
                let roleClass = 'role-player';
                if (s.role === 'Judge') roleClass = 'role-judge';
                else if (s.role === 'Pro') roleClass = 'role-pro';
                else if (s.role === 'Con') roleClass = 'role-con';
                return (
                  <div
                    key={s.id}
                    className={isActive ? 'gm-roster-player is-active' : 'gm-roster-player'}
                  >
                    <div className={`pg-arena-player-avatar ${roleClass}`}>
                      {isAi ? '🤖' : '🤵'}
                    </div>
                    <div className="pg-arena-player-details">
                      <div className="pg-arena-player-name">{s.id}</div>
                      <div className="pg-arena-player-role">{s.role}</div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        <div className="pg-arena-controls">
          <button className="pg-arena-leave" onClick={handleLeave}>
            🚪 返回大厅
          </button>
        </div>
      </div>

      <div className="pg-arena-viewport" style={{ position: 'relative' }}>
        {isClosed ? (
          <div className="loading-canvas g-card">
            <div style={{ fontSize: '3rem', textAlign: 'center' }}>🚪</div>
            <h3>房间已关闭</h3>
            <p>该房间不存在或已结束，请返回大厅。</p>
            <button className="pg-arena-leave" style={{ marginTop: 16 }} onClick={() => navigate('/lobby')}>
              ← 返回大厅
            </button>
          </div>
        ) : !connected ? (
          <div className="loading-canvas g-card">
            <span className="g-spinner" />
            <h3>正在建立网络连接</h3>
            <p>正在通过 WebSocket 连接至对局服务器...</p>
          </div>
        ) : !state ? (
          <div className="loading-canvas g-card">
            <div className="skeleton-canvas animate-pulse" />
            <h3>正在获取对局快照</h3>
            <p>已连接，正在同步初始状态数据...</p>
          </div>
        ) : (
          <>
            <div className="game-plugin-container g-card">
              <GamePluginManager
                gameType={gameType}
                state={state}
                onAction={(action) => ws.send(action)}
                actorId={actorId ?? ''}
                isMyTurn={isMyTurn}
                streamingText={ws.streamingText}
              />
            </div>

            {isFinished && (
              <div style={{
                position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.6)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100,
              }}>
                <div className="g-card" style={{ padding: 32, maxWidth: 400, textAlign: 'center' }}>
                  <div style={{ fontSize: '3rem' }}>{isSpectator ? '👀' : '🏆'}</div>
                  <h2 style={{ margin: '12px 0' }}>{isSpectator ? '观战结束' : '对局结束'}</h2>
                  <div style={{ margin: '16px 0', fontSize: '0.9em', color: 'var(--text-muted)' }}>
                    {gameType === 'lincoln' ? '🏛️ 林肯辩论' : gameType === 'texas_holdem' ? '🃏 德州扑克' : '🐺 狼人杀'}
                    {round != null ? ` · 共${round}轮` : ''}
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                    <button className="pg-lobby-create" onClick={() => navigate(`/replay/${roomId}`)}>
                      📖 查看复盘
                    </button>
                    {!isSpectator && (
                      <button className="g-card-subtle" style={{ padding: '10px 16px' }} onClick={() => navigate('/lobby')}>
                        🔄 再来一局
                      </button>
                    )}
                    <button className="g-card-subtle" style={{ padding: '10px 16px' }} onClick={async () => {
                      const shareText = `【${gameType === 'lincoln' ? '林肯辩论' : gameType === 'texas_holdem' ? '德州扑克' : '狼人杀'}】对局结束，共${round ?? '?'}轮\n🏛️ 查看对局: ${roomId}`;
                      if (navigator.clipboard) await navigator.clipboard.writeText(shareText);
                    }}>
                      📋 分享结果
                    </button>
                  </div>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
