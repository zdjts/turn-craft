import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import Replay from '../routes/Replay';

function expectInDoc(element: HTMLElement | null) {
  expect(element).toBeTruthy();
  expect(element?.ownerDocument.body.contains(element)).toBe(true);
}

function expectNotInDoc(element: HTMLElement | null) {
  expect(element).toBeFalsy();
}

const mockGetRoom = vi.fn();
const mockGetAiInsights = vi.fn();
const mockGetGameDef = vi.fn();

vi.mock('../api/rooms', () => ({
  getRoom: (...args: unknown[]) => mockGetRoom(...args),
  getAiInsights: (...args: unknown[]) => mockGetAiInsights(...args),
  createRoom: vi.fn(),
}));

vi.mock('../games/registry', () => ({
  getGameDef: (...args: unknown[]) => mockGetGameDef(...args),
}));

function renderReplay(roomId = 'room-1') {
  return render(
    <MemoryRouter initialEntries={[`/replay/${roomId}`]}>
      <Routes>
        <Route path="/replay/:roomId" element={<Replay />} />
      </Routes>
    </MemoryRouter>
  );
}

const baseRoom = {
  room_id: 'room-1',
  owner_id: 'user1',
  game_type: 'lincoln',
  engine_state: { finished: true, round: 8, actors: [] },
  actor_slots: [],
  ai_configs: {},
  max_round: 8,
  created_at: '2026-07-16T00:00:00Z',
  is_public: false,
};

describe('V6 Replay — insights rendering', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetGameDef.mockReturnValue({ name: '林肯辩论', icon: '🏛️' });
  });

  it('renders insights section when insights are available', async () => {
    mockGetRoom.mockResolvedValue({ room: baseRoom });
    mockGetAiInsights.mockResolvedValue({
      insights: [{
        actor_id: 'ai-pro',
        role: 'Pro',
        style: 'aggressive',
        overall_assessment: '非常有攻击性的策略',
        key_actions: [{ round: 3, action: '强力反驳', impact: 'high', reason: '论点有力' }],
        highlights: ['成功压制对方'],
        mistakes: [],
      }],
    });

    renderReplay();

    await waitFor(() => {
      expectInDoc(screen.queryByText('🤖 AI 策略深度评价'));
    });

    expectInDoc(screen.queryByText('ai-pro'));
    expectInDoc(screen.queryByText('Pro'));
  });

  it('hides insights section when insights are empty', async () => {
    mockGetRoom.mockResolvedValue({ room: baseRoom });
    mockGetAiInsights.mockResolvedValue({ insights: [] });

    renderReplay();

    await waitFor(() => {
      expectInDoc(screen.queryByText('🎞️ 对局回放记录'));
    });

    expectNotInDoc(screen.queryByText('🤖 AI 策略深度评价'));
  });
});
