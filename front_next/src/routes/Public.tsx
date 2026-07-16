import { useState, useEffect } from 'react';
import { getPublicRooms } from '../api/rooms';
import type { RoomSnapshot } from '../api/rooms';
import PublicRoomList from '../components/PublicRoomList';

export default function PublicRooms() {
  const [rooms, setRooms] = useState<RoomSnapshot[]>([]);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    setLoading(true);
    try {
      const res = await getPublicRooms();
      setRooms(res.rooms ?? []);
    } catch { /* ignore */ } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  return (
    <div className="pg-page">
      <PublicRoomList rooms={rooms} loading={loading} onRefresh={load} />
    </div>
  );
}
