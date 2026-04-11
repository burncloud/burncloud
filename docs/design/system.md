# BurnCloud 设计系统(基础规范)

**版本**:1.0
**生效日期**:2026-04-11
**配套文档**:[principles.md](./principles.md)

## 序言

这份文档是 BurnCloud 项目的**设计地基**。任何 UI 变更在合入前都必须满足本文档列出的硬性要求。

它不是讨论稿,是规范。

它和 [`principles.md`](./principles.md) 的关系:

- **本文件定义"合格线"** —— 满足这些要求,你的设计不会翻车。
- **`principles.md` 定义"卓越线"** —— 在满足本文件之后,用那把砍刀决定"什么配存在"。

> 顺序:**先守地基,再做判断。** 地基一次立好,判断每次重做。

---

## 1. 技术栈

- **UI 框架**:Dioxus 0.7.2(Rust)
- **运行模式**:Desktop(Windows 11 Fluent)、LiveView(Web)、SSR
- **样式栈**:Tailwind CSS + DaisyUI(均为预编译资产)
- **Token 权威定义**:`crates/client/crates/client-shared/src/styles.rs`
- **预编译 CSS**:
  - `crates/client/src/assets/tailwind.css`
  - `crates/client/src/assets/daisyui.css`

> **铁律**:本文档下列任何 Token 的真实数值以 `styles.rs` 为准。本文档只引用,不定义。新增 Token 必须先进 `styles.rs`,再回来更新本文档。

---

## 2. Design Tokens

所有 Token 以 `--bc-*` 前缀命名,定义在 `styles.rs`。

### 2.1 颜色

**品牌色**

| Token | 值 | 用途 |
| --- | --- | --- |
| `--bc-primary` | `#007AFF` | 主操作、链接、焦点 |
| `--bc-primary-hover` | `#0077ED` | 悬停态 |
| `--bc-primary-active` | `#0066D6` | 按下态 |
| `--bc-primary-light` | `rgba(0,122,255,0.10)` | 浅底 / 标签背景 |
| `--bc-primary-dark` | `#5856D6` | 深色变体 |

**语义色**(每个都有 `-light` 浅底变体)

| Token | 值 | 用途 |
| --- | --- | --- |
| `--bc-success` | `#34C759` | 成功 |
| `--bc-warning` | `#FF9500` | 警告 |
| `--bc-danger` | `#FF3B30` | 错误 / 破坏性操作 |
| `--bc-info` | `#5AC8FA` | 信息 |

**中性色**

| 类别 | Token |
| --- | --- |
| 画布 | `--bc-bg-canvas` `#F5F5F7` |
| 卡片 | `--bc-bg-card` / `--bc-bg-card-solid` |
| 浮层 | `--bc-bg-elevated` |
| 悬停 / 选中 | `--bc-bg-hover` / `--bc-bg-selected` |
| 输入框 | `--bc-bg-input` |

**文字**:`--bc-text-primary` / `-secondary` / `-tertiary` / `-on-accent` / `-disabled`
**描边**:`--bc-border` / `-border-hover` / `-border-focus`

### 2.2 字号(8 级)

`--bc-font-xs`(10px)→ `sm`(12)→ `base`(14)→ `md`(15)→ `lg`(17)→ `xl`(20)→ `2xl`(28)→ `3xl`(40)

> **单页面字号层级 ≤ 5 级。** 超过就是层次失控。

### 2.3 间距(基于 4px,9 级)

`--bc-space-1`(4px)→ `2`(8)→ `3`(12)→ `4`(16)→ `5`(20)→ `6`(24)→ `8`(32)→ `10`(40)→ `12`(48)

### 2.4 圆角

`--bc-radius-xs`(4)→ `sm`(8)→ `md`(12)→ `lg`(16)→ `xl`(24)→ `2xl`(32)→ `full`(9999)

### 2.5 阴影

`--bc-shadow-xs` → `sm` → `md` → `lg` → `xl`,加一个品牌发光 `--bc-shadow-primary`。

### 2.6 动效

| Token | 时长 | 曲线 | 用途 |
| --- | --- | --- | --- |
| `--bc-transition-fast` | 150ms | ease | 微反馈(hover) |
| `--bc-transition-normal` | 250ms | ease | 默认 |
| `--bc-transition-slow` | 350ms | ease | 大块变化 |
| `--bc-transition-spring` | 500ms | spring | 进入 / 强调 |

### 2.7 字体

`--bc-font-family`:Apple System / Segoe UI 优先的跨平台无衬线栈。

### 2.8 Token 铁律

- ❌ 禁止硬编码颜色(`#xxxxxx` / `rgb(...)`)、字号(`14px`)、间距(`16px`)、圆角、阴影
- ✅ 必须用 `var(--bc-*)`
- 新增 Token 必须先进 `styles.rs`,经过 review,再使用

---

## 3. 视觉层面(硬性要求)

