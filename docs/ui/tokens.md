# BurnCloud Design Tokens

**权威定义**:`crates/client/crates/client-shared/src/styles.rs`
**命名空间**:`--bc-*`
**数值**:以 `styles.rs` 为准,本文件只列命名与用途

> 这是一份**速查表**,不是定义。改 Token 请改 `styles.rs`,本文件随之更新命名。

---

## 颜色

### 品牌色

| Token | 用途 |
| --- | --- |
| `--bc-primary` | 主操作、链接、焦点 |
| `--bc-primary-hover` | 悬停态 |
| `--bc-primary-active` | 按下态 |
| `--bc-primary-light` | 浅底 / 标签背景 |
| `--bc-primary-dark` | 深色变体 |

### 语义色(均带 `-light` 浅底变体)

| Token | 用途 |
| --- | --- |
| `--bc-success` | 成功 |
| `--bc-warning` | 警告 |
| `--bc-danger` | 错误 / 破坏性操作 |
| `--bc-info` | 信息 |

### 中性背景

| Token | 用途 |
| --- | --- |
| `--bc-bg-canvas` | 页面底层 |
| `--bc-bg-card` / `-card-solid` | 卡片(半透明 / 实底) |
| `--bc-bg-elevated` | 浮层 |
| `--bc-bg-hover` / `-selected` | 悬停 / 选中态 |
| `--bc-bg-input` | 输入框 |

### 文字

| Token | 用途 |
| --- | --- |
| `--bc-text-primary` | 主文字 |
| `--bc-text-secondary` | 次文字 |
| `--bc-text-tertiary` | 辅助 / 占位 |
| `--bc-text-on-accent` | 主色背景上的文字 |
| `--bc-text-disabled` | 禁用 |

### 描边

`--bc-border` / `--bc-border-hover` / `--bc-border-focus`

---

## 字号(8 级)

`--bc-font-xs` → `sm` → `base` → `md` → `lg` → `xl` → `2xl` → `3xl`

> **单页面字号层级 ≤ 5 级。** 超过即层次失控。

---

## 间距(基于 4px,9 级)

`--bc-space-1` → `2` → `3` → `4` → `5` → `6` → `8` → `10` → `12`

---

## 圆角

`--bc-radius-xs` → `sm` → `md` → `lg` → `xl` → `2xl` → `--bc-radius-full`

---

## 阴影

`--bc-shadow-xs` → `sm` → `md` → `lg` → `xl`,加品牌发光 `--bc-shadow-primary`

---

## 动效

| Token | 用途 |
| --- | --- |
| `--bc-transition-fast` | 微反馈(hover) |
| `--bc-transition-normal` | 默认 |
| `--bc-transition-slow` | 大块变化 |
| `--bc-transition-spring` | 进入 / 强调 |

> 时长统一,曲线统一。**禁止自写 `cubic-bezier`**。

---

## 字体

`--bc-font-family`:Apple System / Segoe UI 优先的跨平台无衬线栈

---

## 使用铁律

```rust
// ❌ 硬编码
div { style: "color: #007AFF; padding: 16px;" }

// ✅ 走 Token
div { class: "text-[var(--bc-primary)] p-[var(--bc-space-4)]" }
```

新增 Token 必须先进 `styles.rs`,再更新本文件,再被使用。
