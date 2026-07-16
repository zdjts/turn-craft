import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import Replay from '../routes/Replay';

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

beforeEach(() => {
  vi.clearAllMocks();
  Object.assign(navigator, {
    clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

describe('V7 Share — insight in share text', () => {
  it('includes insight highlight in share text when LLM returns data', async () => {
    mockGetGameDef.mockReturnValue({ name: '林肯辩论', icon: '🏛️' });
    mockGetRoom.mockResolvedValue({
      room: {
        room_id: 'room-1', owner_id: 'u1', game_type: 'lincoln',
        engine_state: { finished: true, round: 8, actors: [{ kind: 'Ai', role: 'Pro', style: 'aggressive' }, { kind: 'Ai', role: 'Con', style: 'rational' }] },
        actor_slots: [], ai_configs: {}, max_round: 8, created_at: '2026-07-16T00:00:00Z', is_public: false,
      },
    });
    mockGetAiInsights.mockResolvedValue({
      insights: [{
        actor_id: 'ai-pro', role: 'Pro', style: 'aggressive',
        overall_assessment: '', key_actions: [], highlights: ['正方在第三轮使用了一个精妙的逻辑陷阱'],
        mistakes: [],
      }],
    });

    renderReplay();

    await waitFor(() => {
      expect(screen.queryByText('🎞️ 对局回放记录')).toBeTruthy();
    });

    fireEvent.click(screen.getByText('📋 分享结果'));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(1);
    });

    const text = (navigator.clipboard.writeText as ReturnType<typeof vi.fn>).mock.calls[0][0] as string;
    expect(text).toContain('逻辑陷阱');
    expect(text).toContain('room-1');
  });

  it('falls back to static template when LLM returns no insight', async () => {
    mockGetGameDef.mockReturnValue({ name: '林肯辩论', icon: '🏛️' });
    mockGetRoom.mockResolvedValue({
      room: {
        room_id: 'room-2', owner_id: 'u1', game_type: 'lincoln',
        engine_state: { finished: true, round: 6, actors: [{ kind: 'Ai', role: 'Pro', style: 'aggressive' }, { kind: 'Ai', role: 'Con', style: 'rational' }] },
        actor_slots: [], ai_configs: {}, max_round: 8, created_at: '2026-07-16T00:00:00Z', is_public: false,
      },
    });
    mockGetAiInsights.mockResolvedValue({ insights: [] });

    renderReplay('room-2');

    await waitFor(() => {
      expect(screen.queryByText('🎞️ 对局回放记录')).toBeTruthy();
    });

    fireEvent.click(screen.getByText('📋 分享结果'));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(1);
    });

    const text = (navigator.clipboard.writeText as ReturnType<typeof vi.fn>).mock.calls[0][0] as string;
    expect(text).toContain('林肯辩论');
    expect(text).toContain('6轮');
    expect(text).not.toContain('逻辑陷阱');
  });
});
