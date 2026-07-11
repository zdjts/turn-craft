# Turn Craft — 大厅视图重构规格

> 目标: 支持 `REGISTRY.all_games()` 动态驱动 N 个游戏的浏览/配置/快速开始，零硬编码。
> 范围: `lobby.rs` / `lobby.css` / `registry.rs`，其余文件不变。

---

## 一、状态模型

### 1.1 新增枚举

```rust
#[derive(Clone, PartialEq)]
enum LobbyMode {
    /// 视图 A — 游戏网格 + 房间快览
    Browse,
    /// 视图 B — 单游戏配置面板
    Config { game_type: String },
}
```

### 1.2 新增信号

| 信号 | 类型 | 初始值 | 说明 |
|------|------|--------|------|
| `mode` | `Signal<LobbyMode>` | `LobbyMode::Browse` | 主视图切换 |
| `selected_game` | `Signal<Option<String>>` | `None` | 当前高亮游戏。Browse 下控制卡片展开，Config 下不变 |

### 1.3 已有信号（不变）

`selected_game_type` / `role_config` / `my_role` / `max_round` / `game_config` / `is_public` / `public_rooms` / `loading_public` / `creating` 全部保留，只在 Config 视图使用。

---

## 二、视图 A: `GameBrowseView`

### 2.1 布局结构

```
┌───────────────────────────────────────┬──────────┐
│  .pg-lobby-banner                     │          │
│  h1 "欢迎来到 Turn Craft"             │          │
│  p 副标题                              │          │
├───────────────────────────────────────┤ .pg-lobby│
│  .pg-lobby-games                      │ -sidebar │
│                                       │          │
│  .pg-lobby-game-card × N              │  h3      │
│  (for def in REGISTRY.all_games())    │  房间列表 │
│                                       │          │
│  ┌─────────────────────┐              │  .pg-    │
│  │ .game-card-icon      │              │  lobby-  │
│  │ .game-card-name      │              │  room-   │
│  │ .game-card-desc      │              │  card × M│
│  │ .game-card-meta      │  ← 基础信息   │          │
│  │                      │              │  按      │
│  │ .is-selected 时展开   │              │  selected│
│  │ .game-card-actions   │              │  _game   │
│  │  [快速开始] [自定义▸] │  ← 操作入口   │  过滤    │
│  └─────────────────────┘              │          │
│                                       │          │
└───────────────────────────────────────┴──────────┘
```

### 2.2 组件 Props

```rust
#[derive(Props, Clone, PartialEq)]
struct GameBrowseViewProps {
    selected_game: Signal<Option<String>>,
    on_select: Callback<String>,
    on_quick_start: Callback<String>,       // 默认参数直接进房
    on_enter_config: Callback<String>,       // 进入视图 B
    rooms: ReadOnlySignal<Vec<RoomSnapshotData>>,
    room_filter: ReadOnlySignal<Option<String>>,  // 过滤条件
    loading_public: ReadOnlySignal<bool>,
    load_rooms: Callback<()>,
}
```

### 2.3 交互行为

| 操作 | 触发 | 效果 |
|------|------|------|
| 点击未选中的卡片 | `on_select(gt)` | `selected_game` := Some(gt)，卡片展开显示按钮 |
| 点击已选中的卡片 | `on_select(gt)` | `selected_game` := None，卡片收起 |
| 点击另一张卡片 | `on_select(gt2)` | 旧卡片收起 → 新卡片展开 |
| 点击 [快速开始] | `on_quick_start(gt)` | 读取 `default_config()` → `create_room()` → 进入对局 |
| 点击 [自定义 ▸] | `on_enter_config(gt)` | `mode` := Config{game_type: gt}，进入视图 B |

### 2.4 快速开始流程

```
on_quick_start(game_type) {
    1. let def = REGISTRY.get(&game_type)  // 取注册信息
    2. let default_cfg = (def.default_config)()  // 取默认配置
    3. role_config    := default_cfg.role_config
    4. my_role        := default_cfg.my_role
    5. max_round      := default_cfg.max_round
    6. game_config    := default_cfg.game_config
    7. is_public      := true
    8. slots          := (def.generate_slots)(&role_config)
    9. req            := CreateRoomRequest { ... }  // 组合全部默认值
    10. create_room(&req).await → nav.push(Game{})
}
```

