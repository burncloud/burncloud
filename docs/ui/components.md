# BurnCloud UI 组件白名单

**权威实现**: `crates/client/crates/client-shared/src/components/`
**硬性规则**: 见 [`system.md`](./system.md) R1–R4 · 命名见 [`naming.md`](./naming.md)

> 控制台页面**必须**使用本文件列出的组件。禁止在业务页新建一次性按钮 / 输入框 / 弹窗样式。

---

## 导入路径

```rust
use burncloud_client_shared::components::{
    BCButton, BCInput, BCModal, BCCard, PageHeader, EmptyState, ErrorBanner,
    SchemaForm, SchemaTable, SkeletonCard, SkeletonVariant, StatusPill, StatKpi,
    // ...
};
use burncloud_client_shared::i18n::{t, t_fmt, use_i18n};
```

---

## 控制台组件（`/console/*`）

| 用途 | 组件 | 禁止替代 |
|------|------|----------|
| 按钮 | `BCButton` + `ButtonVariant` | 裸 `<button class="btn">`、`<button class="btn-secondary">` |
| 输入框 | `BCInput` 或 `SchemaForm` 字段 | 裸 `.input`、`.bc-input-native` 手写 |
| 弹窗 | `BCModal` | 自写 overlay / backdrop |
| 卡片容器 | `BCCard` | 裸 `.card`、手写 `bc-card-solid` 拼装 |
| 页面标题区 | `PageHeader` | 自写 `page-header` 结构 |
| 页面主体 | `div { class: "page-content" }` | 自造 padding / 滚动容器 |
| 空状态 | `EmptyState` | 页面内手写「暂无数据」块 |
| 错误条 | `ErrorBanner` | 裸红色 div |
| 加载骨架 | `SkeletonCard` | 裸 `bc-spinner` 占满列表 |
| 状态标签 | `StatusPill` / `BCBadge` | 手写彩色圆点 |
| KPI 数字 | `StatKpi` | 手写 `stat-card`（新代码优先 StatKpi） |
| 表格（schema） | `SchemaTable` | 手写 `<table class="table">` |
| 表单（schema） | `SchemaForm` | 逐字段手写 input |
| 通用 CRUD | `StandardCrudPage` | 从零写列表+弹窗+删除 |

### BCButton

```rust
// ✅ 控制台主操作
BCButton {
    variant: ButtonVariant::Primary,
    onclick: move |_| do_action(),
    {t(*lang.read(), "common.save")}
}

// ✅ 次要 / 危险
BCButton { variant: ButtonVariant::Secondary, ... }
BCButton { variant: ButtonVariant::Danger, ... }
BCButton { variant: ButtonVariant::Ghost, ... }

// ❌ 禁止
button { class: "btn btn-primary", "保存" }
button { class: "btn btn-secondary", ... }
```

参考实现: `crates/client/crates/client-access/src/lib.rs`

### BCModal

```rust
BCModal {
    open: show_modal(),
    title: t(*lang.read(), "xxx.modal_title").to_string(),
    onclose: move |_| show_modal.set(false),
    footer: Some(rsx! {
        BCButton { variant: ButtonVariant::Ghost, onclick: ..., {t(..., "common.cancel")} }
        BCButton { variant: ButtonVariant::Primary, onclick: ..., {t(..., "common.confirm")} }
    }),
    // 表单或说明正文
    SchemaForm { ... }
}
```

### PageHeader + page-content

```rust
rsx! {
    PageHeader {
        title: t(*lang.read(), "xxx.title").to_string(),
        subtitle: Some(t(*lang.read(), "xxx.subtitle").to_string()),
        actions: rsx! {
            BCButton { variant: ButtonVariant::Primary, onclick: ..., {t(..., "xxx.create")} }
        },
    }
    div { class: "page-content",
        // 业务内容
    }
}
```

### 四态（loading / empty / error / success）

```rust
if loading {
    SkeletonCard { variant: Some(SkeletonVariant::Row) }
} else if let Some(err) = error {
    ErrorBanner { message: t_fmt(lang, "xxx.error", &[("error", &err)]), on_retry: Some(...) }
} else if list.is_empty() {
    EmptyState {
        title: t(lang, "xxx.empty_title").to_string(),
        description: Some(t(lang, "xxx.empty_desc").to_string()),
        cta: Some(rsx! { BCButton { ... } }),
        ..
    }
} else {
    // 正常列表 / 表格
}
```

---

## 营销 / 认证域（GuestLayout）

以下路径允许使用 **landing / login 专用类**，**不得**抄进控制台：

| 路径 | 允许 |
|------|------|
| `src/pages/home.rs` | `landing-btn`、`landing-*` |
| `src/pages/login.rs`、`register`、`forgot_password`、`reset_password` | `login-input`、`landing-btn` |
| `GuestLayout` 子树 | `GlassCard`、`ZenContainer`（可选） |

控制台 (`Layout` + `/console/*`) **禁止** `landing-btn`、`login-input`。

---

## 布局与样式分工

完整命名规则见 [`naming.md`](./naming.md)。摘要:

| 层 | 负责 | 示例 |
|----|------|------|
| **Tailwind 结构** | flex / grid / 尺寸 / overflow | `flex`, `min-h-0`, `w-full`, `overflow-y-auto` |
| **Tailwind `bc-` 语义** | 间距 / 颜色 / 圆角(绑 `--bc-*`) | `gap-bc-4`, `text-bc-text-secondary`, `rounded-bc-md` |
| **组件 / 皮肤** | `BC*`、`styles/*.css` | `BCButton`, `page-content`, `.btn-primary`(仅组件内) |

```rust
// ✅ 控制台标准写法
div { class: "flex flex-col gap-bc-4 text-bc-text-secondary" }

// ❌ 硬编码
div { style: "color: #86868B; gap: 12px;" }

// ❌ 新代码禁止：遗留间距、未绑 token 的 Tailwind 默认档
div { class: "flex flex-col gap-md" }   // 旧代码可有，新页勿增
div { class: "flex flex-col gap-3" }    // 未映射 --bc-space-*
```

遗留类 `gap-md` / `p-lg` 与规范类对照表见 [`naming.md` §5](./naming.md#5-遗留类--规范类映射迁移用)。

---

## 参考页面（抄这些，不要抄 client-api）

| 类型 | 参考文件 |
|------|----------|
| 列表 + 创建弹窗 | `crates/client/crates/client-access/src/lib.rs` |
| Schema CRUD | `crates/client/crates/client-shared/src/components/standard_crud_page.rs` |
| 仪表盘 KPI | `crates/client/crates/client-dashboard/src/dashboard.rs` |
| 渠道管理 | `crates/client/crates/client-models/src/models.rs` |

**勿参考**: `crates/client/crates/client-api/`（遗留 dev shell，样式与 i18n 未对齐主应用）。

---

## 新增组件流程

1. 在 `client-shared/src/components/` 实现，补全 hover / focus / disabled
2. 样式只加在 `styles/` 对应片段或复用已有 `.btn-*` / `.bc-*`
3. 在 `components/mod.rs` 导出
4. 更新本文件一行说明
5. PR 对照 [`system.md`](./system.md) checklist 与 [`naming.md`](./naming.md)
