import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import Leaderboard from '../routes/Leaderboard';

const mockGetLeaderboard = vi.fn();
const mockGetLeaderboardByGame = vi.fn();
const mockGetGameDef = vi.fn();

vi.mock('../api/client', () => ({
  getLeaderboard: (...args: unknown[]) => mockGetLeaderboard(...args),
  getLeaderboardByGame: (...args: unknown[]) => mockGetLeaderboardByGame(...args),
}));

vi.mock('../games/registry', () => ({
  getGameDef: (...args: unknown[]) => mockGetGameDef(...args),
}));

vi.mock('../store/AuthContext', () => ({
  useAuth: () => ({ username: 'alice' }),
}));

function expectInDoc(element: HTMLElement | null) {
  expect(element).toBeTruthy();
  expect(element?.ownerDocument.body.contains(element)).toBe(true);
}

function renderLeaderboard() {
  return render(
    <MemoryRouter>
      <Leaderboard />
    </MemoryRouter>
  );
}

describe('V9 Leaderboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetLeaderboard.mockResolvedValue({ entries: [] });
    mockGetGameDef.mockReturnValue({ name: '林肯辩论', icon: '🏛️' });
  });

  it('renders four tabs', async () => {
    renderLeaderboard();

    await waitFor(() => {
      expectInDoc(screen.queryByText('对局数'));
    });

    expectInDoc(screen.queryByText('胜利榜'));
    expectInDoc(screen.queryByText('经验榜'));
    expectInDoc(screen.queryByText('按游戏'));
  });

  it('shows empty message when no data', async () => {
    renderLeaderboard();

    await waitFor(() => {
      expectInDoc(screen.queryByText('尚未有人完成对局'));
    });
  });

  it('highlights current user in the list', async () => {
    mockGetLeaderboard.mockResolvedValue({
      entries: [
        { user_id: 'u1', username: 'bob', value: 10 },
        { user_id: 'u2', username: 'alice', value: 7 },
        { user_id: 'u3', username: 'charlie', value: 5 },
      ],
    });

    renderLeaderboard();

    await waitFor(() => {
      expectInDoc(screen.queryByText('alice (你)'));
    });

    expectInDoc(screen.queryByText('bob'));
  });
});
