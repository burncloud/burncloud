# Repository 层规范（Database Model）

> 仅在编写 `crates/database/crates/database-*/src/` 下的代码时需要详细阅读。

---

## 核心约束

1. **每个 `database-*` crate 只负责一个业务域**。跨域数据访问通过 Service 层组合，不在 Database 层跨 crate 调用。
2. **Model 层只做 SQL 操作，不含业务逻辑**。条件判断、规则计算一律在 Service 层。
3. **多数据库兼容：不可硬编码 SQL 方言**。用 `db.kind()` 分支或 `adapt_sql()` / `ph()` / `phs()`。
4. **行类型定义在 `burncloud_common::types`**，Model 层只负责查询，不重复定义类型。

---

## 标准 Model 结构

```rust
use burncloud_common::types::XxxRow;
use burncloud_database::{Database, Result};

pub struct XxxModel;

impl XxxModel {
    pub async fn get_by_id(db: &Database, id: i64) -> Result<Option<XxxRow>> {
        let conn = db.get_connection()?;
        let is_postgres = db.kind() == "postgres";

        let sql = if is_postgres {
            "SELECT * FROM xxx_table WHERE id = $1"
        } else {
            "SELECT * FROM xxx_table WHERE id = ?"
        };

        let row = sqlx::query_as(sql)
            .bind(id)
            .fetch_optional(conn.pool())
            .await?;

        Ok(row)
    }

    pub async fn create(db: &Database, input: &XxxInput) -> Result<i64> {
        let conn = db.get_connection()?;

        let sql = if db.kind() == "postgres" {
            "INSERT INTO xxx_table (col_a, col_b) VALUES ($1, $2) RETURNING id"
        } else {
            "INSERT INTO xxx_table (col_a, col_b) VALUES (?, ?)"
        };

        // PostgreSQL 用 RETURNING，SQLite 用 last_insert_rowid()
        if db.kind() == "postgres" {
            let row: (i64,) = sqlx::query_as(sql)
                .bind(&input.col_a)
                .bind(&input.col_b)
                .fetch_one(conn.pool())
                .await?;
            Ok(row.0)
        } else {
            let result = sqlx::query(sql)
                .bind(&input.col_a)
                .bind(&input.col_b)
                .execute(conn.pool())
                .await?;
            Ok(result.last_insert_id().unwrap_or(0))
        }
    }

    pub async fn get_all(db: &Database) -> Result<Vec<XxxRow>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query_as("SELECT * FROM xxx_table ORDER BY id")
            .fetch_all(conn.pool())
            .await?;
        Ok(rows)
    }

    pub async fn delete(db: &Database, id: i64) -> Result<bool> {
        let conn = db.get_connection()?;

        let sql = if db.kind() == "postgres" {
            "DELETE FROM xxx_table WHERE id = $1"
        } else {
            "DELETE FROM xxx_table WHERE id = ?"
        };

        let result = sqlx::query(sql).bind(id).execute(conn.pool()).await?;
        Ok(result.rows_affected() > 0)
    }
}
```

---

## 多数据库 SQL 兼容规则

本项目同时支持 SQLite（开发/单机）和 PostgreSQL（生产）。SQL 需兼容两种方言。

### 占位符

| 场景 | 写法 |
|------|------|
| 短查询，1-3 个参数 | 用 `ph()` / `phs()` 构建 |
| 长查询，逻辑复杂 | 用 `if db.kind() == "postgres"` 分支 |

```rust
use burncloud_database::placeholder::{ph, phs};

let is_postgres = db.kind() == "postgres";

// ph(is_postgres, position) → "$1" 或 "?"
let sql = format!("WHERE id = {}", ph(is_postgres, 1));

// phs(is_postgres, count) → "$1, $2, $3" 或 "?, ?, ?"
let placeholders = phs(is_postgres, ids.len());
let sql = format!("WHERE id IN ({})", placeholders);
```

### 保留字转义

PostgreSQL 中 `group`、`type`、`order` 是保留字，需要双引号；SQLite 用反引号：

```rust
let group_col = if db.kind() == "postgres" { "\"group\"" } else { "`group`" };
```

### INSERT 返回 ID

```rust
if db.kind() == "postgres" {
    // RETURNING id 直接返回
    let row: (i64,) = sqlx::query_as("INSERT ... RETURNING id")...
} else {
    // SQLite: execute 后取 last_insert_rowid
    let result = sqlx::query("INSERT ...").execute(conn.pool()).await?;
    Ok(result.last_insert_id().unwrap_or(0))
}
```

---

## `adapt_sql()` 辅助函数

对于参数固定、只需切换占位符风格的简单查询，可用 `adapt_sql()`：

```rust
use burncloud_database::adapt_sql;

// adapt_sql 将 ? 替换为 $1, $2... (PostgreSQL) 或保持不变 (SQLite)
let is_postgres = db.kind() == "postgres";
let sql = adapt_sql(is_postgres, "SELECT * FROM users WHERE id = ? AND status = ?");
```

---

## 事务

批量操作或多步写入必须用事务：

```rust
let mut tx = conn.pool().begin().await?;

sqlx::query("INSERT INTO ...")
    .bind(val_a)
    .execute(&mut *tx)
    .await?;

sqlx::query("UPDATE ...")
    .bind(val_b)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
```

---

## `CrudRepository` Trait（Wave 3 规划）

`burncloud_common::CrudRepository` 是未来统一接口的基础 Trait：

```rust
use burncloud_common::CrudRepository;

#[async_trait::async_trait]
impl CrudRepository<XxxRow, i64, DatabaseError> for XxxModel {
    async fn find_by_id(&self, id: &i64) -> Result<Option<XxxRow>, DatabaseError> { ... }
    async fn list(&self) -> Result<Vec<XxxRow>, DatabaseError> { ... }
    async fn create(&self, input: &XxxRow) -> Result<XxxRow, DatabaseError> { ... }
    async fn update(&self, id: &i64, input: &XxxRow) -> Result<bool, DatabaseError> { ... }
    async fn delete(&self, id: &i64) -> Result<bool, DatabaseError> { ... }
}
```

现阶段（Wave 1-2）静态方法即可，新 crate 可选择实现 Trait。

---

## 反例

```rust
// ✗ 禁止：硬编码 PostgreSQL 方言
let sql = "SELECT * FROM users WHERE id = $1";

// ✗ 禁止：硬编码 SQLite 方言
let sql = "SELECT * FROM users WHERE id = ?";

// ✗ 禁止：跨 database-* crate 直接调用
use burncloud_database_user::UserAccountModel;
// （只能在 Service 层通过 Service 组合）

// ✗ 禁止：Model 层包含业务逻辑
pub async fn find_active_premium_users(db: &Database) -> Result<Vec<UserAccount>> {
    // 这是业务筛选，应在 Service 层
    if user.subscription == "premium" && user.balance > 0 { ... }
}
```
