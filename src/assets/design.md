# MCPDock 前端设计规范 (Design System)

本规范基于 MCPDock 仪表盘页面的初步设计提取，旨在为后续使用 Tauri + Vue + Naive UI 开发桌面端应用时，提供视觉和交互的一致性指导。

## 1. 设计原则 (Design Principles)

- **极简与克制 (Minimal & Restrained):** 减少不必要的装饰，通过留白和排版区分层级。
- **清晰高效 (Clear & Efficient):** 信息展示直观，操作路径简短。作为管理工具，效率优先。
- **一致性 (Consistency):** 在色彩、排版、组件行为上保持全局一致，降低用户学习成本。
- **现代感 (Modern feel):** 采用平滑的过渡动画和细微的阴影提升质感。

## 2. 色彩系统 (Color Palette)

系统色彩很大程度上参考了 Naive UI 的默认配色，并针对桌面端稍作调整。

### 品牌/主色调 (Brand / Primary)

主要用于主要按钮、激活状态、重要强调。

- **Primary (主色):** `#18a058` (Naive UI 默认绿)
- **Primary Hover (悬停):** `#36ad6a`
- **Primary Pressed (按下):** `#0c7a43`
- **Primary Light (浅主色/背景):** `#eaf5ef` (用于侧边栏激活项背景)

### 状态色 (Status)

用于表示不同状态或反馈。

- **Success (成功/运行中):** 沿用主色 `#18a058`
- **Warning (警告/已暂停):** `#f0a020`
- **Error (错误/异常):** `#d03050`
- **Info (信息/默认状态):** `#2080f0`
- **Inactive (未激活/已停止):** `#9ca3af` (Gray-400)

### 中性色/界面色 (Neutral / Surface)

用于文本、背景、边框。

- **Background (全局背景):** `#f3f4f6` (偏冷的浅灰，突出内容卡片)
- **Surface (卡片/面板背景):** `#ffffff` (纯白)
- **Border (边框/分割线):** `#efeff5`
- **Text Main (主要文本/标题):** `#333639`
- **Text Muted (次要文本/描述):** `#767c82`
- **Text Disabled (禁用文本):** `#c2c2c2`

## 3. 排版 (Typography)

- **字体家族 (Font Family):** 优先使用系统无衬线字体，保证跨平台性能和阅读体验。
  - `'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`
- **字号层级 (Font Sizes):**
  - 页面标题 (H1): `20px` (text-xl), `font-medium`
  - 卡片标题/区块标题 (H2/H3): `16px` (text-lg), `font-medium`
  - 统计大数字: `24px` (text-2xl), `font-semibold`
  - 正文/主内容 (Body): `14px` (text-sm), 常规字重
  - 辅助说明/标签 (Caption): `12px` (text-xs), `text-muted`

## 4. 布局与间距 (Layout & Spacing)

### 基础布局

- **结构:** 典型的 Admin 左右布局。
- **侧边栏 (Sidebar):** 展开宽度 `240px`，收起宽度 `64px`。
- **顶栏 (Header):** 高度 `64px` (或 `4rem`)。
- **主内容区 (Main Content):** 占满剩余空间，内部通常采用最大宽度限制（如 `max-w-6xl`）并居中，防止宽屏下内容过度拉伸。

### 间距 (Spacing)

采用 4px 的倍数作为基础间距系统 (Tailwind 默认标准)。

- **页面内边距 (Page Padding):** `24px` (p-6)
- **卡片内边距 (Card Padding):** `20px` (p-5) 或 `16px` (p-4)
- **组件间距 (Gap):** 常用 `16px` (gap-4) 或 `24px` (gap-6)

## 5. 核心组件样式指南 (Component Guidelines)

在实际开发中使用 Naive UI 时，可通过覆盖默认变量 (theme overrides) 来匹配此规范。

- **卡片 (Card):**
  - 背景: `#ffffff`
  - 边框: `1px solid #efeff5`
  - 圆角 (Border Radius): `8px` (rounded-lg)
  - 阴影: 默认轻微阴影 (`shadow-sm`)，悬停时增加阴影 (`hover:shadow-md`) 并配合 `transition-shadow`。
- **按钮 (Button):**
  - 圆角: `8px` (`rounded-lg`)，与全局卡片和交互组件保持一致。
  - 主按钮 (Primary): 背景使用主色，文本白色，无边框。
  - 次按钮/操作按钮 (Secondary/Tertiary): 背景透明，悬停时背景变浅灰 (`hover:bg-gray-100`)。
- **列表项 (List Item):**
  - 悬停效果: 整个列表项背景变为极浅灰色 (`hover:bg-gray-50`)，重要文字（如标题）可变为主色，提供清晰的交互反馈。
- **图标 (Icons):**
  - 推荐使用统一风格的线框图标库 (如 Lucide Icons 或 Feather Icons)。
  - 常规大小: `20px` (w-5 h-5) 或 `16px` (w-4 h-4)。
  - 图标应作为文本的辅助，保持视觉平衡。
- **滚动条 (Scrollbar):**
  - 采用细长、不显眼的自定义滚动条，悬停时略微变深。

## 6. 交互与动画 (Interaction & Animation)

- **侧边栏折叠:** 使用平滑的宽度过渡 (`transition: width 0.3s ease-in-out`)，内容配合淡入淡出 (`opacity`) 处理。
- **状态反馈:** 所有可点击元素（按钮、链接、列表项）都必须有 `Hover`（悬停）状态和 `Active`（按下）状态的视觉反馈。
- **动画持续时间:** 保持简短，通常在 `150ms - 300ms` 之间，避免拖沓。
