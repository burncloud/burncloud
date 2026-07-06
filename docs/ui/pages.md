# BurnCloud 页面模板

**规则**: [`system.md`](./system.md) · **组件**: [`components.md`](./components.md)

新建页面时**先选模板**，不要从空白 `rsx!` 开始。

---

## 模板 A：控制台标准列表页

**适用**: Token、用户、渠道等「标题 + 列表 + 创建/编辑弹窗」

**参考**: `crates/client/crates/client-access/src/lib.rs`

```rust
#[component]
pub fn XxxPage() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
    let mut show_create = use_signal(|| false);
    let toast = use_toast();

    let items = use_resource(move || async move { XxxService::list().await });

    rsx! {
        PageHeader {
            title: t(*lang.read(), "xxx.title").to_string(),
            subtitle: Some(t(*lang.read(), "xxx.subtitle").to_string()),
            actions: rsx! {
                BCButton {
                    variant: ButtonVariant::Primary,
                    onclick: move |_| show_create.set(true),
                    {t(*lang.read(), "xxx.create")}
                }
            },
        }

        div { class: "page-content flex flex-col gap-lg",
            // loading → SkeletonCard
            // empty  → EmptyState + CTA
            // data   → 列表或 SchemaTable
        }

        BCModal {
            open: show_create(),
            title: t(*lang.read(), "xxx.create_title").to_string(),
            onclose: move |_| show_create.set(false),
            SchemaForm { schema, data: form_data, mode: FormMode::Create, on_submit: handle_create }
        }
    }
}
```

**Checklist**

- [ ] `PageHeader` + `page-content`
- [ ] 文案全部 `t()`
- [ ] 按钮全部 `BCButton`
- [ ] 含 loading / empty / error 至少三种状态

---

## 模板 B：Schema 驱动 CRUD（最快）

**适用**: 标准 REST CRUD，字段已由 JSON schema 描述

**参考**: `StandardCrudPage` in `client-shared`

```rust
#[component]
pub fn XxxPage() -> Element {
    let schema = xxx_schema(); // 或 include 自 schema 模块
    rsx! {
        StandardCrudPage {
            schema: schema.clone(),
            api_endpoint: "/api/xxx".to_string(),
        }
    }
}
```

能走 schema 的列表页**优先**此模板，避免重复写 Modal + 删除确认。

---

## 模板 C：控制台仪表盘

**适用**: Dashboard、Monitor 概览

**参考**: `crates/client/crates/client-dashboard/src/dashboard.rs`

```rust
rsx! {
    PageHeader {
        title: t(*lang.read(), "dashboard.title").to_string(),
        subtitle: Some(t(*lang.read(), "dashboard.subtitle_24h").to_string()),
    }
    div { class: "page-content flex flex-col gap-xl",
        // ErrorBanner（按 API 分条展示）
        div { class: "stats-grid cols-4",
            // loading → SkeletonCard × 4
            // else    → StatKpi × N
        }
        // 可选: table、channel health、recent logs
    }
}
```

KPI 标签文案走 i18n，禁止硬编码 `"REQUESTS · ALL"` 类英文字符串。

---

## 模板 D：营销落地页

**适用**: `HomePage`、对外介绍

**布局**: `GuestLayout` · **样式**: `landing-*`、`landing-btn`

**参考**: `crates/client/src/pages/home.rs`

- 可用 `landing-btn-light` / `landing-btn-ghost`
- 文案走 `t()`（首页已有 i18n key）
- **不要**使用控制台 `BCButton` / `page-header`

---

## 模板 E：认证页（登录 / 注册）

**适用**: `LoginPage`、`RegisterPage`、`ForgotPasswordPage`

**布局**: `GuestLayout` · **样式**: `login-*`、`landing-btn-dark`

**参考**: `crates/client/src/pages/login.rs`

- 左侧品牌区 + 右侧 `login-form`
- 输入用 `login-input` 包裹层（认证域例外，见 [`components.md`](./components.md)）
- 提交按钮可用 `landing-btn landing-btn-dark`

---

## 路由与 Layout 对应

| 路由前缀 | Layout | 模板 |
|----------|--------|------|
| `/console/*` | `Layout`（侧栏 + 可选 TitleBar） | A / B / C |
| `/home`、`/login`、`/register`… | `GuestLayout` | D / E |

在 `crates/client/src/app.rs` 的 `Route` 枚举注册新路由，页面模块放在 `src/pages/` 或独立 `client-*` crate。

---

## 反例

```rust
// ❌ 控制台页从零写 table + 裸 button
div { class: "page-header", h1 { "Users" } }
button { class: "btn btn-primary", onclick: ..., "新建" }

// ❌ 在 dashboard 用 landing-btn
button { class: "landing-btn landing-btn-dark", ... }

// ✅ 见模板 A / C
```
