# BurnCloud 设计系统(基础规范)

**版本**:2.0
**生效日期**:2026-04-11
**配套文档**:[principles.md](./principles.md)

> 这份文档是项目的**设计地基**。它定义"合格线" —— 满足这里列的硬性要求,你的设计不会翻车。
> [`principles.md`](./principles.md) 定义"卓越线" —— 决定"什么配存在"。
> **顺序:先守地基,再做判断。**

---

## 1. 技术栈

- **UI 框架**:Dioxus 0.7.2(Rust),组件用 RSX 编写,样式通过 `class:` 属性引用 Tailwind 工具类
- **样式栈**:Tailwind CSS + DaisyUI(预编译资产)
- **跨组件复用**:走 `client-shared`,不在页面里写一次性组件
- **运行模式**:Desktop(Win 11 Fluent)/ LiveView(Web)/ SSR

---

## 2. Design Tokens

**所有 Token 的权威定义和数值**:`crates/client/crates/client-shared/src/styles.rs`
**命名空间**:`--bc-*`

**分组速查**(只列存在的命名,数值以 `styles.rs` 为准):

| 类别 | Token 命名 |
| --- | --- |
| 品牌色 | `--bc-primary` / `-hover` / `-active` / `-light` / `-dark` |
| 语义色 | `--bc-success` / `-warning` / `-danger` / `-info`(各带 `-light`) |
| 中性背景 | `--bc-bg-canvas` / `-card` / `-card-solid` / `-elevated` / `-hover` / `-selected` / `-input` |
| 文字 | `--bc-text-primary` / `-secondary` / `-tertiary` / `-on-accent` / `-disabled` |
| 描边 | `--bc-border` / `-border-hover` / `-border-focus` |
| 字号 | `--bc-font-xs … 3xl`(8 级) |
| 间距 | `--bc-space-1 … 12`(基于 4px,9 级) |
| 圆角 | `--bc-radius-xs … 2xl` / `-full` |
| 阴影 | `--bc-shadow-xs … xl` / `-primary` |
| 动效 | `--bc-transition-fast` / `-normal` / `-slow` / `-spring` |
| 字体 | `--bc-font-family` |

**铁律**:

- ❌ `color: #007AFF` / `padding: 16px` / `border-radius: 8px`
- ✅ `color: var(--bc-primary)` / `padding: var(--bc-space-4)` / `border-radius: var(--bc-radius-sm)`
- 新增 Token 必须先进 `styles.rs`,再使用

---

## 3. 红线(违反即不可合入)

### R1 · i18n(项目级宪法)

UI 中**禁止硬编码任何语言字符串**。所有可见文案走 i18n 系统,中英双语为基线。
依据:[`docs/CONSTITUTION.md`](../CONSTITUTION.md) § 1.4。

### R2 · Token

不得硬编码颜色 / 字号 / 间距 / 圆角 / 阴影 / 动效。一律走 `--bc-*`。

### R3 · 组件来源

不得在页面里写一次性按钮 / 卡片 / 输入框。新组件先进 `client-shared`,补四态,再被使用。

### R4 · 语义化

不得使用 `<div onclick>` 模拟交互。用 `<button>` / `<a>` / `<input>`,否则键盘和屏幕阅读器全瞎。

---

## 4. Merge 前 Checklist(唯一规范出口)

提交 PR 前自查每一项。**这就是 system.md 的硬性要求 —— 没有第二份。**

**Token 与代码**

- [ ] 没有硬编码颜色 / 字号 / 间距 / 圆角 / 阴影,全部走 `--bc-*`
- [ ] 没有自造一次性组件,新组件已进 `client-shared`
- [ ] 所有 UI 文案走 i18n,无硬编码字符串(中英双语齐全)
- [ ] 所有交互元素是语义化标签,不是 `<div onclick>`

**视觉**

- [ ] 单页面字号层级 ≤ 5 级
- [ ] 文字对比度 ≥ WCAG AA(正文 4.5:1,大字 3:1)
- [ ] 图标尺寸成体系(16/20/24 三选其一),风格不混用

**交互**

- [ ] 可交互元素五态齐全:default / hover / **focus(键盘可见)** / active / disabled
- [ ] 异步视图四状态齐全:loading / success / **empty(有引导)** / **error(可重试)**
- [ ] 主操作在首屏可见,不藏在折叠菜单后
- [ ] 破坏性操作可撤销 OR 二次确认
- [ ] 错误提示包含"发生了什么 + 该做什么",不只是错误码

**体验**

- [ ] 关键路径 ≤ 3 次用户操作(点击 / 输入 / 提交)
- [ ] Desktop(≥ 1024px)/ Tablet(768–1023)/ Mobile(< 768)三端各自验证过,不是缩放
- [ ] 所有交互 Tab 可达,顺序合理
- [ ] 不仅靠颜色传达信息(error 必配图标或文字)
- [ ] 首屏 ≤ 2s(本地开发环境冷启动)、动画 60fps、点击响应 ≤ 100ms
- [ ] 图片有显式宽高(防 CLS)

**情感**

- [ ] 动效统一使用 `--bc-transition-*`,不自写 `cubic-bezier`
- [ ] 没有装饰性动画(动效只为解释变化,不为炫技)

**最后一项**

- [ ] **已经走过一遍 [`principles.md`](./principles.md) 的四个筛子**

> 前面 N 项让你**及格**,最后一项让你**不平庸**。

---

## 5. 反例速查

```rust
// ❌ 硬编码 Token
div { style: "color: #007AFF; padding: 16px;" }
// ✅
div { class: "text-[var(--bc-primary)] p-[var(--bc-space-4)]" }

// ❌ 一次性组件 + 缺 focus 态
div { onclick: handle, "提交" }
// ✅ 语义化 + 来自 client-shared
Button { variant: Primary, on_click: handle, "提交" }

// ❌ 硬编码文案
button { "Submit" }
// ✅ 走 i18n
button { {t!("common.submit")} }

// ❌ 只有错误码
"Error: 500"
// ✅ 发生了什么 + 该做什么
"网络请求失败,请检查连接后重试"
```
