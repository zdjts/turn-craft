import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import Game from '../routes/Game';

function expectInDoc(element: HTMLElement | null) {
  expect(element).toBeTruthy();
  expect(element?.ownerDocument.body.contains(element)).toBe(true);
}

function expectNotInDoc(element: HTMLElement | null) {
  expect(element).toBeFalsy();
}

const mockWsReturn = vi.fn();

vi.mock('../ws/useWebSocket', () => ({
  useWebSocket: (...args: unknown[]) => mockWsReturn(...args),
}));

vi.mock('../components/ConnectionStatus', () => ({
  default: () => <div data-testid="connection-status" />,
}));

vi.mock('../games/pluginManager', () => ({
  default: () => <div data-testid="game-plugin" />,
}));

function renderGame(roomId = 'room-1', actorId = 'player1') {
  return render(
    <MemoryRouter initialEntries={[`/game/${roomId}/${actorId}`]}>
      <Routes>
        <Route path="/game/:roomId/:actorId" element={<Game />} />
      </Routes>
    </MemoryRouter>
  );
}

describe('V9 Game End Panel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows game end overlay when finished is true', async () => {
    mockWsReturn.mockReturnValue({
      state: { kind: 'connected' },
      lastState: { finished: true, game_type: 'lincoln', round: 8, actors: [] },
      streamingText: {},
      actionError: null,
      canRetry: false,
      send: vi.fn(),
      retry: vi.fn(),
      skip: vi.fn(),
      disconnect: vi.fn(),
    });

    renderGame();

    await waitFor(() => {
      expectInDoc(screen.queryByText('对局结束'));
    });

    expectInDoc(screen.queryByText('📖 查看复盘'));
  });

  it('hides play-again button for spectator', async () => {
    mockWsReturn.mockReturnValue({
      state: { kind: 'connected' },
      lastState: { finished: true, game_type: 'lincoln', round: 8, actors: [] },
      streamingText: {},
      actionError: null,
      canRetry: false,
      send: vi.fn(),
      retry: vi.fn(),
      skip: vi.fn(),
      disconnect: vi.fn(),
    });

    renderGame('room-1', 'spectator');

    await waitFor(() => {
      expectInDoc(screen.queryByText('观战结束'));
    });

    expectNotInDoc(screen.queryByText('🔄 再来一局'));
  });
});
