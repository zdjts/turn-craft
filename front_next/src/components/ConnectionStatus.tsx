import type { ConnState } from '../ws/useWebSocket';
import { connStateLabel } from '../ws/useWebSocket';

interface Props {
  connState: ConnState;
  actionError: string | null;
  canRetry: boolean;
  streamingText: Record<string, string>;
  onRetry: () => void;
  onSkip: () => void;
}

export default function ConnectionStatus({
  connState,
  actionError,
  canRetry,
  streamingText,
  onRetry,
  onSkip,
}: Props) {
  const dotClass = () => {
    switch (connState.kind) {
      case 'connected': return 'status-dot online';
      case 'reconnecting': return 'status-dot reconnecting';
      case 'closed': return 'status-dot offline';
      default: return 'status-dot offline';
    }
  };

  return (
    <div className="pg-arena-conn">
      <div className={dotClass()} />
      <span className="status-text">{connStateLabel(connState)}</span>

      {actionError && (
        <div className="retry-banner">
          <div className="retry-banner-content">
            <span>⚠️ 操作失败: {actionError}</span>
            <div className="retry-banner-actions">
              {canRetry && (
                <button className="retry-btn" onClick={onRetry}>
                  🔄 重试
                </button>
              )}
              <button className="skip-btn" onClick={onSkip}>
                ⏭️ 跳过
              </button>
            </div>
          </div>
        </div>
      )}

      {Object.keys(streamingText).length > 0 && (
        <div className="streaming-indicator">
          <span className="g-spinner" /> AI 正在生成...
        </div>
      )}
    </div>
  );
}
