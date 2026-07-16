import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { get } from '../api/client';

export default function InvitePage() {
  const { code } = useParams<{ code: string }>();
  const navigate = useNavigate();
  const [error, setError] = useState('');

  useEffect(() => {
    if (!code) return;
    get<{ status: string; room_id: string }>(`/invite/${code}`)
      .then((res) => {
        if (res.status === 'success' && res.room_id) {
          navigate(`/game/${res.room_id}/spectator`, { replace: true });
        } else {
          setError('邀请链接无效');
        }
      })
      .catch(() => setError('房间已关闭或邀请链接已过期'));
  }, [code, navigate]);

  return (
    <div className="login-page">
      <div className="login-card g-card" style={{ textAlign: 'center' }}>
        {error ? (
          <>
            <h2>🚪 房间已关闭</h2>
            <p>{error}</p>
            <button className="g-btn g-btn-primary" onClick={() => navigate('/lobby')} style={{ marginTop: 16 }}>
              返回大厅
            </button>
          </>
        ) : (
          <>
            <h2>🔗 正在加入房间...</h2>
            <span className="g-spinner" />
          </>
        )}
      </div>
    </div>
  );
}
