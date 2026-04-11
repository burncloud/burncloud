# BurnCloud 设计文档

这是 BurnCloud 的设计宪法 —— 4 份文件,各管一件事。

| 文件 | 是什么 | 什么时候看 |
| --- | --- | --- |
| [`system.md`](./system.md) | **基础规范**(地基) | 写 UI、提 PR 前 |
| [`tokens.md`](./tokens.md) | **Token 速查表** | 找颜色 / 字号 / 间距时 |
| [`principles.md`](./principles.md) | **决策铁律**(砍刀) | 设计 review、决定取舍时 |
| `README.md`(本文件) | 索引 + 3 分钟概览 | 第一次来 |

---

## 3 分钟概览

### 我们的设计哲学只有两条

1. **守地基** —— [`system.md`](./system.md) 列的硬性要求,违反即不可合入
2. **默认砍掉** —— [`principles.md`](./principles.md) 的铁律,让每个留下的元素自己举证

**地基让你不翻车,砍刀让你不平庸。** 缺一不可,顺序不能反。

### 工作流

```
新页面 / 新功能
    │
    ▼
回答 3 个问题(principles.md §2)
    为谁做?他想完成什么?他离开时带走什么感觉?
    │
    ▼
走 4 个筛子(principles.md §3)
    能删吗?能合吗?能晚吗?配吗?
    │
    ▼
按 system.md checklist 实现
    │
    ▼
PR review:再走一遍 4 个筛子
```

### 红线(违反即不可合入)

来自 [`system.md`](./system.md) §2:

- **R1 · i18n**:UI 不得硬编码任何语言字符串(中英双语为基线)
- **R2 · Token**:不得硬编码颜色 / 字号 / 间距,一律走 `--bc-*`
- **R3 · 组件**:不在页面里写一次性组件,新组件先进 `client-shared`
- **R4 · 语义化**:不用 `<div onclick>`,用 `<button>` / `<a>` / `<input>`

### 技术栈

Dioxus 0.7.2(Rust)+ RSX + Tailwind + DaisyUI + `client-shared` 组件库。
Token 权威定义:`crates/client/crates/client-shared/src/styles.rs`。

---

## 如何使用这套文档

- **写代码时**:打开 [`tokens.md`](./tokens.md) 找 Token 名
- **提 PR 前**:对照 [`system.md`](./system.md) §3 的 checklist 自查
- **决定做不做**:用 [`principles.md`](./principles.md) §3 的四个筛子
- **新人 onboarding**:从本文件开始,15 分钟读完全部 4 份

---

## 维护规则

- 改 Token 数值 → 改 `styles.rs`,**不**改文档(文档只列命名)
- 改 Token 命名 → 同步改 `tokens.md`
- 改硬性要求 → 改 `system.md`,**不**写在其他文件
- 改决策原则 → 改 `principles.md`,**不**和 `system.md` 混

> **每份文件只有一个主人。** 这是这套文档自己的"默认砍掉"。
