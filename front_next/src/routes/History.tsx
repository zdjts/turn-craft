import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { getHistoryRooms, deleteRoom, setRoomPublic } from '../api/rooms';
import type { RoomSnapshot } from '../api/rooms';
import { getGameDef } from '../games/registry';

export default function History() {
  const navigate = useNavigate();
  const [rooms, setRooms] = useState<RoomSnapshot[]>([]);
  const [loading, setLoading] = useState(true);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const res = await getHistoryRooms();
      setRooms(res.rooms ?? []);
    } catch { /* ignore */ } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const handleTogglePublic = async (roomId: string, current: boolean) => {
    try {
      await setRoomPublic(roomId, !current);
      load();
    } catch { /* ignore */ }
  };

  const handleDelete = async (roomId: string) => {
    try {
      await deleteRoom(roomId);
      setDeleteConfirm(null);
      load();
    } catch { /* ignore */ }
  };

  return (
    <div className="pg-history animate-fade-in">
      <div className="page-header">
        <div className="header-left">
          <h1>📜 历史对局房间</h1>
          <p>您创建的所有对局记录，支持设置公开展示属性。</p>
        </div>
        <button className="refresh-btn-large g-card-subtle" onClick={load}>🔄 刷新记录</button>
      </div>

      {loading ? (
        <div className="g-skeleton-list">
          {[1, 2, 3, 4].map((i) => <div key={i} className="g-skeleton-row-lg" />)}
        </div>
      ) : rooms.length === 0 ? (
        <div className="g-empty g-card">
          <div className="empty-icon">📜</div>
          <h3>暂无对局历史</h3>
          <p>您尚未创建过对局，快去大厅发起一场博弈吧！</p>
          <button className="pg-history-go" onClick={() => navigate('/lobby')}>前往大厅</button>
        </div>
      ) : (
        <div className="pg-history-list">
          {rooms.map((room) => {
            const def = getGameDef(room.game_type);
            const engine = room.engine_state as Record<string, unknown>;
            const isDone = (engine?.finished as boolean) ?? false;
            const timeStr = room.created_at.slice(0, 16).replace('T', ' ');
            return (
              <div key={room.room_id} className="pg-history-card g-card animate-slide-up">
                <div className="pg-pg-history-card-left">
                  <div className="game-badge">{def?.icon ?? '❓'} {def?.name ?? '未知游戏'}</div>
                  <div className="room-id-mono">ID: {room.room_id}</div>
                </div>

                <div className="pg-pg-history-card-mid">
                  <div className="meta-item">
                    <span className="label">状态: </span>
                    <span className="value">{isDone ? '✅ 已结算' : '⏳ 未完成'}</span>
                  </div>
                  <div className="meta-item">
                    <span className="label">总局数限制: </span>
                    <span className="value">{room.max_round} 轮</span>
                  </div>
                  <div className="meta-item">
                    <span className="label">创建时间: </span>
                    <span className="value">{timeStr}</span>
                  </div>
                </div>

                <div className="pg-pg-history-card-right">
                  <div className="pg-history-visibility">
                    <span className="pg-history-vis-label">公开状态: </span>
                    <button
                      className={room.is_public ? 'pg-history-toggle is-active' : 'pg-history-toggle'}
                      onClick={() => handleTogglePublic(room.room_id, room.is_public)}
                    >
                      {room.is_public ? '公开中 ▸' : '私有 ▸'}
                    </button>
                  </div>
                  <div className="actions-row">
                    <button className="g-btn-ghost replay" onClick={() => navigate(`/replay/${room.room_id}`)}>
                      🎞️ 回放
                    </button>
                    <button className="g-btn-ghost spectate" onClick={() => navigate(`/game/${room.room_id}/spectator`)}>
                      👁️ 观战
                    </button>
                    <button className="g-btn-ghost delete" onClick={() => setDeleteConfirm(room.room_id)}>
                      🗑️ 销毁
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {deleteConfirm && (
        <div className="modal-overlay">
          <div className="modal-confirm g-card">
            <h3>确认删除</h3>
            <p>确定要永久删除房间 {deleteConfirm} 吗？此操作不可撤销。</p>
            <div className="modal-actions">
              <button className="modal-btn cancel" onClick={() => setDeleteConfirm(null)}>取消</button>
              <button className="modal-btn confirm-delete" onClick={() => handleDelete(deleteConfirm)}>确认删除</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
