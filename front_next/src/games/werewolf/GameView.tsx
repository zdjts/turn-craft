import { useState } from 'react';
import type { GamePluginProps } from '../pluginManager';

function getPlayerRole(players: unknown, myId: string): string {
  const arr = (players as Array<Record<string, unknown>>) ?? [];
  const me = arr.find((p) => p.id === myId);
  return (me?.role as string) ?? '未知';
}

function getPlayerAlive(players: unknown, myId: string): boolean {
  const arr = (players as Array<Record<string, unknown>>) ?? [];
  const me = arr.find((p) => p.id === myId);
  return (me?.is_alive as boolean) ?? false;
}

function parsePhaseName(phase: unknown): string {
  if (phase === 'Init') return '等待开始...';
  if (phase === 'NightWolf') return '🌙 狼人行动';
  if (phase === 'NightSeer') return '🌙 预言家行动';
  if (phase === 'NightWitch') return '🌙 女巫行动';
  if (phase === 'DayAnnounce') return '☀️ 昨夜结果';
  if (phase === 'DaySpeech') return '☀️ 发言阶段';
  if (phase === 'DayVote') return '🗳️ 投票阶段';
  if (typeof phase === 'object' && phase) {
    const p = phase as Record<string, unknown>;
    if (p.DayHunterShoot) return `🔫 猎人抉择`;
    if (p.GameOver) return `🏁 游戏结束 (${p.GameOver})`;
  }
  return '未知';
}

