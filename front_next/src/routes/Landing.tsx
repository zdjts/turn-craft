import { useNavigate } from 'react-router-dom';
import { useAuth } from '../store/AuthContext';

const GAMES = [
  { icon: '🏛️', name: '林肯辩论', desc: '经典英式辩论 · 法官裁判 · 正反方交锋', players: '3 人' },
  { icon: '🃏', name: '德州扑克', desc: '2-6 人经典德扑 · 盲注博弈 · 心理对抗', players: '2-6 人' },
  { icon: '🐺', name: '狼人杀', desc: '7 人社交推理 · 狼人暗杀 · 好人投票', players: '7 人' },
  { icon: '🃏', name: '二十一点', desc: '经典 Blackjack · 庄家对赌 · 策略博弈', players: '1-6 人' },
];

const FEATURES = [
  { icon: '🤖', title: 'AI 人格', desc: '7 种 AI 风格，从保守到混乱，每一局都不同' },
  { icon: '📊', title: '策略复盘', desc: '对局结束查看 AI 的策略分析和决策洞察' },
  { icon: '👥', title: '多人对战', desc: '邀请好友加入，与人类和 AI 混合对局' },
  { icon: '🏆', title: '成就排行', desc: '解锁成就，冲击排行榜' },
];

export default function Landing() {
  const { isAuthenticated } = useAuth();
  const navigate = useNavigate();

  const handleCTA = () => {
    navigate(isAuthenticated ? '/lobby' : '/login');
  };

  return (
    <div className="landing">
      <div className="landing-hero">
        <div className="landing-hero-bg" />
        <h1 className="landing-hero-title">AI 驱动的多人策略对局平台</h1>
        <p className="landing-hero-subtitle">与 AI 同局博弈，复盘策略，分享精彩对局</p>
        <button className="landing-cta" onClick={handleCTA}>
          {isAuthenticated ? '🚀 进入大厅' : '🎮 免费开始'}
        </button>
      </div>

      <div className="landing-section">
        <h2 className="landing-section-title">选择你的游戏</h2>
        <div className="landing-games">
          {GAMES.map((g) => (
            <div key={g.name} className="landing-game-card">
              <div className="landing-game-icon">{g.icon}</div>
              <div className="landing-game-name">{g.name}</div>
              <div className="landing-game-desc">{g.desc}</div>
              <div className="landing-game-players">{g.players}</div>
            </div>
          ))}
        </div>
      </div>

      <div className="landing-section landing-features-section">
        <h2 className="landing-section-title">为什么选择 Turn Craft</h2>
        <div className="landing-features">
          {FEATURES.map((f) => (
            <div key={f.title} className="landing-feature-card">
              <div className="landing-feature-icon">{f.icon}</div>
              <div className="landing-feature-title">{f.title}</div>
              <div className="landing-feature-desc">{f.desc}</div>
            </div>
          ))}
        </div>
      </div>

      <div className="landing-cta-section">
        <h2 className="landing-cta-title">准备好开始你的第一局了吗？</h2>
        <button className="landing-cta" onClick={handleCTA}>
          {isAuthenticated ? '🚀 进入大厅' : '🎮 免费开始'}
        </button>
      </div>

      <footer className="landing-footer">
        Turn Craft · AI 驱动的多人策略对局平台
        <a href="https://github.com/anomalyco/turn-craft/issues/new" target="_blank" rel="noopener noreferrer" className="landing-footer-link">
          💬 反馈建议
        </a>
      </footer>
    </div>
  );
}
