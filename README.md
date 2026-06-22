# Turn-Craft

Turn-Craft 是一个基于 Rust 开发的**大模型多智能体 (Multi-Agent)** 驱动的多人回合制桌游/辩论平台。

它提供了一套基于 WebSocket 和有限状态机 (FSM) 的实时游戏引擎框架，允许人类玩家与多个配置了不同性格和策略的 AI 玩家同台同局竞技、推理或辩论。

## 🎮 当前包含的游戏

目前平台已原生集成了以下三种不同类型的对局：

1. **林肯-道格拉斯辩论 (Lincoln-Douglas Debate)**
   - 结构化的 1v1 辩论赛，包含立论、质询、驳论等多个阶段。
   - 可配置正反方为 AI 或人类，进行逻辑交锋。
2. **德州扑克 (Texas Hold'em)**
   - 经典的桌面扑克博弈游戏。
   - 展现 AI 对筹码控制、桌面信息阅读以及欺诈 (Bluffing) 的能力。
3. **狼人杀 (Werewolf)**
   - 7 人标准局（包含狼人、预言家、女巫、猎人、平民）。
   - 复杂的多阶段状态扭转（天黑请闭眼、查验、双药、自爆、白天发言与投票）。
   - 允许人类玩家扮演任意角色混入全 AI 局，或纯旁观 AI 之间的尔虞我诈。

## 🏗️ 架构与模块划分

项目采用全 Rust 技术栈 (Rust Backend + Rust Wasm Frontend) 进行构建，包含以下三个核心 Cargo Workspace：

* **`platform_core` (核心抽象引擎)**
  * 定义了通用的 `GameEngine` Trait。
  * 包含了各游戏的底层核心逻辑、状态机模型及 Action/Event 定义。
* **`backend` (后端服务)**
  * 基于 **Axum** 的高性能 Web 服务器。
  * **WebSocket 房间模型 (Actor Model)**：每个房间拥有独立的 Tokio 协程 Actor，负责处理多端高并发下的状态原子扭转，并向客户端广播状态快照。
  * **异步 AI 工作流 (AiWorker)**：后台常驻的 LLM 请求管线，解耦了缓慢的 AI 接口调用与高频的房间网络同步。
  * **持久化**：使用 **Sqlx (SQLite)** 将房间快照 (Snapshot) 和玩家状态落盘，随时可以断线重连或恢复历史对局。
* **`front` (前端 Web 客户端)**
  * 基于 **Dioxus** 编写的 Rust WebAssembly (Wasm) 前端。
  * 提供游戏大厅、房间创建、选座、全局 AI 提示词 (Prompt) 调整以及各游戏的沉浸式交互界面。

## 🚀 快速开始

### 环境依赖
- Rust (Latest stable)
- Dioxus CLI (`cargo install dioxus-cli` 或 `cargo binstall dioxus-cli`)

### 启动后端
```bash
cd backend

# 配置环境变量（可选，项目内有默认配置 fallback）
# cp .env.example .env

# 自动运行数据库迁移并启动后端服务器 (默认监听 8080)
cargo run
```

### 启动前端
```bash
cd front

# 使用 Dioxus 启动本地开发服务器
dx serve
```
启动后，在浏览器访问控制台输出的地址 (通常为 `http://localhost:8080` 对于 dx )。

## ⚙️ AI 配置 (Prompt Engineering)

对于每一局游戏，你可以在创建房间界面为每个 AI 席位独立配置大模型参数，包括：
- **API Key** 与 **Base URL** (兼容 OpenAI 接口格式的任何模型服务，如 DeepSeek, GPT-4, Claude 代理等)
- **模型名称 (Model)** 
- **系统提示词 (System Prompt)**：系统已为不同角色（如狼人、预言家）预设了基础 Prompt，你可以自行覆盖，比如给某个狼人 AI 添加“性格暴躁，喜欢跳预言家”的隐藏人设。

## 🗺️ 未来规划 (Roadmap)

我们正在计划向更加数据驱动 (Data-driven) 的方向演进，未来的目标包括：
* **JSON 驱动的通用状态机 (JSON-driven FSM)**：将当前硬编码 (Hardcode) 在 Rust 里的阶段扭转、技能逻辑完全抽离为 JSON 配置文件。届时实现“血染钟楼”、“阿瓦隆”或“谁是卧底”等新游戏，将无需再编写后端 Rust 逻辑，只需配置游戏规则与动作即可无缝接入 AI 引擎。
