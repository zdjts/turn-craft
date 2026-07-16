import { useState } from 'react';
import type { GamePluginProps } from '../pluginManager';

interface CardData {
  suit: string;
  rank: string;
}

interface PlayerView {
  id: string;
  kind: string;
  position: string;
  chips: number;
  hand_count: number;
  current_bet: number;
  total_bet: number;
  folded: boolean;
  all_in: boolean;
}

interface ShowdownResult {
  player_id: string;
  hand: CardData[];
  hand_rank: string;
  is_winner: boolean;
}

interface HistEntry {
  actor_id: string;
  action_desc: string;
  phase: string;
  ai_content: string | null;
}

interface TexasState {
  game_type: string;
  room_id: string;
  phase: string;
  pot: number;
  current_bet: number;
  community_cards: CardData[];
  players: PlayerView[];
  active_player: string | null;
  dealer_index: number;
  small_blind: number;
  big_blind: number;
  finished: boolean;
  your_hand: CardData[];
  showdown_results: ShowdownResult[];
  history: HistEntry[];
}

function parseState(raw: Record<string, unknown>): TexasState | null {
  if (!raw.game_type) return null;
  return {
    game_type: raw.game_type as string,
    room_id: (raw.room_id as string) ?? '',
    phase: (raw.phase as string) ?? 'Unknown',
    pot: (raw.pot as number) ?? 0,
    current_bet: (raw.current_bet as number) ?? 0,
    community_cards: (raw.community_cards as CardData[]) ?? [],
    players: (raw.players as PlayerView[]) ?? [],
    active_player: (raw.active_player as string) ?? null,
    dealer_index: (raw.dealer_index as number) ?? 0,
    small_blind: (raw.small_blind as number) ?? 0,
    big_blind: (raw.big_blind as number) ?? 0,
    finished: (raw.finished as boolean) ?? false,
    your_hand: (raw.your_hand as CardData[]) ?? [],
    showdown_results: (raw.showdown_results as ShowdownResult[]) ?? [],
    history: (raw.history as HistEntry[]) ?? [],
  };
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
    Two: '2', Three: '3', Four: '4', Five: '5', Six: '6', Seven: '7', Eight: '8',
    Nine: '9', Ten: '10', Jack: 'J', Queen: 'Q', King: 'K', Ace: 'A',
  };
  return map[rank] ?? rank;
}

function phaseLabel(phase: string): string {
  const map: Record<string, string> = {
    WaitingForPlayers: '等待开始',
    PreFlop: '翻牌前',
    Flop: '翻牌',
    Turn: '转牌',
    River: '河牌',
    Showdown: '摊牌',
    Finished: '已结束',
  };
  return map[phase] ?? phase;
}

function handRankLabel(rank: string): string {
  const map: Record<string, string> = {
    HighCard: '高牌', OnePair: '一对', TwoPair: '两对', ThreeOfAKind: '三条',
    Straight: '顺子', Flush: '同花', FullHouse: '葫芦', FourOfAKind: '四条',
    StraightFlush: '同花顺', RoyalFlush: '皇家同花顺',
  };
  return map[rank] ?? rank;
}

function PokerCard({ card, size }: { card: CardData; size: string }) {
  const sizeClass = size === 'large' ? 'card-large' : size === 'small' ? 'card-small' : 'card-tiny';
  const colorCls = (card.suit === 'Hearts' || card.suit === 'Diamonds') ? 'red' : 'black';
  return (
    <div className={`poker-card ${sizeClass} ${colorCls}`}>
      <div className="card-corner top-left">
        <div className="card-rank">{rankDisplay(card.rank)}</div>
        <div className="card-suit">{suitSymbol(card.suit)}</div>
      </div>
      <div className="card-center">{suitSymbol(card.suit)}</div>
      <div className="card-corner bottom-right">
        <div className="card-rank">{rankDisplay(card.rank)}</div>
        <div className="card-suit">{suitSymbol(card.suit)}</div>
      </div>
    </div>
  );
}