- **对齐**:同列同 baseline、同栏同左边界,禁止"差几个像素"
- **留白**:相邻区块之间至少 `--bc-space-6`(24px),禁止贴边
- **层次**:每个屏幕只能有 **一个** 视觉重心(主操作 / 主内容)
- **字号层级**:单页面 ≤ 5 级
- **对比度**:正文 ≥ 4.5:1,大字(≥ 18px 或 14px bold)≥ 3:1(WCAG AA)
- **图标**:统一字号成体系(16 / 20 / 24);风格统一(Lucide / SF Symbols 任选其一,不混用)

---

## 4. 交互层面(硬性要求)

### 4.1 组件四态必须齐全

任何可交互元素(按钮、链接、输入框、卡片)都必须实现:

- **default**(默认)
- **hover**(悬停)
- **focus**(键盘焦点,必须有 `--bc-border-focus` 描边或等价提示)
- **active**(按下)
- **disabled**(禁用,使用 `--bc-text-disabled`)

缺一不可。`focus` 态尤其不能省 —— 否则键盘用户全瞎。

### 4.2 状态反馈四件套

涉及异步数据的视图必须明确处理:

- **loading**(加载中)
- **success / 数据**(正常)
- **empty**(空状态,需有引导文案)
- **error**(错误,需可重试)

### 4.3 可发现性

- 主操作必须在**首屏可见**,禁止藏在折叠菜单 / 三点菜单后
- 同一功能在全应用只能有**一个标准入口**(可有快捷方式,但不能有"另一个主入口")

### 4.4 容错性

- 破坏性操作(删除、清空、退出)必须**可撤销** OR **二次确认**
- 错误提示必须包含:**发生了什么 + 用户该做什么**,禁止只显示 `Error: 500`

---

## 5. 体验层面(硬性要求)

### 5.1 信息架构

- 关键路径 ≤ 3 步(从入口到完成核心任务)
- 导航深度 ≤ 3 层

### 5.2 响应式

Dioxus Desktop / LiveView 三端各自最优,**禁止"缩放"式适配**:

- **Desktop**(≥ 1024px):多栏布局,信息密度高
- **Tablet**(768–1023px):单栏 / 双栏可切换
- **Mobile**(< 768px):单栏,主操作底部固定

### 5.3 无障碍(a11y)

- **键盘可达**:所有交互必须 Tab 可达,顺序合理
- **焦点可见**:焦点态必须有明显视觉提示
- **语义化**:用 `<button>` / `<a>` / `<input>`,不要用 `<div onclick>`
- **色盲友好**:不能仅靠颜色传达信息(error 必须配图标或文字)

### 5.4 性能预算

- **首屏**:≤ 2 秒
- **动画**:60 fps 下限
- **交互响应**:用户操作到视觉反馈 ≤ 100ms
- **图片**:必须压缩、必须有尺寸属性(防 CLS)

---

## 6. 系统化(硬性要求)

- **只用已有组件**,不要在页面里写一次性的按钮 / 卡片 / 输入框
- **新组件必须先进** `client-shared`,补四态、补文档,再被使用
- **不创建变体**:已有 `Button` 就不要再做 `MyButton`
- **i18n**:UI 不允许硬编码任何语言字符串(参见 `docs/CONSTITUTION.md` § 1.4)

---

## 7. 情感层面

### 7.1 品牌调性

- **基调**:Apple-inspired 克制感 + Windows 11 Fluent Design 透明 / 模糊
- **关键词**:专业、克制、高性能、值得信赖
- **不要**:卡通风、霓虹色、过度渐变、emoji 装饰

### 7.2 动效

- **时长**:150–350ms 之间,统一使用 `--bc-transition-*`
- **曲线**:统一,不要每个组件自己写 cubic-bezier
- **目的**:动效**只为解释变化**(从哪来、到哪去),不为炫技
- **禁止**:无意义的入场动画、装饰性 loading、视差滚动

---

## 8. Merge 前检查清单

提交 PR 前,自查每一项:

- [ ] 没有硬编码颜色 / 字号 / 间距 / 圆角 / 阴影,全部走 `--bc-*`
- [ ] 单页面字号层级 ≤ 5 级
- [ ] 文字对比度 ≥ WCAG AA
- [ ] 所有可交互元素有 default / hover / focus / active / disabled 五态
- [ ] focus 态键盘可见
- [ ] 异步视图有 loading / success / empty / error 四种状态
- [ ] 主操作在首屏可见
- [ ] 破坏性操作可撤销 OR 二次确认
- [ ] 错误提示包含"发生了什么 + 该做什么"
- [ ] 关键路径 ≤ 3 步
- [ ] Desktop / Tablet / Mobile 三端各自验证过
- [ ] 所有交互 Tab 可达,顺序合理
- [ ] 没有 `<div onclick>`,用语义化标签
- [ ] 不仅靠颜色传达信息
- [ ] 首屏 ≤ 2s,动画 60fps,交互延迟 ≤ 100ms
- [ ] 没有自造一次性组件,新组件已进 `client-shared`
- [ ] 没有硬编码任何语言字符串
- [ ] 动效统一用 `--bc-transition-*`
- [ ] **已经走过一遍 [`principles.md`](./principles.md) 的四个筛子**

> 最后一项最重要。前面 17 项让你**及格**,最后 1 项让你**不平庸**。
