# Turn Craft — CSS 类名规范与样式组织约定

> 版本: v1.0  
> 适用范围: `front/assets/` 全部 CSS 文件及组件中的 `class:` 属性  
> 本规范从"支持无限游戏类型扩展"出发，目标是让新游戏的 UI 开发者不必阅读已有代码就能写出风格一致的样式。

---

## 一、核心原则

1. **可预测**: 看到类名能推断元素所属的页面/组件和它的角色
2. **可隔离**: 游戏之间的样式不会互相污染
3. **可复用**: 通用模式只定义一次，所有游戏引用同一套类名
4. **不破坏现有功能**: 迁移按文件逐步进行，遗留类名保留至对应页面完成迁移

---

## 二、类名分层体系

类名分为三级，从通用到专用：

```
┌──────────────────────────────────────────────┐
│ L1  全局样式 (Global)                        │
│     .g-*       全局通用组件/工具类             │
│     e.g. .g-btn-primary, .g-card, .g-spinner │
├──────────────────────────────────────────────┤
│ L2  页面样式 (Page)                           │
│     .pg-*      页面独有的布局和组件             │
│     e.g. .pg-login-card, .pg-lobby-layout    │
├──────────────────────────────────────────────┤
│ L3  游戏样式 (Game)                           │
│     .gm-*      游戏引擎组件 (所有游戏共享)      │
│     .{game}-*  游戏私有样式                    │
│     e.g. .gm-timeline, .poker-card, .wl-status│
└──────────────────────────────────────────────┘
```

### 2.1 L1 — 全局样式 (`.g-*`)

定义在 `main.css` 中，全局唯一，任何页面和游戏都可以直接使用。

**包含内容**:

| 类别 | 前缀 | 示例 | 说明 |
|------|------|------|------|
| 按钮 | `.g-btn-{variant}` | `.g-btn-primary`, `.g-btn-danger`, `.g-btn-ghost` | 主按钮、危险按钮、幽灵按钮 |
| 卡片容器 | `.g-card` | `.g-card` | 玻璃拟态卡片 |
| 表单字段 | `.g-field` | `.g-field`, `.g-field-label`, `.g-field-input` | 统一的表单字段 |
| 状态指示 | `.g-*` | `.g-spinner`, `.g-skeleton`, `.g-empty` | loading/空状态/错误 |
| 标签/徽章 | `.g-badge-{variant}` | `.g-badge-success`, `.g-badge-danger`, `.g-badge-info` | 角色标签、状态标签 |
| 布局 | `.g-*` | `.g-grid-2col`, `.g-grid-3col` | 通用网格布局 |
| 主题 | `:root` 变量 | `--color-accent`, `--color-bg-deep` 等 | 不在此规范范围内，保持现有变量命名 |

**规则**:
- `.g-*` 类名**只在 `main.css` 中定义**，禁止在其他文件中重写
- 全局 CSS 变量（主题）继续使用现有命名（`--bg-*`, `--text-*`, `--accent` 等），迁移时只改类名不改变量

**现有类名迁移目标**:

| 现有类名 | 目标类名 | 优先级 |
|---------|---------|--------|
| `.glass-panel` | `.g-card` | P0 |
| `.glass-panel-subtle` | `.g-card-subtle` | P0 |
| `.form-field` | `.g-field` | P1 |
| `.form-field label` | `.g-field-label` | P1 |
| `.form-field input` | `.g-field-input` | P1 |
| `.spinner` | `.g-spinner` | P0 |
| `.skeleton-item` | `.g-skeleton-row` | P1 |
| `.skeleton-card-grid` | `.g-skeleton-card` | P1 |
| `.empty-state-card` | `.g-empty` | P1 |
| `.styled-checkbox` | `.g-toggle` | P1 |
| `.role-badge.human` | `.g-badge-success` | P1 |
| `.role-badge.ai` | `.g-badge-info` | P1 |

### 2.2 L2 — 页面样式 (`.pg-*`)

每个顶级路由页面使用独立的前缀。定义在各页面对应的 CSS 文件中。

**页面前缀分配**:

| 页面 | 路由 | CSS 文件 | 前缀 | 示例 |
|------|------|---------|------|------|
| 登录 | `/login` | `login.css` | `.pg-login-*` | `.pg-login-card`, `.pg-login-form` |
| 大厅 | `/` | `lobby.css` | `.pg-lobby-*` | `.pg-lobby-banner`, `.pg-lobby-layout` |
| 对局壳 | `/game/:id` | `game.css` | `.pg-arena-*` | `.pg-arena-shell`, `.pg-arena-sidebar` |
| AI 配置 | `/settings/:id` | `settings.css` | `.pg-settings-*` | `.pg-settings-form` |
| 历史 | `/history` | `history.css` | `.pg-history-*` | `.pg-history-card`, `.pg-history-list` |
| 公开房间 | `/public` | `public.css` | `.pg-public-*` | `.pg-public-grid`, `.pg-public-card` |
| 个人主页 | `/profile` | `profile.css` | `.pg-profile-*` | `.pg-profile-stats` |
| 回放 | `/replay/:id` | `replay.css` | `.pg-replay-*` | `.pg-replay-meta` |
| 关于 | `/about` | `about.css` | `.pg-about-*` | `.pg-about-card` |

