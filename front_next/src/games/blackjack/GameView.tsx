import type { GamePluginProps } from '../pluginManager';

interface Card {
  suit: string;
  rank: string;
}

interface Player {
  id: string;
  kind: string;
  hand: Card[];
  hand_value: number;
  is_bust: boolean;
  is_finished: boolean;
  bet: number;
}

interface DealerInfo {
  cards: Card[];
  value: number;
  is_bust: boolean;
}

interface BlackjackState {
  game_type: string;
  players: Player[];
  dealer: DealerInfo;
  phase: string;
  finished: boolean;
  results: { actor_id: string; outcome: string; payout: number }[];
}

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

function MiniCard({ card }: { card: Card }) {
  const color = (card.suit === 'Hearts' || card.suit === 'Diamonds') ? 'red' : 'black';
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 2, padding: '2px 6px', borderRadius: 4, border: '1px solid #ccc', margin: 2, fontSize: '0.9em', color }}>
      {rankDisplay(card.rank)}{suitSymbol(card.suit)}
    </span>
  );
}

export default function BlackjackGameView({
  state: raw,
  onAction,
  actorId,
  isMyTurn,
}: GamePluginProps) {
  const s = raw as unknown as BlackjackState;
  const phase = s.phase ?? '';

  const phaseLabel = (p: string) => {
    if (p.startsWith('PlayerTurn')) return '玩家回合';
    if (p === 'Betting') return '下注阶段';
    if (p === 'Dealing') return '发牌阶段';
    if (p === 'DealerTurn') return '庄家回合';
    if (p === 'Settlement') return '结算';
    return p;
  };

  return (
    <div className="blackjack-game" style={{ padding: 16 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <h3>🃏 二十一点</h3>
        <span className="phase-label">{phaseLabel(phase)}</span>
      </div>

      <div className="g-card" style={{ padding: 16, marginBottom: 16, background: 'var(--bg-secondary)' }}>
        <h4 style={{ marginBottom: 8 }}>🤵 庄家</h4>
        <div style={{ fontSize: '1.1em' }}>
          {s.dealer?.cards?.map((c, i) => (
            <MiniCard key={i} card={c} />
          ))}
          {!s.finished && s.dealer?.cards?.length > 1 ? (
            <span style={{ marginLeft: 8, color: 'var(--text-muted)' }}>点数: {s.dealer.value}</span>
          ) : null}
          {s.dealer?.is_bust && <span style={{ color: 'red', marginLeft: 8 }}>爆牌!</span>}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {s.players?.map((p) => {
          const isMe = p.id === actorId;
          return (
            <div key={p.id} className="g-card-subtle" style={{
              padding: 12,
              border: isMe ? '2px solid var(--accent)' : '1px solid var(--border-subtle)',
              background: isMe ? 'rgba(255,215,0,0.05)' : 'transparent',
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                <span><strong>{isMe ? `你 (${p.id})` : p.id}</strong> {p.kind === 'Ai' ? '🤖' : '👤'}</span>
                <span>💰 {p.bet} | 点数: {p.hand_value}</span>
              </div>
              <div>
                {p.hand?.map((c, i) => <MiniCard key={i} card={c} />)}
                {p.is_bust && <span style={{ color: 'red', marginLeft: 8 }}>爆牌!</span>}
                {p.is_finished && !p.is_bust && <span style={{ color: 'green', marginLeft: 8 }}>✅ 停牌</span>}
              </div>
            </div>
          );
        })}
      </div>

      {s.finished && s.results && (
        <div className="g-card" style={{ padding: 16, marginTop: 16 }}>
          <h4>🏆 结算结果</h4>
          {s.results.map((r) => (
            <div key={r.actor_id} style={{ marginTop: 4 }}>
              {r.actor_id}: {r.outcome === 'win' ? '✅ 胜利' : r.outcome === 'lose' ? '❌ 失败' : '🤝 平局'} (赔付 {r.payout})
            </div>
          ))}
        </div>
      )}

      {phase === 'Betting' && (
        <div className="gm-action-bar" style={{ marginTop: 16 }}>
          <div className="gm-action-row">
            <button className="gm-action-submit" onClick={() => onAction({ action: 'bet' })}>
              💰 开始下注
            </button>
          </div>
        </div>
      )}

      {isMyTurn && !s.finished && (
        <div className="gm-action-bar" style={{ marginTop: 16 }}>
          <div className="gm-action-row">
            <button className="gm-action-submit" onClick={() => onAction({ action: 'hit' })}>
              👆 要牌
            </button>
            <button className="gm-action-submit" style={{ background: 'var(--accent-dim)' }} onClick={() => onAction({ action: 'stand' })}>
              ✋ 停牌
            </button>
            <button className="gm-action-submit" style={{ background: 'gold', color: '#000' }} onClick={() => onAction({ action: 'double' })}>
              💰 加倍
            </button>
          </div>
        </div>
      )}

      {!isMyTurn && !s.finished && phase !== 'Settlement' && (
        <div className="gm-action-bar locked" style={{ marginTop: 16 }}>
          <div className="gm-action-row">
            <div style={{ padding: 12, color: 'var(--text-muted)' }}>等待其他玩家行动...</div>
          </div>
        </div>
      )}
    </div>
  );
}
