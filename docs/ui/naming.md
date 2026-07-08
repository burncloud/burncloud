# BurnCloud CSS / Tailwind 命名规范

**版本**: 1.0  
**生效日期**: 2026-07-07

**权威配置**: [`crates/client/tailwind.config.js`](../../crates/client/tailwind.config.js) · Token 真源 [`styles/00_burncloud_design_system_apple_inspired.css`](../../crates/client/crates/client-shared/src/styles/00_burncloud_design_system_apple_inspired.css)

> 本文是 **class 命名与 RSX 写法** 的唯一详述出口。Token 含义见 [`tokens.md`](./tokens.md)；组件白名单见 [`components.md`](./components.md)。

---

## 1. 三层分工

控制台页面 RSX 的 `class` 由三层叠加，各司其职：

| 层 | 职责 | 示例 | 绑 `--bc-*`？ |
|----|------|------|---------------|
| **结构** | flex / grid / 尺寸 / 溢出 | `flex`, `grid`, `min-h-0`, `w-full`, `overflow-y-auto` | 否 |
| **语义（Tailwind `bc-`）** | 间距 / 颜色 / 圆角 / 阴影 | `gap-bc-4`, `text-bc-text-secondary`, `rounded-bc-md` | **是** |
| **组件** | 按钮 / 输入 / 页面壳 | `BCButton`, `bc-input`, `page-content` | 组件内已绑 |

```rust
// ✅ 标准组合
div {
    class: "flex flex-col gap-bc-4 min-h-0 text-bc-text-secondary",
    // ...
}
```

**禁止** 用 `style:` 写间距或颜色（见 [`system.md`](./system.md) R2）。

---

## 2. 前缀语义

| 前缀 / 模式 | 含义 | 使用范围 |
|-------------|------|----------|
| `--bc-*` | CSS 变量（设计 token 真源） | `styles/00_*.css`、必要时 `var()` |
| `*-bc-*`（Tailwind） | 引用 `--bc-*` 的工具类 | **控制台新代码首选** |
| `text-caption` 等 | 排版语义类（`10_typography.css`） | 标题层级 |
| `btn-*` | 按钮皮肤 | **仅** `BCButton` 内部或 `styles/04_*.css` |
| `bc-input-*` | 输入框皮肤 | **仅** `BCInput` / `SchemaForm` |
| `landing-*` / `login-*` | 营销 / 认证专用 | Guest 域，**禁止**进控制台 |
| `bc-fluent-*` | 历史遗留 | **deprecated**，新代码勿增；营销文案待统一为 Apple HIG |
| `gap-md` / `p-lg` 等 | 遗留间距短名 | **deprecated**，见 §5 映射表 |
| `bc-gap-*`（`25_*.css`） | 遗留一次性工具 | **deprecated**，新代码用 Tailwind `gap-bc-*` |
| `bg-muted` / shadcn 类 | 外部设计系统 | **禁止** |

---

## 3. 控制台新代码规范

### 3.1 间距与内外边距

使用 Tailwind + `bc-` 间距档位（定义于 `tailwind.config.js` → `theme.extend.spacing`）。

| ✅ 使用 | 对应 token | 像素 |
|---------|------------|------|
| `gap-bc-3` / `p-bc-3` / `m-bc-3` | `--bc-space-3` | 12px |
| `gap-bc-4` / `p-bc-4` / `m-bc-4` | `--bc-space-4` | 16px |
| `gap-bc-6` / `p-bc-6` / `m-bc-6` | `--bc-space-6` | 24px |
| `gap-bc-8` / `p-bc-8` | `--bc-space-8` | 32px |

方向变体：`px-bc-4`、`py-bc-6`、`mb-bc-2`、`mt-bc-4` 等，规则相同。

| ❌ 新代码禁止 |
|---------------|
| `gap-md`, `gap-lg`, `p-md`, `p-lg`, `mb-sm`（`09_*.css` 遗留） |
| `bc-gap-6`, `bc-pl-6`（`25_*.css` 遗留） |
| `gap-3`, `p-6`（Tailwind 默认刻度，**未**绑定 `--bc-space-*`） |
| `style: "gap: 16px"` |

### 3.2 颜色

| ✅ 使用 | 对应 token |
|---------|------------|
| `text-bc-text` | `--bc-text-primary` |
| `text-bc-text-secondary` | `--bc-text-secondary` |
| `text-bc-text-tertiary` | `--bc-text-tertiary` |
| `text-bc-primary` / `bg-bc-primary` | `--bc-primary` |
| `text-bc-danger` / `bg-bc-danger-light` | `--bc-danger` / `-light` |
| `bg-bc-canvas` | `--bc-bg-canvas` |
| `bg-bc-card` | `--bc-bg-card-solid` |
| `border-bc-border` | `--bc-border` |

过渡：旧代码中的 `border-[var(--bc-border)]` 允许保留至迁移完成；**新代码优先** `border-bc-border`。

| ❌ 禁止 |
|---------|
| 裸 hex / `rgb()` / `rgba()` in `style` 或 Tailwind 任意值 `text-[#86868B]` |
| `text-gray-500` 等 Tailwind 默认色板 |
| shadcn：`bg-muted`, `text-foreground` 等 |

### 3.3 字号

优先 **语义排版类**（与 Apple 层级一致）：

