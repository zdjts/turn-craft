import { useState } from 'react';
import type { GamePluginProps } from '../pluginManager';

interface HistoryEntry {
  actor_id: string;
  role: string;
  content: string;
}

interface ActorInfo {
  id: string;
  kind: string;
  role: string;
}

interface LincolnState {
  game_type: string;
  room_id: string;
  actors: ActorInfo[];
  active_actor: string | null;
  round: number;
  max_round: number;
  finished: boolean;
  history: HistoryEntry[];
}

function parseState(raw: Record<string, unknown>): LincolnState | null {
  if (!raw.game_type) return null;
  return {
    game_type: raw.game_type as string,
    room_id: (raw.room_id as string) ?? '',
    actors: (raw.actors as ActorInfo[]) ?? [],
    active_actor: (raw.active_actor as string) ?? null,
    round: (raw.round as number) ?? 0,
    max_round: (raw.max_round as number) ?? 0,
    finished: (raw.finished as boolean) ?? false,
    history: (raw.history as HistoryEntry[]) ?? [],
  };
}

export default function LincolnGameView({
  state: raw,
  onAction,
  isMyTurn,
  streamingText,
}: GamePluginProps) {
  const [draft, setDraft] = useState('');
  const [showAiContent, setShowAiContent] = useState(true);

  const s = parseState(raw);
  if (!s) {
    return (
      <div className="gm-syncing">
        <div className="sync-g-spinner" />
        <span>正在同步对局状态...</span>
      </div>
    );
  }

  const handleSubmit = () => {
    const content = draft.trim();
    if (!content) return;
    onAction({ content });
    setDraft('');
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      if (e.ctrlKey) {
        setDraft((prev) => prev + '\n');
      } else {
        e.preventDefault();
        handleSubmit();
      }
    }
  };

  const roleDisplay = (role: string): [string, string, string] => {
    switch (role) {
      case 'Judge': return ['judge', '👑', '裁判'];
      case 'Pro': return ['pro', '🟢', '正方'];
      case 'Con': return ['con', '🔴', '反方'];
      default: return ['', '❓', '未知'];
    }
  };

  const isAi = (actorId: string) =>
    s.actors.some((a) => a.id === actorId && a.kind.toLowerCase() === 'ai');

  const activeStreaming = s.active_actor
    ? streamingText[s.active_actor]
    : undefined;

  return (
    <div className="pg-lincoln">
      <div className="gm-timeline">
        <div className="gm-phase">
          <div className="gm-phase-title">🏛️ 林肯 — 道格拉斯辩论</div>
          <div className="gm-phase-round">轮次 {s.round} / {s.max_round}</div>
          <button
            className="g-card-subtle gm-ai-toggle"
            style={{ marginLeft: 'auto', fontSize: '0.85em', padding: '4px 12px', cursor: 'pointer' }}
            onClick={() => setShowAiContent(!showAiContent)}
          >
            {showAiContent ? '👀 隐藏 AI 发言' : '🙈 显示 AI 发言'}
          </button>
        </div>

        {s.history.length === 0 && (
          <div className="gm-empty">
            <div className="gm-empty-icon">⚖️</div>
            <p className="gm-empty-text">等待裁判宣读辩题...</p>
          </div>
        )}

        {s.history.map((entry, idx) => {
          const [roleCls, icon, label] = roleDisplay(entry.role);
          const shouldHide = isAi(entry.actor_id) && !showAiContent;
          return (
            <div key={`${idx}:${entry.actor_id}`} className="gm-timeline-item">
              <div className={`gm-timeline-avatar ${roleCls}`}>{icon}</div>
              <div className="gm-timeline-body">
                <div className="gm-timeline-meta">
                  <span className="gm-timeline-author">{entry.actor_id}</span>
                  <span className={`gm-timeline-tag ${roleCls}`}>{label}</span>
                </div>
                <div className={`gm-timeline-content ${roleCls}`}>
                  {shouldHide ? (
                    <span style={{ color: '#888', fontStyle: 'italic' }}>🤖 AI 发言已隐藏</span>
                  ) : (
                    entry.content
                  )}
                </div>
              </div>
            </div>
          );
        })}

        {activeStreaming && activeStreaming.length > 0 && (() => {
          const actorInfo = s.actors.find((a) => a.id === s.active_actor);
          const [roleCls, icon, label] = roleDisplay(actorInfo?.role ?? '');
          return (
            <div className="gm-timeline-item gm-streaming">
              <div className={`gm-timeline-avatar ${roleCls}`}>{icon}</div>
              <div className="gm-timeline-body">
                <div className="gm-timeline-meta">
                  <span className="gm-timeline-author">{s.active_actor}</span>
                  <span className={`gm-timeline-tag ${roleCls}`}>{label}</span>
                  <span className="gm-streaming-indicator">⏳ 生成中...</span>
                </div>
                <div className={`gm-timeline-content ${roleCls}`}>
                  {activeStreaming}
                  <span className="gm-streaming-cursor">█</span>
                </div>
              </div>
            </div>
          );
        })()}
      </div>

      <div className={isMyTurn ? 'gm-action-bar' : 'gm-action-bar locked'}>
        <div className="gm-action-row">
          <textarea
            className="gm-action-input"
            placeholder={isMyTurn ? '作为裁判，宣读你的辩题...' : '等待你的回合...'}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={!isMyTurn}
          />
          <button
            className="gm-action-submit"
            disabled={!isMyTurn || !draft.trim()}
            onClick={handleSubmit}
          >
            提交
          </button>
        </div>
        <div className="gm-action-hint">Ctrl + Enter 快速发送</div>
      </div>
    </div>
  );
}
