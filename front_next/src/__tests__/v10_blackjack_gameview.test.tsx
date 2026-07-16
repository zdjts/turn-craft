import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BlackjackGameView from '../games/blackjack/GameView';

function expectInDoc(element: HTMLElement | null) {
  expect(element).toBeTruthy();
  expect(element?.ownerDocument.body.contains(element)).toBe(true);
}

function expectNotInDoc(element: HTMLElement | null) {
  expect(element).toBeFalsy();
}

const baseState = {
  game_type: 'blackjack',
  phase: 'PlayerTurn',
  finished: false,
  dealer: { cards: [{ suit: 'Spades', rank: 'Ten' }], value: 10, is_bust: false },
  players: [
    { id: 'player1', kind: 'Human', hand: [{ suit: 'Hearts', rank: 'Ace' }, { suit: 'Clubs', rank: 'Seven' }], hand_value: 18, is_bust: false, is_finished: false, bet: 50 },
    { id: 'ai-1', kind: 'Ai', hand: [{ suit: 'Diamonds', rank: 'Five' }, { suit: 'Spades', rank: 'Three' }], hand_value: 8, is_bust: false, is_finished: false, bet: 20 },
  ],
  results: [],
};

describe('V10 Blackjack GameView', () => {
  it('renders Hit/Stand/Double buttons when it is my turn and game not finished', () => {
    render(
      <BlackjackGameView
        state={baseState}
        onAction={vi.fn()}
        actorId="player1"
        isMyTurn={true}
        streamingText={{}}
      />
    );

    expectInDoc(screen.queryByText('👆 要牌'));
    expectInDoc(screen.queryByText('✋ 停牌'));
    expectInDoc(screen.queryByText('💰 加倍'));
  });

  it('hides action buttons when it is not my turn', () => {
    render(
      <BlackjackGameView
        state={baseState}
        onAction={vi.fn()}
        actorId="player1"
        isMyTurn={false}
        streamingText={{}}
      />
    );

    expectNotInDoc(screen.queryByText('👆 要牌'));
    expectNotInDoc(screen.queryByText('✋ 停牌'));
    expectNotInDoc(screen.queryByText('💰 加倍'));
    expectInDoc(screen.queryByText('等待其他玩家行动...'));
  });

  it('shows player hand and points', () => {
    render(
      <BlackjackGameView
        state={baseState}
        onAction={vi.fn()}
        actorId="player1"
        isMyTurn={true}
        streamingText={{}}
      />
    );

    expectInDoc(screen.queryByText(/点数: 18/));
    expectInDoc(screen.queryByText(/A/));
    expectInDoc(screen.queryByText(/7/));
  });
});