中间不跳转视图、不弹 toast（除非网络错误），所见即所得。

### 2.5 右侧房间过滤

```rust
let filtered_rooms = use_memo(move || {
    let filter = selected_game();    // Option<String>
    let all = public_rooms();
    match filter {
        Some(ref gt) => all.iter().filter(|r| r.game_type == *gt).cloned().collect(),
        None => all,
    }
});
```

选中德州 → 只显示 `game_type == "texas_holdem"` 的房间。选中 None → 显示全部。

---

## 三、视图 B: `GameConfigView`

### 3.1 布局结构

```
┌───────────────────────────────────────┬──────────┐
│  ← 返回游戏列表                        │          │
│                                       │          │
│  .pg-lobby-config                     │ 房间列表  │
│  ┌─────────────────────────────┐      │ (保留,   │
│  │ h3 ⚙️ 配置: {game_name}      │      │  同视图A)│
│  │                              │      │          │
│  │ .pg-lobby-config-public      │      │          │
│  │  ☑ 公开房间                  │      │          │
│  │                              │      │          │
│  │ DynamicLobbyCard             │      │          │
│  │  (游戏自定义配置表单)          │      │          │
│  │                              │      │          │
│  │ [🏟️ 创建房间并进入]           │      │          │
│  └─────────────────────────────┘      │          │
└───────────────────────────────────────┴──────────┘
```

### 3.2 组件 Props

```rust
#[derive(Props, Clone, PartialEq)]
struct GameConfigViewProps {
    game_type: String,                      // 当前配置的游戏
    on_back: Callback<()>,                  // 返回浏览视图
    role_config: Signal<HashMap<String, String>>,
    my_role: Signal<String>,
    max_round: Signal<usize>,
    game_config: Signal<Option<Value>>,
    is_public: Signal<bool>,
    creating: ReadOnlySignal<bool>,
    on_create: Callback<()>,                // 复用现有 handle_create_room 逻辑
}
```

### 3.3 交互

| 操作 | 效果 |
|------|------|
| ← 返回游戏列表 | `mode` := Browse，`selected_game` 保留不清空 |
| 配置表单修改 | 通过 `DynamicLobbyCard` 写入 `role_config` / `game_config` 等信号 |
| 公开房间 toggle | 写入 `is_public` |
| 创建房间 | 拼装 `CreateRoomRequest` → `create_room()` → 导航 |

### 3.4 初始进入时的处理

从 Browse 进入 Config 时，如果 `role_config` / `my_role` 等信号尚未按选中游戏初始化，需要先调用 `select_game(gt)`（复用现有逻辑，该函数已做初始化）：

```rust
// 已有，无需修改
let mut select_game = move |gt: &str| {
    selected_game_type.set(gt.to_string());
    if let Some(def) = REGISTRY.get(gt) {
        let default_cfg = (def.default_config)();
        role_config.set(default_cfg.role_config);
        my_role.set(default_cfg.my_role);
        max_round.set(default_cfg.max_round);
        game_config.set(default_cfg.game_config);
    }
};
```

在 `on_enter_config` 和 `on_quick_start` 中都需要先调 `select_game(gt)`。

---

## 四、CSS 结构

### 4.1 游戏卡片（选中展开前）

```css
.pg-lobby-games {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 20px;
}

.pg-lobby-game-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 28px 20px;
    border-radius: 16px;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    cursor: pointer;
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.pg-lobby-game-card:hover {
    border-color: rgba(16, 185, 129, 0.3);
    transform: translateY(-3px);
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.2);
}

.pg-lobby-game-card.is-selected {
    border-color: var(--accent);
    background: linear-gradient(
        135deg,
        rgba(16, 185, 129, 0.08) 0%,
        rgba(16, 185, 129, 0.02) 100%
    );
    box-shadow: 0 0 25px rgba(16, 185, 129, 0.15);
}
```

### 4.2 卡片展开区（选中时动画出现）

```css
.pg-lobby-game-card-actions {
    display: grid;
    grid-template-rows: 0fr;
    overflow: hidden;
    transition: grid-template-rows 0.25s ease-out;
    width: 100%;
    margin-top: 4px;
}

.pg-lobby-game-card.is-selected .pg-lobby-game-card-actions {
    grid-template-rows: 1fr;
}

.pg-lobby-game-card-actions-inner {
    min-height: 0;
    display: flex;
    gap: 10px;
    justify-content: center;
    padding-top: 12px;
    border-top: 1px solid var(--border-dim);
}
```

