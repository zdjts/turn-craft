export default function About() {
  return (
    <div className="pg-about animate-fade-in">
      <div className="page-header">
        <h1>ℹ️ 关于项目</h1>
        <p>Turn Craft — 回合制博弈游戏平台</p>
      </div>

      <div className="g-card" style={{ padding: 24, lineHeight: 1.8 }}>
        <h3>🎯 项目定位</h3>
        <p>
          Turn Craft 是一个回合制博弈游戏平台，支持林肯辩论、德州扑克、狼人杀等多款游戏。
          玩家可以创建房间、配置 AI 对手，与 AI 或真人朋友进行策略对局。
        </p>

        <h3>🛠️ 技术栈</h3>
        <ul>
          <li>后端: Rust + Axum + SQLx + SQLite</li>
          <li>前端: React 18 + TypeScript + Vite</li>
          <li>游戏引擎: platform_core (Rust 纯逻辑)</li>
          <li>通信: WebSocket + REST API</li>
        </ul>

        <h3>📄 开源协议</h3>
        <p>MIT License</p>
      </div>
    </div>
  );
}