export default function TexasHoldemGameView({
  state: raw,
  onAction,
  actorId,
  isMyTurn,
}: GamePluginProps) {
  const [raiseAmount, setRaiseAmount] = useState('0');
  const [showAiContent, setShowAiContent] = useState(false);

  const s = parseState(raw);
  if (!s) {
    return (
      <div className="loading-screen">
        <div className="loading-g-spinner" />
        <div className="loading-text">正在连接牌桌...</div>
      </div>
    );
  }

  const isSpectator = actorId === 'spectator';
  const myPlayer = s.players.find((p) => p.id === actorId);
  const amActive = s.active_player === actorId;

  return (
    <div className="poker-game-container">
      <div className="poker-top-bar">
        <div className="game-info">
          <span className="game-title">德州扑克</span>
          <span className="separator">|</span>
          <span className="phase-text">{phaseLabel(s.phase)}</span>
        </div>
        <div className="blind-info">盲注: {s.small_blind}/{s.big_blind}</div>
      </div>

      <div className="poker-table-wrapper">
        <div className="poker-table">
          <div className="poker-pot">
            <div className="pot-chips-icon">💰</div>
            <div className="pot-amount">{s.pot}</div>
          </div>

          <div className="poker-community">
            {s.community_cards.map((card, idx) => (
              <PokerCard key={idx} card={card} size="large" />
            ))}
            {Array.from({ length: 5 - s.community_cards.length }).map((_, idx) => (
              <div key={`ph-${idx}`} className="card-slot empty" />
            ))}
          </div>

          {s.players.map((player, idx) => {
            const isMe = player.id === actorId;
            const isDealer = idx === s.dealer_index;
            const isActive = s.active_player === player.id;
            const seatClass = [
              'poker-seat',
              `seat-${idx}`,
              isActive ? 'active' : '',
              player.folded ? 'folded' : '',
              player.all_in ? 'all-in' : '',
            ].filter(Boolean).join(' ');

            const displayHand = isMe
              ? s.your_hand
              : [];

            return (
              <div key={player.id} className={seatClass}>
                {isDealer && <div className="dealer-button">D</div>}
                {player.position && <div className="position-tag">{player.position}</div>}
                <div className="poker-player-info">
                  <div className="avatar">{player.kind === 'Ai' ? '🤖' : '👤'}</div>
                  <div className="poker-player-name">{isMe ? `你 (${player.id})` : player.id}</div>
                </div>
                <div className="poker-player-chips">💰 {player.chips}</div>
                {player.current_bet > 0 && (
                  <div className="poker-player-bet">
                    <div className="bet-chip">🪙</div>
                    <div className="bet-amount">{player.current_bet}</div>
                  </div>
                )}
                {displayHand.length > 0 && (
                  <div className="poker-player-hand">
                    {displayHand.map((card, idx2) => (
                      <PokerCard key={`${player.id}-${idx2}`} card={card} size="small" />
                    ))}
                  </div>
                )}
                {player.folded && <div className="poker-player-status folded">弃牌</div>}
                {player.all_in && <div className="poker-player-status allin">ALL IN</div>}
                {isActive && (
                  <div className="active-indicator">
                    <div className="active-dot" />
                    <div className="active-text">思考中...</div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      <div className="poker-actions">
        {s.phase === 'WaitingForPlayers' && (
          <div className="waiting-start">
            <div className="waiting-icon">🎮</div>
            <div className="waiting-text">准备开始游戏</div>
            <button className="btn-start-game" onClick={() => onAction({ action: 'start' })}>
              开始游戏
            </button>
          </div>
        )}

        {s.finished ? (
          <div className="game-over-panel">
            <div className="game-over-icon">🏆</div>
            <div className="game-over-text">游戏结束</div>
            {s.showdown_results.filter((r) => r.is_winner).map((r) => (
              <div key={r.player_id} className="winner-name">🏆 {r.player_id} 获胜!</div>
            ))}
            <button className="btn-new-game" onClick={() => onAction({ action: 'start' })}>
              再来一局
            </button>
          </div>
        ) : isMyTurn && amActive ? (
          <div className="my-turn-panel">
            <div className="turn-header">
              <div className="turn-icon">🎯</div>
              <div className="turn-text">轮到你行动</div>
            </div>
            <div className="poker-actions-row">
              <button className="poker-btn poker-btn-fold" onClick={() => onAction({ action: 'fold' })}>
                弃牌
              </button>

              {(() => {
                const canCheck = myPlayer ? myPlayer.current_bet >= s.current_bet : false;
                const needCall = s.current_bet > (myPlayer?.current_bet ?? 0);
                const callAmount = s.current_bet - (myPlayer?.current_bet ?? 0);
                return (
                  <>
                    {canCheck && (
                      <button className="poker-btn poker-btn-check" onClick={() => onAction({ action: 'check' })}>
                        过牌
                      </button>
                    )}
                    {needCall && (
                      <button className="poker-btn poker-btn-call" onClick={() => onAction({ action: 'call' })}>
                        跟注 {callAmount}
                      </button>
                    )}
                  </>
                );
              })()}

              <div className="raise-group">
                <input
                  className="raise-input"
                  type="number"
                  value={raiseAmount}
                  onChange={(e) => setRaiseAmount(e.target.value)}
                  placeholder="金额"
                />
                <button
                  className="poker-btn poker-btn-raise"
                  onClick={() => {
                    const amt = parseInt(raiseAmount) || 0;
                    if (amt > s.current_bet) onAction({ action: 'raise', amount: amt });
                  }}
                >
                  加注
                </button>
              </div>

              <button className="poker-btn poker-btn-allin" onClick={() => onAction({ action: 'all_in' })}>
                ALL IN
              </button>
            </div>
          </div>
        ) : isSpectator ? (
          <div className="spectator-panel">
            <div className="spectator-icon">👀</div>
            <div className="spectator-text">观战模式</div>
            <div className="spectator-hint">
              {s.active_player ? `等待 ${s.active_player} 行动...` : '等待游戏继续...'}
            </div>
          </div>
        ) : (
          <div className="waiting-panel">
            <div className="waiting-g-spinner" />
            <div className="waiting-text">
              {s.active_player ? `等待 ${s.active_player} 行动...` : '等待中...'}
            </div>
          </div>
        )}
      </div>

      {(s.phase === 'Showdown' || s.phase === 'Finished') && s.showdown_results.length > 0 && (
        <div className="showdown-panel">
          <div className="showdown-title">摊牌结果</div>
          <div className="showdown-cards">
            {s.showdown_results.map((result) => (
              <div key={result.player_id} className={result.is_winner ? 'showdown-player winner' : 'showdown-player'}>
                <div className="showdown-name">{result.player_id}</div>
                <div className="showdown-hand">
                  {result.hand.map((card, idx) => (
                    <PokerCard key={`sd-${result.player_id}-${idx}`} card={card} size="tiny" />
                  ))}
                </div>
                <div className="showdown-rank">{handRankLabel(result.hand_rank)}</div>
                {result.is_winner && <div className="showdown-winner-badge">🏆</div>}
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="history-panel g-card poker-history-panel">
        <div className="history-header poker-history-header">
          <h4>📜 历史流水记录</h4>
          <button
            className="g-card-subtle gm-ai-toggle poker-history-toggle"
            onClick={() => setShowAiContent(!showAiContent)}
          >
            {showAiContent ? '👀 隐藏 AI 心声' : '🙈 显示 AI 心声'}
          </button>
        </div>
        <div className="history-list poker-history-list">
          {s.history.length === 0 ? (
            <div className="poker-history-empty">暂无记录</div>
          ) : (
            s.history.map((entry, idx) => (
              <div key={idx} className="history-item poker-history-item">
                <div className="history-action">
                  <span className="poker-history-actor">{entry.actor_id}</span>
                  <span className="poker-history-desc">[{phaseLabel(entry.phase)}] {entry.action_desc}</span>
                </div>
                {showAiContent && entry.ai_content && (
                  <div className="history-ai-content poker-history-ai-card">🤖: {entry.ai_content}</div>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
