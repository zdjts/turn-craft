import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { RoomSnapshot } from '../api/rooms';
import { joinRoom } from '../api/rooms';
import { getGameDef } from '../games/registry';

interface Props {
  rooms: RoomSnapshot[];
  loading: boolean;
  roomFilter?: string;
  onRefresh: () => void;
}

export default function PublicRoomList({ rooms, loading, roomFilter, onRefresh }: Props) {
  const navigate = useNavigate();
  const [error, setError] = useState('');

  const filtered = roomFilter
    ? rooms.filter((r) => r.game_type === roomFilter)
    : rooms;

  const handleJoin = async (room: RoomSnapshot) => {
    const slots = room.actor_slots as { slot_name: string; occupant: string }[] | null;
    const emptySlot = slots?.find((s) => s.occupant === 'Empty');
    if (emptySlot) {
      try {
        await joinRoom(room.room_id, emptySlot.slot_name);
        navigate(`/game/${room.room_id}/${emptySlot.slot_name}`);
      } catch (err) {
        setError(String(err));
      }
    } else {
      navigate(`/game/${room.room_id}/spectator`);
    }
  };

  return (
    <div className="pg-lobby-right g-card">
      <div className="pg-lobby-rooms-header">
        <h3>🌐 活跃公开房间</h3>
        <button className="pg-lobby-refresh" onClick={onRefresh} title="刷新列表">🔄</button>
      </div>

      {error && <div className="g-error">{error}</div>}

      {loading ? (
        <div className="g-skeleton-list">
          {[1, 2, 3].map((i) => <div key={i} className="g-skeleton-row" />)}
        </div>
      ) : filtered.length === 0 ? (
        <div className="g-empty">
          <div className="empty-icon">🍃</div>
          <p>当前没有活跃的公开房间，你可以自己创建一个！</p>
        </div>
      ) : (
        <div className="pg-lobby-rooms">
          {filtered.map((room) => {
            const gameDef = getGameDef(room.game_type);
            const gameName = gameDef?.name ?? '未知游戏';
            const gameIcon = gameDef?.icon ?? '❓';
            const timeStr = room.created_at.slice(0, 16).replace('T', ' ');
            const slots = room.actor_slots as { slot_name: string; occupant: string }[] | null;
            const emptyCount = slots?.filter((s) => s.occupant === 'Empty').length ?? 0;
            const firstEmpty = slots?.find((s) => s.occupant === 'Empty');

            return (
              <div key={room.room_id} className="pg-lobby-room-card g-card-subtle">
                <div className="pg-lobby-room-top">
                  <span className="pg-lobby-room-game">{gameIcon} {gameName}</span>
                  <span className="pg-lobby-room-slots">空位: {emptyCount}</span>
                </div>
                <div className="pg-lobby-room-mid">
                  <div className="pg-lobby-room-id">ID: {room.room_id}</div>
                  <div className="pg-lobby-room-meta">局数上限: {room.max_round} 轮</div>
                  <div className="pg-lobby-room-time">创建时间: {timeStr}</div>
                </div>
                <div className="pg-lobby-room-bot">
                  {firstEmpty ? (
                    <button
                      className="pg-lobby-join player"
                      onClick={() => handleJoin(room)}
                    >
                      加入对局
                    </button>
                  ) : (
                    <button
                      className="pg-lobby-join spectator"
                      onClick={() => navigate(`/game/${room.room_id}/spectator`)}
                    >
                      观战模式
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
