# BurnCloud 设计系统(基础规范)

**版本**:3.0
**生效日期**:2026-04-11

> 这份文档是项目的设计地基 —— 满足这里列的硬性要求,你的设计不会翻车。

配套文档:[tokens.md](./tokens.md) · [README.md](./README.md)

---

## 1. 技术栈

- **UI 框架**:Dioxus 0.7.2(Rust),组件用 RSX 编写,样式通过 `class:` 引用 Tailwind 工具类
- **样式栈**:Tailwind CSS + DaisyUI(预编译资产)
- **跨组件复用**:走 `client-shared`,不在页面里写一次性组件
- **运行模式**:Desktop / LiveView(Web)/ SSR
- **Token**:全部走 `--bc-*`,定义在 `crates/client/crates/client-shared/src/styles.rs`,速查见 [`tokens.md`](./tokens.md)

---

## 2. 红线(违反即不可合入)

### R1 · i18n(项目级宪法)

UI 中**禁止硬编码任何语言字符串**。所有可见文案走 i18n 系统,中英双语为基线。
依据:[`docs/CONSTITUTION.md`](../CONSTITUTION.md) § 1.4。

### R2 · Token

不得硬编码颜色 / 字号 / 间距 / 圆角 / 阴影 / 动效。一律走 `--bc-*`,见 [`tokens.md`](./tokens.md)。

### R3 · 组件来源

不得在页面里写一次性按钮 / 卡片 / 输入框。新组件先进 `client-shared`,补四态,再被使用。

### R4 · 语义化

不得使用 `<div onclick>` 模拟交互。用 `<button>` / `<a>` / `<input>`,否则键盘和屏幕阅读器全瞎。

---

## 3. Merge 前 Checklist(唯一规范出口)

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

---

## 4. 反例速查

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