**规则**:
- `.pg-*` 前缀的类名**只在对应页面 CSS 文件中定义**
- 页面可以引用 L1 全局样式（`.g-card`, `.g-btn-primary` 等），但不能定义或覆盖它们
- 页面专用的子组件（如 history 的删除确认弹窗）也使用该页面的前缀（`.pg-history-modal`）

**现有类名迁移目标**:

| 页面 | 现有类名 | 目标类名 |
|------|---------|---------|
| login | `.login-container` | `.pg-login-container` |
| login | `.login-card` | `.pg-login-card` |
| login | `.login-form` | `.pg-login-form` |
| login | `.login-header` | `.pg-login-header` |
| login | `.login-submit-btn` | `.pg-login-submit` + `.g-btn-primary` |
| login | `.login-error-bubble` | `.pg-login-error` |
| lobby | `.lobby-container` | `.pg-lobby` |
| lobby | `.lobby-header-banner` | `.pg-lobby-banner` |
| lobby | `.lobby-layout` | `.pg-lobby-layout` |
| lobby | `.lobby-left-col` | `.pg-lobby-left` |
| lobby | `.lobby-right-col` | `.pg-lobby-right` |
| lobby | `.game-carousel-wrapper` | `.pg-lobby-games` |
| lobby | `.game-select-card` | `.pg-lobby-game-card` |
| lobby | `.game-select-card.active` | `.pg-lobby-game-card.is-active` |
| lobby | `.game-config-form` | `.pg-lobby-config` |
| lobby | `.create-room-btn` | `.pg-lobby-create` + `.g-btn-primary` |
| lobby | `.public-rooms-list` | `.pg-lobby-rooms` |
| lobby | `.public-room-card` | `.pg-lobby-room-card` |
| arena | `.arena-shell` | `.pg-arena` |
| arena | `.arena-sidebar` | `.pg-arena-sidebar` |
| arena | `.arena-viewport` | `.pg-arena-viewport` |
| arena | `.arena-room-card` | `.pg-arena-info` |
| arena | `.arena-roster-section` | `.pg-arena-roster` |
| arena | `.sidebar-bottom-controls` | `.pg-arena-controls` |
| arena | `.leave-arena-btn` | `.pg-arena-leave` + `.g-btn-danger` |
| arena | `.loading-canvas` | `.pg-arena-loading` |
| history | `.history-container` | `.pg-history` |
| history | `.history-card` | `.pg-history-card` |
| history | `.toggle-switch` | `.pg-history-toggle` |
| history | `.action-btn` | `.pg-history-action` + `.g-btn-ghost` |
| history | `.modal-overlay` | `.pg-history-modal-overlay` |
| history | `.modal-confirm` | `.pg-history-modal` |
| public | `.public-rooms-page` | `.pg-public` |
| public | `.public-rooms-grid` | `.pg-public-grid` |
| public | `.public-grid-card` | `.pg-public-card` |
| public | `.grid-join-btn` | `.pg-public-join` |
| profile | `.profile-container` | `.pg-profile` |
| profile | `.profile-grid-top` | `.pg-profile-header` |
| profile | `.profile-user-card` | `.pg-profile-user` |
| profile | `.profile-stats-card` | `.pg-profile-stats` |
| replay | `.replay-container` | `.pg-replay` |
| replay | `.replay-details-layout` | `.pg-replay-detail` |
| about | `.about-container` | `.pg-about` |
| about | `.about-grid` | `.pg-about-grid` |
| about | `.about-card` | `.pg-about-card` |

### 2.3 L3 — 游戏样式 (`.gm-*` / `.{game}-*`)

**又分为两层**：

#### 2.3a 游戏通用组件 (`.gm-*`)

定义在 `game.css` 中，所有游戏**必须**使用的公共 UI 模式。这些是 GameEngine 的视觉契约。

| 组件 | 类名 | 说明 |
|------|------|------|
| 时间轴/历史流 | `.gm-timeline`, `.gm-timeline-item`, `.gm-timeline-meta`, `.gm-timeline-content` | 替代现有 `.bubble-row`/`.bubble-body` |
| 动作输入区域 | `.gm-action-bar`, `.gm-action-input`, `.gm-action-submit` | 替代现有 `.action-console`/`.console-*` |
| 玩家名册 | `.gm-roster`, `.gm-roster-player`, `.gm-roster-player.is-active` | 统一玩家展示 |
| 回合/阶段指示 | `.gm-phase`, `.gm-phase-round` | 替代 `.timeline-round`/`.phase-text` |
| 流式输出气泡 | `.gm-streaming`, `.gm-streaming-text`, `.gm-streaming-cursor` | 替代 `.streaming-bubble` |
| AI 内容切换 | `.gm-ai-toggle` | 替代 `.toggle-ai-btn` 内联按钮 |
| 加载中 | `.gm-loading` | 游戏加载中状态 |