export default function WerewolfGameView({
  state: raw,
  onAction,
  actorId,
  streamingText,
}: GamePluginProps) {
  const [showAiContent, setShowAiContent] = useState(true);
  const [draft, setDraft] = useState('');

  const rawPhase = raw.phase;
  const winnerSide = typeof rawPhase === 'object' && rawPhase ? (rawPhase as Record<string, unknown>).GameOver as string | undefined : undefined;
  const isFinished = !!winnerSide;
  const myRole = getPlayerRole(raw.players, actorId);
  const myAlive = getPlayerAlive(raw.players, actorId);
  const phaseName = parsePhaseName(raw.phase);
  const day = (raw.day as number) ?? 1;

  const players = (raw.players as Array<Record<string, unknown>>) ?? [];
  const history = (raw.history as Array<Record<string, unknown>>) ?? [];
  const activeActor = (raw.active_actor as string) ?? null;
  const amActive = activeActor === actorId;

  const handleSubmit = () => {
    const content = draft.trim();
    if (!content) return;
    onAction({ content });
    setDraft('');
  };

  const handleVote = (target: string) => {
    onAction({ action: 'vote', target });
  };

  const activeStreaming = activeActor ? streamingText[activeActor] : undefined;

  return (
    <div className="pg-lincoln">
      <div className="gm-timeline">
        <div className="gm-phase">
          <div className="gm-phase-title">🐺 狼人杀 — 7人标准局</div>
          <div className="gm-phase-round">第 {day} 天 — {phaseName}</div>
          <button
            className="g-card-subtle gm-ai-toggle"
            style={{ marginLeft: 'auto', fontSize: '0.85em', padding: '4px 12px', cursor: 'pointer' }}
            onClick={() => setShowAiContent(!showAiContent)}
          >
            {showAiContent ? '👀 隐藏 AI 思考' : '🙈 显示 AI 思考'}
          </button>
        </div>

        <div
          className="players-status-bar"
          style={{ display: 'flex', gap: 8, padding: '10px 20px', flexWrap: 'wrap', background: 'var(--bg-card)', borderBottom: '1px solid var(--border-subtle)' }}
        >
          {players.map((p) => {
            const id = p.id as string;
            const alive = (p.is_alive as boolean) ?? false;
            const knownRole = p.role as string | undefined;
            const isMe = id === actorId;
            const isWolf = knownRole === 'Werewolf';

            return (
              <div
                key={id}
                style={{
                  padding: '4px 10px', borderRadius: 12, fontSize: '0.85em',
                  opacity: alive ? 1 : 0.4,
                  color: alive ? 'var(--text-primary)' : 'var(--text-muted)',
                  background: isMe ? 'rgba(255, 215, 0, 0.2)' : isWolf ? 'rgba(255, 50, 50, 0.2)' : 'transparent',
                  border: '1px solid var(--border-subtle)',
                  display: 'flex', alignItems: 'center', gap: 4,
                }}
              >
                {!alive ? '💀 ' : '👤 '}{id}
                {isWolf && <span style={{ fontSize: '1.1em' }}>🐺</span>}
              </div>
            );
          })}
        </div>

        {!isFinished && (
          <div style={{ textAlign: 'center', color: 'var(--accent)', margin: '8px 0' }}>
            {`你的身份: ${myRole}  |  状态: ${myAlive ? '存活' : '已阵亡'}`}
          </div>
        )}

        {history.map((evt, idx) => {
          const content = evt.content as string | undefined;
          if (!content) return null;
          const actor = (evt.actor_id as string) ?? 'System';
          const isSys = actor === 'System';
          const d = (evt.day as number) ?? 0;
          return (
            <div key={idx} className="gm-timeline-item">
              <div className={isSys ? 'gm-timeline-avatar' : 'gm-timeline-avatar pro'}>
                {isSys ? '⚖️' : '👤'}
              </div>
              <div className="gm-timeline-body">
                <div className="gm-timeline-meta">
                  <span className="gm-timeline-author">{isSys ? '系统播报' : actor}</span>
                  <span className="gm-timeline-tag">Day {d}</span>
                </div>
                <div className={isSys ? 'gm-timeline-content sys-msg' : 'gm-timeline-content'}>
                  {content}
                </div>
              </div>
            </div>
          );
        })}

        {activeStreaming && activeStreaming.length > 0 && (
          <div className="gm-timeline-item gm-streaming">
            <div className="gm-timeline-avatar pro">👤</div>
            <div className="gm-timeline-body">
              <div className="gm-timeline-meta">
                <span className="gm-timeline-author">{activeActor}</span>
                <span className="gm-streaming-indicator">⏳ 生成中...</span>
              </div>
              <div className="gm-timeline-content">
                {activeStreaming}
                <span className="cursor-blink">█</span>
              </div>
            </div>
          </div>
        )}

        {isFinished && (
          <div className="showdown-panel" style={{ marginTop: 20 }}>
            <div className="showdown-title">
              {winnerSide === 'Wolves' ? '🐺 狼人阵营胜利！' : '🧑‍🌾 好人阵营胜利！'}
            </div>
            <div className="showdown-cards">
              {players.map((p) => {
                const id = p.id as string;
                const role = (p.role as string) ?? '未知';
                const alive = (p.is_alive as boolean) ?? false;
                const isWolf = role === 'Werewolf';
                const isWinner = (winnerSide === 'Wolves' && isWolf) || (winnerSide === 'Humans' && !isWolf);
                const roleIcon = role === 'Werewolf' ? '🐺' : role === 'Seer' ? '👁️' : role === 'Witch' ? '🧪' : role === 'Hunter' ? '🔫' : '🧑‍🌾';
                return (
                  <div key={id} className={isWinner ? 'showdown-player winner' : 'showdown-player'}>
                    <div className="showdown-name">{id}</div>
                    <div className="showdown-hand" style={{ fontSize: '1.5em', margin: '10px 0' }}>
                      {roleIcon}
                    </div>
                    <div className="showdown-rank">{role}{!alive ? ' (阵亡)' : ''}</div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {!isFinished && rawPhase === 'Init' && (
        <div className="gm-action-bar">
          <div className="gm-action-row">
            <button className="gm-action-submit" onClick={() => onAction({ action_type: 'start' })}>
              🎮 开始游戏
            </button>
          </div>
        </div>
      )}

      {!isFinished && myAlive && (
        <div className={amActive ? 'gm-action-bar' : 'gm-action-bar locked'}>
          {amActive && (
            <>
              <div className="gm-action-row">
                <textarea
                  className="gm-action-input"
                  placeholder="输入你的发言..."
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                />
                <button className="gm-action-submit" disabled={!draft.trim()} onClick={handleSubmit}>
                  发言
                </button>
              </div>

              <div style={{ padding: '8px 16px' }}>
                <div style={{ fontSize: '0.85em', marginBottom: 8, color: 'var(--text-muted)' }}>投票目标:</div>
                <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  {players.filter((p) => (p.is_alive as boolean) && p.id !== actorId).map((p) => (
                    <button
                      key={p.id as string}
                      className="g-card-subtle"
                      style={{ padding: '4px 8px', cursor: 'pointer' }}
                      onClick={() => handleVote(p.id as string)}
                    >
                      投票 {p.id as string}
                    </button>
                  ))}
                </div>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
