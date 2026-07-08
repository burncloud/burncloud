# BurnCloud Design Tokens

**权威定义**:`crates/client/crates/client-shared/src/styles/00_burncloud_design_system_apple_inspired.css`（`styles/mod.rs` 拼接）
**命名空间**:`--bc-*`
**数值**:以 `styles/00_*.css` 为准,本文件只列命名与用途

> 这是一份**速查表**,不是定义。改 Token 请改 `styles/00_burncloud_design_system_apple_inspired.css`,本文件随之更新命名。

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

### Tailwind 映射（控制台 RSX 首选）

配置见 [`crates/client/tailwind.config.js`](../../crates/client/tailwind.config.js) `theme.extend.spacing`。命名规则见 [`naming.md`](./naming.md)。

| Token | 像素 | Tailwind 示例 | 遗留（勿新增） |
| --- | --- | --- | --- |
| `--bc-space-1` | 4px | `gap-bc-1`, `p-bc-1` | `gap-xs`, `p-xs` |
| `--bc-space-2` | 8px | `gap-bc-2`, `mb-bc-2` | `gap-sm`, `mb-sm` |
| `--bc-space-3` | 12px | `gap-bc-3`, `p-bc-3` | `gap-md`, `p-md` |
| `--bc-space-4` | 16px | `gap-bc-4`, `p-bc-4` | `gap-lg`, `p-lg` |
| `--bc-space-5` | 20px | `gap-bc-5`, `p-bc-5` | `gap-xl`, `p-xl` |
| `--bc-space-6` | 24px | `gap-bc-6`, `p-bc-6` | `p-xxl`, `bc-gap-6` |
| `--bc-space-8` | 32px | `gap-bc-8`, `p-bc-8` | `p-xxxl` |
| `--bc-space-10` | 40px | `p-bc-10` | — |
| `--bc-space-12` | 48px | `p-bc-12` | — |

### 颜色 Tailwind 映射

| Token | Tailwind 示例 |
| --- | --- |
| `--bc-text-primary` | `text-bc-text` |
| `--bc-text-secondary` | `text-bc-text-secondary` |
| `--bc-primary` | `text-bc-primary`, `bg-bc-primary` |
| `--bc-bg-canvas` | `bg-bc-canvas` |
| `--bc-border` | `border-bc-border` |

完整列表见 `tailwind.config.js` `theme.extend.colors`。

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

// ✅ RSX 首选：Tailwind bc-* 映射（见 naming.md）
div { class: "text-bc-primary p-bc-4" }

// ✅ 底层等价写法（迁移期允许，新代码优先上一行）
div { class: "text-[var(--bc-primary)] p-[var(--bc-space-4)]" }
```

新增 Token 必须先进 `styles/00_*.css`,再更新本文件与 [`naming.md`](./naming.md),再被使用。