**规则**:
- `.gm-*` 类名**只在 `game.css` 中定义**
- 游戏开发者必须使用 `.gm-*` 而不是重新发明等价组件
- 如果需要的组件还不存在，应先在 `game.css` 补充 `.gm-*` 定义，再在游戏中使用

**现有类名迁移目标**:

| 现有类名 | 目标类名 |
|---------|---------|
| `.bubble-row` | `.gm-timeline-item` |
| `.bubble-avatar` | `.gm-timeline-avatar` |
| `.bubble-body` | `.gm-timeline-body` |
| `.bubble-meta` | `.gm-timeline-meta` |
| `.bubble-name` | `.gm-timeline-author` |
| `.bubble-tag` | `.gm-timeline-tag` |
| `.bubble-content` | `.gm-timeline-content` |
| `.action-console` | `.gm-action-bar` |
| `.console-row` | 删掉，flex 布局内联 |
| `.console-textarea` | `.gm-action-input` |
| `.console-submit` | `.gm-action-submit` |
| `.console-hint` | `.gm-action-hint` |
| `.timeline-header` | `.gm-phase` |
| `.timeline-title` | `.gm-phase-title` |
| `.timeline-round` | `.gm-phase-round` |
| `.streaming-bubble` | `.gm-streaming` |
| `.streaming-indicator` | `.gm-streaming-indicator` |
| `.cursor-blink` | `.gm-streaming-cursor` |
| `.toggle-ai-btn` | `.gm-ai-toggle` |
| `.timeline-empty` | `.gm-empty` |
| `.timeline-syncing` | `.gm-syncing` |

#### 2.3b 游戏私有样式 (`.{game}-*`)

每个游戏一个独立 CSS 文件，使用该游戏的短名作为前缀。

**游戏前缀分配**:

| 游戏 | game_type | CSS 文件 | 前缀 | 示例 |
|------|-----------|---------|------|------|
| 林肯辩论 | `lincoln` | `lincoln.css` | `.ln-*` | `.ln-speech`, `.ln-judge-ruling` |
| 德州扑克 | `texas_holdem` | `poker.css` (保持不变) | `.poker-*` | `.poker-table`, `.poker-card` |
| 狼人杀 | `werewolf` | `werewolf.css` | `.wl-*` | `.wl-player`, `.wl-vote-target` |

**规则**:
- `.{game}-*` 类名**只在对应游戏 CSS 文件中定义**
- 禁止跨游戏引用另一个游戏的私有类名
- 游戏可以引用 L1 全局样式和 L2/L3 通用组件（`.g-card`, `.gm-timeline`）
- 新游戏注册时，在 `registry.rs` 中声明其短名前缀，作为开发文档的一部分

**现有类名迁移目标** (poker.css 已有独立前缀，只需微调):

| 现有类名 | 目标类名 |
|---------|---------|
| `.player-seat` | `.poker-seat` |
| `.player-info` | `.poker-player-info` |
| `.player-chips` | `.poker-player-chips` |
| `.player-bet` | `.poker-player-bet` |
| `.player-hand` | `.poker-player-hand` |
| `.player-status` | `.poker-player-status` |
| `.community-cards-area` | `.poker-community` |
| `.pot-area` | `.poker-pot` |
| `.action-area` | `.poker-actions` |
| `.action-buttons` | `.poker-actions-row` |
| `.btn-action` | `.poker-btn` |

---

## 三、状态修饰符规范

使用 `.is-{state}` 命名（参考 BEM 的 modifier 惯例），而不是语义化的 `.active`/`.disabled`：

```
正确:
  .g-btn-primary.is-loading
  .pg-lobby-game-card.is-active
  .gm-timeline-item.is-streaming
  .poker-seat.is-folded
  .poker-seat.is-all-in

错误:
  .g-btn-primary.loading         ← 与 g-btn-primary 不在同一命名空间
  .pg-lobby-game-card.active     ← 可能与全局 .active 冲突
```

现有 `.active`, `.selected`, `.folded` 等需逐文件迁移。

---

## 四、CSS 文件组织

