# BurnCloud 设计文档

这是 BurnCloud 的设计宪法 —— **6 份文件**,各管一件事。

| 文件 | 是什么 | 什么时候看 |
| --- | --- | --- |
| [`system.md`](./system.md) | **基础规范**(地基) | 写 UI、提 PR 前 |
| [`naming.md`](./naming.md) | **CSS / Tailwind 命名规范** | 写 `class:`、间距、颜色时 |
| [`components.md`](./components.md) | **组件白名单** + 正反例 | 用按钮/表单/弹窗时 |
| [`pages.md`](./pages.md) | **页面模板** (A–E) | 新建页面时 |
| [`tokens.md`](./tokens.md) | **Token 速查表** | 找颜色 / 字号 / 间距时 |
| `README.md`(本文件) | 索引 + 3 分钟概览 | 第一次来 |

---

## 3 分钟概览

### 视觉风格

**参照 Apple Human Interface Guidelines** —— 留白优先、克制动效、层次靠字重和间距而非边框和阴影。

这是项目唯一的视觉语言主张。不混 Fluent、不混 Material。具体落地走 [`system.md`](./system.md) 的红线和 checklist,本节只定调。

### 我们的设计哲学

**守地基** —— [`system.md`](./system.md) 列的硬性要求,违反即不可合入。

### 工作流

```
新页面 / 新功能
    │
    ▼
读 naming.md（class 怎么写）+ components.md（用哪个组件）
    │
    ▼
按 system.md checklist 实现
    │
    ▼
PR review:对照 checklist 自查
```

### 红线(违反即不可合入)

来自 [`system.md`](./system.md) §2:

- **R1 · i18n**:UI 不得硬编码任何语言字符串(中英双语为基线)
- **R2 · Token**:不得硬编码颜色 / 字号 / 间距,一律走 `--bc-*`
- **R3 · 组件**:不在页面里写一次性组件,新组件先进 `client-shared`
- **R4 · 语义化**:不用 `<div onclick>`,用 `<button>` / `<a>` / `<input>`

### 技术栈

Dioxus(Rust) + RSX + Tailwind(布局) + `styles/*.css`(组件皮肤) + `client-shared` 组件库。
Token 权威定义:`crates/client/crates/client-shared/src/styles/00_burncloud_design_system_apple_inspired.css`（见 `styles/mod.rs`）。
组件 API 见 [`components.md`](./components.md)。

---

## 如何使用这套文档

- **新建页面**:先读 [`pages.md`](./pages.md) 选模板,再读 [`components.md`](./components.md)
- **写 class / 间距 / 颜色**:先读 [`naming.md`](./naming.md),再查 [`tokens.md`](./tokens.md)
- **写代码时**:打开 [`tokens.md`](./tokens.md) 找 Token 名
- **提 PR 前**:对照 [`system.md`](./system.md) §3 的 checklist 自查
- **AI / Agent**:Cursor 规则 `.cursor/rules/ui-design-system.mdc` 会指向本目录
- **新人 onboarding**:从本文件开始,约 15 分钟读完全部 6 份

---

## 维护规则

- 改 Token 数值 → 改 `styles/00_burncloud_design_system_apple_inspired.css`（**不**改 `tokens.md` 里的数值）
- 改 Token 命名 → 同步改 `tokens.md` 与 `00_*.css`
- 改组件皮肤（按钮/输入/弹窗等）→ 改 `styles/` 对应片段（`04_button_styles.css` 等）
- 改硬性要求 → 改 `system.md`
- 新增/重命名组件 → 同步改 `components.md`
- 新增 class 命名规则 → 同步改 `naming.md`
- 新增页面类型 → 同步改 `pages.md`
- 重组样式：运行 `crates/client/crates/client-shared/scripts/split_styles.py`（仅当从单体 CSS 重新切分时需要）

`styles.rs` 已拆为 `styles/mod.rs` + `styles/*.css`，由 `DESIGN_SYSTEM_CSS` 拼接注入 `AppStyles`。

> **每份文件只有一个主人。** 勿在 `crates/client/crates/client-api/docs/` 写 UI 规范(已废弃)。