| 类名 | 典型用途 |
|------|----------|
| `text-caption` | 辅助说明、表头 |
| `text-body` | 正文 |
| `text-subtitle` | 小节标题 |
| `text-title` | 页面内标题 |
| `text-large-title` | 强调数字 / 大标题 |

或使用 Tailwind 映射：`text-bc-sm`、`text-bc-lg` 等（见 [`tokens.md`](./tokens.md)）。

单页字号层级 **≤ 5 级**（[`system.md`](./system.md)）。

### 3.4 圆角与阴影

| ✅ | 示例 |
|----|------|
| 圆角 | `rounded-bc-sm`, `rounded-bc-md`, `rounded-bc-lg` |
| 阴影 | `shadow-bc-sm`, `shadow-bc-primary` |

### 3.5 按钮与表单

| 场景 | ✅ | ❌ |
|------|----|----|
| 控制台按钮 | `BCButton { variant: ButtonVariant::Primary, ... }` | `<button class="btn btn-primary">` |
| 控制台输入 | `BCInput` / `SchemaForm` | 手写 `bc-input-native` |
| 弹窗 | `BCModal` | 自写 overlay |

`btn-primary` 等类名仍存在于 `BCButton` 实现中（历史 Daisy 皮肤名），**业务 RSX 不得直接写**。

---

## 4. 域隔离

| 域 | 路由 / 布局 | 允许的 class 族 |
|----|-------------|-----------------|
| **控制台** | `Layout`, `/console/*` | `*-bc-*` Tailwind、`BC*` 组件、`page-content`、`text-caption` 等 |
| **营销 / 认证** | `home`, `login`, `register`, `GuestLayout` | `landing-*`, `login-input`, `landing-btn` |
| **遗留** | `client-api` | 不参考、不扩展 |

控制台 **禁止**：`landing-btn`, `login-input`, `bc-fluent-*`（新代码）。

---

## 5. 遗留类 → 规范类映射（迁移用）

旧代码可保留；**新代码与迁移改写** 使用右列。

### 间距（`09_layout_helpers_*.css`）

| 遗留 | 规范 | token | 像素 |
|------|------|-------|------|
| `gap-xs` | `gap-bc-1` | `--bc-space-1` | 4px |
| `gap-sm` | `gap-bc-2` | `--bc-space-2` | 8px |
| `gap-md` | `gap-bc-3` | `--bc-space-3` | 12px |
| `gap-lg` | `gap-bc-4` | `--bc-space-4` | 16px |
| `gap-xl` | `gap-bc-5` | `--bc-space-5` | 20px |
| `p-lg` | `p-bc-4` | `--bc-space-4` | 16px |
| `p-md` | `p-bc-3` | `--bc-space-3` | 12px |
| `p-xxl` | `p-bc-6` | `--bc-space-6` | 24px |
| `p-xxxl` | `p-bc-8` | `--bc-space-8` | 32px |
| `mb-sm` | `mb-bc-2` | `--bc-space-2` | 8px |

`margin-*` / `padding-*` 短名同理：将中间档名换成 `bc-N`。

### 工具类（`25_*.css`）

| 遗留 | 规范 |
|------|------|
| `bc-gap-6` | `gap-bc-6` |
| `bc-pl-6` | `pl-bc-6` |
| `bc-text-brand` | `text-bc-primary` |

---

## 6. 正反例

```rust
// ✅ 控制台列表页
div { class: "page-content flex flex-col gap-bc-4",
    div { class: "flex items-center justify-between gap-bc-3",
        h2 { class: "text-subtitle text-bc-text m-0", ... }
        BCButton { variant: ButtonVariant::Primary, ... }
    }
}

// ❌ 遗留间距 + 裸按钮
div { class: "flex flex-col gap-lg",
    button { class: "btn btn-primary", "Save" }
}

// ❌ 未绑 token 的 Tailwind 默认间距
div { class: "flex gap-3 p-6" }

// ❌ 硬编码
div { style: "color: #007AFF; padding: 16px;" }

// ✅ 营销域（仅 home / login）
a { class: "landing-btn landing-btn-light", ... }
```

---

## 7. 维护流程

1. **新增 token** → 改 `styles/00_*.css` → 更新 [`tokens.md`](./tokens.md) → 如需 Tailwind 工具类，改 `tailwind.config.js` → 更新本文映射表  
2. **新增组件 class** → 加在 `styles/` 对应片段 → 更新 [`components.md`](./components.md)  
3. **新增命名规则** → **先改本文**，再写 RSX  
4. **PR 自查** → [`system.md`](./system.md) §3 checklist

---

## 8. 相关文件

| 文件 | 关系 |
|------|------|
| [`tailwind.config.js`](../../crates/client/tailwind.config.js) | `gap-bc-*` 等工具类的生成配置 |
| [`styles/09_*.css`](../../crates/client/crates/client-shared/src/styles/09_layout_helpers_burncloud_specific_not_tailwind_d.css) | 遗留间距（deprecated） |
| [`styles/25_*.css`](../../crates/client/crates/client-shared/src/styles/25_token_compliant_utility_classes_issue_179_migrat.css) | 遗留工具（deprecated） |
| [`check-ui-conventions.sh`](../../crates/client/scripts/check-ui-conventions.sh) | 当前仅检查按钮；间距命名 CI 待后续 PR |
