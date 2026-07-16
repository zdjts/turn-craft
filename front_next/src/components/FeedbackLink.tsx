import { useLocation } from 'react-router-dom';

export default function FeedbackLink() {
  const location = useLocation();

  const roomMatch = location.pathname.match(/^\/game\/([^/]+)/);
  const roomId = roomMatch ? roomMatch[1] : 'N/A';

  const body = [
    '## 反馈上下文',
    '',
    `- **页面路由**: ${location.pathname}`,
    `- **房间 ID**: ${roomId}`,
    '',
    '## 反馈内容',
    '',
    '(请在此描述您的问题或建议)',
  ].join('\n');

  const href = `https://github.com/anomalyco/turn-craft/issues/new?body=${encodeURIComponent(body)}`;

  return (
    <div className="sidebar-feedback">
      <a
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        style={{ color: 'var(--text-muted)', fontSize: '0.85em', textDecoration: 'none' }}
      >
        💬 反馈建议
      </a>
    </div>
  );
}
