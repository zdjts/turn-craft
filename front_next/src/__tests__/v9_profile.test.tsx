import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import Profile from '../routes/Profile';

const mockGetHistoryRooms = vi.fn();
const mockGetAchievements = vi.fn();
const mockGetGameDef = vi.fn();

vi.mock('../api/rooms', () => ({
  getHistoryRooms: (...args: unknown[]) => mockGetHistoryRooms(...args),
}));

vi.mock('../api/client', () => ({
  getAchievements: (...args: unknown[]) => mockGetAchievements(...args),
}));

vi.mock('../games/registry', () => ({
  getGameDef: (...args: unknown[]) => mockGetGameDef(...args),
}));

vi.mock('../store/AuthContext', () => ({
  useAuth: () => ({ username: 'testuser' }),
}));

function renderProfile() {
  return render(
    <MemoryRouter>
      <Profile />
    </MemoryRouter>
  );
}

function expectInDoc(element: HTMLElement | null) {
  expect(element).toBeTruthy();
  expect(element?.ownerDocument.body.contains(element)).toBe(true);
}

function expectNotInDoc(element: HTMLElement | null) {
  expect(element).toBeFalsy();
}

const baseRoom = (overrides: Record<string, unknown> = {}) => ({
  room_id: 'r1', owner_id: 'u1', game_type: 'lincoln',
  engine_state: { finished: false },
  actor_slots: [], ai_configs: {}, max_round: 8,
  created_at: '2026-07-16T00:00:00Z', is_public: false,
  ...overrides,
});

describe('V9 Profile', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetGameDef.mockReturnValue({ name: '林肯辩论', icon: '🏛️' });
  });

  it('renders four stat cards', async () => {
    mockGetHistoryRooms.mockResolvedValue({
      rooms: [
        baseRoom({ engine_state: { finished: true } }),
        baseRoom({ room_id: 'r2', engine_state: { finished: true } }),
        baseRoom({ room_id: 'r3' }),
      ],
    });
    mockGetAchievements.mockResolvedValue({ achievements: [] });

    renderProfile();

    await waitFor(() => {
      expectInDoc(screen.queryByText('总对局数'));
    });

    expectInDoc(screen.queryByText('已完成'));
    expectInDoc(screen.queryByText('完成率'));
    expectInDoc(screen.queryByText('参与游戏数'));
    expect(screen.getByText('3')).toBeTruthy();
    expect(screen.getByText('2')).toBeTruthy();
  });

  it('shows empty state for new user', async () => {
    mockGetHistoryRooms.mockResolvedValue({ rooms: [] });
    mockGetAchievements.mockResolvedValue({ achievements: [] });

    renderProfile();

    await waitFor(() => {
      expectInDoc(screen.queryByText('开始你的第一局对局吧！'));
    });

    expectInDoc(screen.queryByText('暂无对局记录'));
  });

  it('renders achievements grid with 9 items', async () => {
    mockGetHistoryRooms.mockResolvedValue({ rooms: [] });
    mockGetAchievements.mockResolvedValue({
      achievements: [
        { id: 'first_game', name: '初入棋局', description: '完成第一局游戏', unlocked: true },
        { id: 'lincoln_5', name: '林肯5胜', description: '林肯辩论胜5局', unlocked: true },
        { id: 'texas_10', name: '德州10胜', description: '德州扑克胜10局', unlocked: true },
        { id: 'werewolf_3_good', name: '好人3胜', description: '好人阵营赢3局', unlocked: true },
        { id: 'total_50', name: '对局50', description: '总对局50局', unlocked: true },
        { id: 'all_styles', name: '全能风格', description: '使用7种AI风格', unlocked: true },
        { id: 'streak_5', name: '五连绝世', description: '连续赢5局', unlocked: true },
        { id: 'spectate_10', name: '观战10次', description: '观战10局', unlocked: true },
        { id: 'invite_friend', name: '呼朋引伴', description: '邀请好友对局', unlocked: true },
      ],
    });

    renderProfile();

    await waitFor(() => {
      expectInDoc(screen.queryByText('初入棋局'));
    });

    expectInDoc(screen.queryByText('林肯5胜'));
    expectInDoc(screen.queryByText('德州10胜'));
    expectInDoc(screen.queryByText('好人3胜'));
    expectInDoc(screen.queryByText('对局50'));
    expectInDoc(screen.queryByText('全能风格'));
    expectInDoc(screen.queryByText('五连绝世'));
    expectInDoc(screen.queryByText('观战10次'));
    expectInDoc(screen.queryByText('呼朋引伴'));
  });
});