展开内部两个按钮：

```css
.pg-lobby-quick-start {
    padding: 10px 24px;
    border-radius: 8px;
    background: linear-gradient(135deg, var(--accent), #059669);
    color: #fff;
    font-size: 14px;
    font-weight: 700;
    border: none;
    cursor: pointer;
}

.pg-lobby-config-toggle {
    padding: 10px 20px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
}
```

### 4.3 页面描述 + 人数 meta

```css
.pg-lobby-game-card-desc {
    font-size: 13px;
    color: var(--text-secondary);
    text-align: center;
    line-height: 1.5;
}

.pg-lobby-game-card-meta {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 600;
}
```

### 4.4 左侧整体布局

```css
.pg-lobby-layout {
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: 32px;
    align-items: start;
}

@media (max-width: 1024px) {
    .pg-lobby-layout {
        grid-template-columns: 1fr;
    }
}
```

### 4.5 配置面板布局（视图 B）

```css
.pg-lobby-config-panel {
    padding: 28px;
    border-radius: 16px;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.pg-lobby-config-back {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    align-self: flex-start;
    transition: color 0.15s ease;
}

.pg-lobby-config-back:hover {
    color: var(--accent);
}
```

---

## 五、`GameUIDefinition` 扩展

```rust
// registry.rs
pub struct GameUIDefinition {
    // 现有字段（不变）
    pub game_type: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub lobby_card: fn(GameConfigProps) -> Element,
    pub game_component: fn(GamePluginProps) -> Element,
    pub default_config: fn() -> DefaultGameConfig,
    pub generate_slots: fn(&HashMap<String, String>) -> Vec<String>,

    // 新增字段
    pub description: &'static str,    // 一句话简介，如 "3人对战 · 法官主导"
    pub min_players: usize,          // 最少玩家数
    pub max_players: usize,          // 最多玩家数
}
```

三个已有游戏的描述注册：

```rust
// lincoln
description: "经典英式辩论 · 法官裁判 · 正反方交锋",
min_players: 3,
max_players: 3,

// texas_holdem
description: "2-6 人经典德扑 · 盲注博弈 · 心理对抗",
min_players: 2,
max_players: 6,

// werewolf
description: "7 人社交推理 · 狼人暗杀 · 好人投票",
min_players: 7,
max_players: 7,
```

---

## 六、实现检查清单

- [ ] `registry.rs`: `GameUIDefinition` 加 3 个字段，3 个游戏补全描述
- [ ] `lobby.rs`: 新增 `LobbyMode` 枚举
- [ ] `lobby.rs`: 新增 `mode: Signal<LobbyMode>`, `selected_game: Signal<Option<String>>`
- [ ] `lobby.rs`: 新增 `GameBrowseView` 子组件（视图 A）
- [ ] `lobby.rs`: 新增 `GameConfigView` 子组件（视图 B）
- [ ] `lobby.rs`: `Lobby` 顶层改为 `match mode()` 分发两个视图
- [ ] `lobby.rs`: 新增 `quick_create` 快速开始函数
- [ ] `lobby.rs`: `handle_create_room` 保持不变，从 Config 视图调用
- [ ] `lobby.rs`: 右侧房间列表加 `use_memo` 过滤逻辑
- [ ] `lobby.css`: `.-lobby-game-card` + `.is-selected` + 展开动画
- [ ] `lobby.css`: `.pg-lobby-quick-start` / `.pg-lobby-config-toggle` 按钮样式
- [ ] `lobby.css`: `.pg-lobby-config-back` / `.pg-lobby-config-panel` 视图 B 样式
- [ ] `lobby.css`: 删除旧的 `.game-carousel-wrapper` / `.section-card h3`（不再需要）
- [ ] 删除旧的 `selected_game_type` 信号或改为内部使用（Config 视图需要）

---

## 七、不在此次变更范围内

- `DynamicLobbyCard` — 原样复用
- `handle_create_room` 核心逻辑 — 原样复用
- `public_rooms` / `loading_public` / `creating` 信号 — 原样复用
- `game.css` / `game.rs` — 不影响对局页
- 后端 `create_room` API — 不影响
- 旧类名 `.game-carousel-wrapper` / `.game-select-card` — 暂不删除（保留兼容）
