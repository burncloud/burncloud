# 数据模型规范

> 在 `crates/database/crates/database-*/src/` 中定义或修改结构体时阅读。

---

## 两类核心类型

每个实体最多两个结构体，定义在同一文件：

| 类型 | 命名 | 用途 | 来自数据库 |
|------|------|------|---------|
| **行类型** | `XxxRow` / `UserApiKey` | 映射 SQL 查询结果，可直接序列化为 API 响应 | 是（FromRow） |
| **输入类型** | `XxxInput` / `UserApiKeyInput` | 创建 / 更新操作的入参，字段通常为 Optional | 否 |

> 项目**不采用** DO / BO / VO 三层分离。
> 行类型同时承担 DB 映射和 API 序列化职责，用显式字段过滤代替分层隔离。

---

## 行类型规则

```rust
// crates/database/crates/database-models/src/user_api_key.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserApiKey {
    pub id: i32,
    pub user_id: String,
    pub key: String,
    pub status: i32,
    pub name: Option<String>,
    pub remain_quota: i64,       // nanocents（9位精度）
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub created_time: Option<i64>,
    pub expired_time: i64,
}
```

**规则：**

1. 行类型 `derive(Serialize, Deserialize)` — 可直接用于 API 响应 JSON
2. **敏感字段**（`password_hash`）加 `#[serde(skip_serializing)]` 或在 Handler 中手工构造响应 DTO，不出现在 API 响应中
3. 货币金额统一用 `i64` **nanocents**（10⁻⁹ 精度），不用 `f64`
4. 布尔值在 SQLite 中存为 `INTEGER`，需手写 `FromRow` 实现（见下方模板）

---

## 输入类型规则

```rust
/// Input for creating a user API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserApiKeyInput {
    pub user_id: String,
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub unlimited_quota: Option<bool>,
    pub expired_time: Option<i64>,
}

/// Input for updating a user API key (所有字段 Optional — 只更新提供的字段)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserApiKeyUpdateInput {
    pub name: Option<String>,
    pub remain_quota: Option<i64>,
    pub status: Option<i32>,
    pub expired_time: Option<i64>,
}
```

**规则：**

1. 创建输入：必填字段用具体类型，可选字段用 `Option<T>`
2. 更新输入：**所有字段都是 `Option<T>`**，`#[derive(Default)]`，只更新非 None 的字段
3. 输入类型不含 `id`、`created_at` 等由数据库生成的字段
4. 输入类型不含 `password_hash`，密码 hash 在 Service 层计算后传入

---

## 手写 `FromRow`（SQLite 布尔值）

`sqlx::Any` 下 SQLite `INTEGER` 无法自动转 `bool`，需手写：

```rust
impl<'r> sqlx::FromRow<'r, sqlx::any::AnyRow> for UserApiKey {
    fn from_row(row: &'r sqlx::any::AnyRow) -> std::result::Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(UserApiKey {
            // ... 其他字段
            unlimited_quota: row.try_get::<i64, _>("unlimited_quota")? != 0,
            // ...
        })
    }
}
```

当结构体包含 `bool` 字段时，不用 `#[derive(FromRow)]`，改用手写实现。

---

## 类型归属位置

| 类型范畴 | 归属 crate | 示例 |
|---------|----------|------|
| 业务域行类型 | `database-*` 各自 crate | `UserAccount`（database-user）|
| 跨 crate 共享类型 | `database-models` | `UserApiKey`、`ChannelAbility` |
| 纯计算 / 配置结构 | `common` | `PricingConfig`、`CurrencyPricing` |
| Handler 专用请求 DTO | `server/src/api/` handler 文件 | `RegisterRequest`（auth.rs）|

---

## 命名约定

```
行类型：      UserApiKey         PascalCase，无后缀
操作类型：    UserApiKeyModel    行类型 + "Model"
创建输入：    UserApiKeyInput    行类型 + "Input"
更新输入：    UserApiKeyUpdateInput  行类型 + "UpdateInput"
```

与数据库命名对齐（见 `DATABASE.md`）：
```
表名：user_api_keys  →  行类型：UserApiKey  →  操作类型：UserApiKeyModel
```

---

## 反例

```rust
// ✗ 禁止：货币字段用 f64（浮点精度问题）
pub balance: f64,
// ✓ 正确：用 i64 nanocents
pub balance_usd: i64,

// ✗ 禁止：敏感字段出现在 API 响应（未 skip）
#[derive(Serialize)]
pub struct UserResponse {
    pub password_hash: String,  // 永远不要序列化这个
}

// ✗ 禁止：更新输入字段用非 Option（导致无法做 partial update）
pub struct UpdateInput {
    pub name: String,   // 用户只想更新 status，却被迫填 name
}
// ✓ 正确：全部 Option
pub struct UpdateInput {
    pub name: Option<String>,
}

// ✗ 禁止：同一个结构体跨越业务域（难以维护）
pub struct UserAndGroup {
    pub user_id: String,
    pub group_name: String,
    // ...
}
```