```
front/assets/
├── main.css           # L1 全局样式 + CSS 变量 + Reset + Scrollbar
├── login.css          # L2 .pg-login-*
├── lobby.css          # L2 .pg-lobby-*
├── game.css           # L2 .pg-arena-* (壳层) + L3 .gm-* (游戏通用组件)
├── settings.css       # L2 .pg-settings-*
├── history.css        # L2 .pg-history-*
├── public.css         # L2 .pg-public-*
├── profile.css        # L2 .pg-profile-*
├── replay.css         # L2 .pg-replay-*
├── about.css          # L2 .pg-about-*
├── lincoln.css        # L3 .ln-* (新建，从 game.css 拆出)
├── poker.css          # L3 .poker-*
└── werewolf.css       # L3 .wl-* (新建，从 game.css 和 inline style 拆出)
```

**新建文件**:
- `lincoln.css`: 从 `game.css` 拆分林肯辩论专属样式（`.bubble-avatar.judge` → `.ln-avatar-judge` 等）
- `werewolf.css`: 从 `game.css` 和内联样式收集狼人杀专属样式

**关于 Tailwind**:
- 当前 `tailwind.css` 仅使用了 5 个 utility 类。**建议移除**，把这些类转为 `main.css` 中的 `.g-*` 工具类（如 `.g-text-center`, `.g-text-sm`, `.g-mb-4`）。
- 如果后续决定拥抱 Tailwind，需要调整 Dioxus 构建流程做按需编译（当前是完整的 263 行 CSS 打入 WASM bundle）。

---

## 五、命名速查表

### 5.1 通用状态词

| 状态 | 修饰符 |
|------|--------|
| 激活/选中 | `.is-active` |
| 禁用 | `.is-disabled` |
| 加载中 | `.is-loading` |
| 错误 | `.is-error` |
| 成功 | `.is-success` |
| 隐藏 | `.is-hidden` |
| 折叠 | `.is-collapsed` |
| 展开 | `.is-expanded` |

### 5.2 游戏状态词（L3 通用）

| 状态 | 修饰符 |
|------|--------|
| 回合中(轮到该玩家) | `.is-active` |
| 弃牌 | `.is-folded` |
| 全押/自爆 | `.is-all-in` |
| 阵亡/出局 | `.is-eliminated` |
| 流式生成中 | `.is-streaming` |

---

## 六、迁移路线图

### Phase 1: L1 全局 (预计 2-3 天)

1. 在 `main.css` 中定义所有 `.g-*` 类
2. 修改 `layout.rs` 中的侧边栏/toast/shell 引用
3. 标记旧类名 `#[deprecated]`（通过注释，Rust 无法标记 CSS 类名）
4. 保留旧类名作为 alias 直到 Phase 2 结束

### Phase 2: L2 页面 (预计 3-5 天)

按影响范围从小到大：
1. `about.css` → `.pg-about-*`（最简单，不涉及交互）
2. `profile.css` → `.pg-profile-*`
3. `replay.css` → `.pg-replay-*`
4. `settings.css` → `.pg-settings-*`
5. `login.css` → `.pg-login-*`
6. `history.css` → `.pg-history-*`
7. `public.css` → `.pg-public-*`
8. `lobby.css` → `.pg-lobby-*`（最复杂，改动最大）
9. `game.css` 壳层 → `.pg-arena-*`

每完成一个页面就删除该页面 CSS 文件中的旧类名。

### Phase 3: L3 游戏 (预计 5-7 天)

1. 在 `game.css` 中定义全部 `.gm-*` 通用组件
2. 从 `game.css` 拆分 `lincoln.css` + `werewolf.css`
3. 迁移 Lincoln 组件引用 (`.bubble-*` → `.gm-timeline-*`)
4. 迁移 Werewolf 组件引用 + 内联样式提取
5. Poker 类名前缀统一 (`.player-seat` → `.poker-seat` 等)
6. 更新 `registry.rs` 注册游戏短名前缀

### Phase 4: 清理 (预计 1 天)

1. 删除所有已迁移的旧类名
2. 删除 `tailwind.css` 或改为按需编译
3. 补充 Design Token 文档（`--game-*` 变量给游戏开发者用）
4. 将此规范文件移动到 `front/assets/STYLE_GUIDE.md`

---

## 七、新游戏接入清单

当开发者需要注册新游戏时，风格相关 checklist：

```
□ 在 front/assets/ 创建 <game>.css，使用 .{prefix}-* 前缀
□ 在 main.css 的 Design Token 段确认所需新变量（如果需要）
□ 使用 .gm-* 通用组件（action bar, timeline, roster）
   □ 如果 .gm-* 不满足需求，先在 game.css 补充通用组件
□ 使用 .g-* 全局样式（按钮、卡片、表单字段）
□ 在 game.css 引用 .gm-* 而不是重新定义等价样式
□ 不在组件中使用 style: "..." 内联样式（除动态计算值如 width）
□ 将 game_type → 短名 → CSS 文件的映射加到 registry.rs 注释
□ 所有交互状态使用 .is-{state} 修饰符
```
