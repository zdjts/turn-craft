# Turn Craft — 回合制博弈游戏平台

林肯辩论 · 德州扑克 · 狼人杀 · 二十一点。搭配个性化 AI 助手，开启精彩协作。

## 快速启动

```bash
docker compose up -d
```

浏览器打开 `http://localhost`，注册账号即可开始。

## 游戏列表

| 游戏 | 人数 | 类型 | 说明 |
|------|------|------|------|
| 🏛️ 林肯辩论 | 3 人 | main | 法官主持，AI 正反方交锋 |
| 🃏 德州扑克 | 2-6 人 | main | 经典德扑，盲注博弈 |
| 🐺 狼人杀 | 7 人 | experimental | 社交推理，狼人暗杀 |
| 🃏 二十一点 | 1-6 人 | main | 庄家对赌，策略博弈 |

## 特性

- **AI 对手** — 7 种行为风格（默认/激进/保守/创意/狡猾/理性/混乱）
- **LLM 策略评价** — 对局结束后 AI 自动生成策略复盘
- **多人社交** — 邀请码、观战模式、玩家事件广播
- **社区系统** — 排行榜、成就系统、个人主页
- **全量回放** — 每局完整历史，支持分享

## 配置

复制 `.env.example` 为 `.env`，按需修改：

```env
DATABASE_URL=sqlite:///app/data/dev.db
JWT_SECRET=change-me-to-a-random-string
DEEPSEEK_API_KEY=sk-xxx          # AI 功能必需
DEEPSEEK_BASE_URL=https://api.deepseek.com/v1
DEEPSEEK_MODEL=deepseek-chat
```

缺少 `DEEPSEEK_API_KEY` 时后端仍可启动，AI 相关功能不可用。

## 开发

### 依赖

- Rust 1.85+
- Node.js 22+
- SQLite

### 后端

```bash
cd backend
DATABASE_URL="sqlite://dev.db?mode=rwc" cargo run
```

### 前端

```bash
cd front_next
npm install
npm run dev
```

开发模式下 Vite 代理后端请求到 `localhost:8080`。

### 测试

```bash
make test-all       # 全部测试
make test-core      # 后端核心回归 (7 项)
make test-platform  # 平台核心
make test-ui        # UI 路径回归
make test-front     # 前端组件测试 (Vitest)
make check          # 编译检查零 warning
```

## 架构

```
front_next/          — React 18 + TypeScript + Vite
backend/             — Rust + Axum 0.8 + SQLx
platform_core/       — 游戏引擎核心（trait + 实现）
docs/                — 协议文档
```

### 生产部署

```bash
docker compose build backend   # 首次构建较慢
docker compose up -d
./scripts/backup.sh             # 备份数据
./scripts/restore.sh <file>     # 恢复数据
```

## 协议

WebSocket 协议详见 `docs/protocol.md`。
