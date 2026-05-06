# BurnCloud — AI 助手快速参考

> 此文件在每次 Claude Code 会话启动时自动加载。
> 阅读时间 < 2 分钟。

---

## 架构速览

```
Client (Dioxus) → Server (axum) → Router (data plane)
                         ↓
                  Service (business logic)
                         ↓
                  Database crates (SQLx)
                         ↓
                  Common (shared utilities)
```

依赖方向严格单向：Server 依赖 Service，Service 依赖 Database，Database 依赖 Common。

**禁止反向依赖。禁止 Service 引入 axum 类型。**
（Router 对 service-billing/service-user 的依赖是宪法例外，见 docs/code/README.md §1.1）

---

## 关键约定（违反会导致编译错误或运行时 bug）

| 约定 | 正确 | 错误 |
|------|------|------|
| Handler 状态注入 | `State(state): State<AppState>` | `State<Arc<AppState>>` |
| Handler 返回类型 | `impl IntoResponse` | `Result<Json<T>, AppError>` |
| 响应构造 | `ok(data).into_response()` | 手写 `Json(...)` |
| 响应错误 | `err("message").into_response()` | 手写 `StatusCode::BAD_REQUEST` |
| SQL 占位符 | `ph(db.is_postgres(), 1)` / `phs(...)` | `$1` 或 `?` 硬编码 |
| Service 传参 | `db: &Database` 按引用传入 | 持有 `db: Database` 字段 |
| 日志 | `log::info!` / `log::warn!` / `log::error!` | `eprintln!` / `println!` |
| 错误类型 | `thiserror::Error` derive | 裸 `String` / `Box<dyn Error>` |

## Upstream Policy（上游接入硬规则）

BurnCloud 仅接入**可溯源的信誉平台**，禁止匿名/不可溯源的第三方中转。

**准入标准**：客户能明确知道资源来路 + 平台有公开品牌和运营主体。

| 类别 | 平台 | ChannelType | Base URL |
|------|------|-------------|----------|
| 官方直连 | Anthropic | 14 | `https://api.anthropic.com` |
| 官方直连 | OpenAI | 1 | `https://api.openai.com` |
| 官方直连 | 智谱AI (Zhipu) | 16 | `https://open.bigmodel.cn` |
| 官方直连 | DeepSeek | 43 | `https://api.deepseek.com` |
| 官方直连 | 阿里云百炼 (Ali) | 17 | `https://dashscope.aliyuncs.com` |
| 官方直连 | 月之暗面 (Moonshot) | 25 | `https://api.moonshot.cn` |
| 信誉聚合 | OpenRouter | 20 | `https://openrouter.ai/api/v1` |
| 信誉聚合 | SiliconFlow | 40 | `https://api.siliconflow.cn/v1` |

**禁止接入**：one-api、new-api 等自建匿名中转。代码中 `ChannelType::NewApi(58)` 为历史遗留，禁止新增此类渠道。

**适用范围**：源代码、配置文件、环境变量示例、文档示例、测试用例、CI 脚本。

新平台接入需满足准入标准并经 PR 评审。

---

## Service 两种模式

**模式 A（有状态，有配置字段）** — 例如 `UserService`
```rust
pub struct UserService { jwt_secret: String }
impl UserService {
    pub fn new() -> Self { ... }
    pub async fn register(&self, db: &Database, ...) -> Result<...> { ... }
}
```

**模式 B（无状态，纯操作封装）** — 例如 `GroupService`, `TokenService`
```rust
pub struct GroupService;
impl GroupService {
    pub async fn get_all(db: &Database) -> Result<Vec<RouterGroup>> { ... }
}
```

---

## 完整规范文档

| 主题 | 文件 |
|------|------|
| 核心规则（必读） | `docs/code/README.md` |
| Handler / Router | `docs/code/SERVER.md` |
| Service 层 | `docs/code/SERVICE.md` |
| 数据库命名 & SQL | `docs/code/DATABASE.md` |
| 数据模型规范 | `docs/code/MODEL.md` |
| 错误处理 | `docs/code/ERROR.md` |
| 完整 copy-paste 模板 | `docs/code/TEMPLATES.md` |

---

## Workspace Crate 结构

```
crates/
  server/        ← axum HTTP 层，Handler 和路由
  service/       ← 业务逻辑（service-user, service-group 等）
  database/      ← 核心 Database + schema + placeholder utils
                ← 子 crate 在 database/crates/ 下按业务域组织（user, token, group, channel, router, router-log, billing, models, setting, installer, download 等）
  common/        ← CrudRepository trait，跨 crate 纯工具
  router/        ← LLM 数据面路由（独立，不在 Service 依赖链内）
  client/        ← Dioxus 前端
  cli/           ← 命令行工具
```

---

## 仓库根目录纪律

**禁止在仓库根目录新增任何文件或目录。** 新内容按下表归位：

| 类别 | 落点 | 示例 |
|------|------|------|
| 部署 / 容器 / 编排 | `deploy/` | `Dockerfile`、`docker-compose.yml`、k8s manifests |
| 部署 / 运维脚本 | `deploy/scripts/` | `entrypoint.sh`、`migrate.sh`、`release.sh` |
| Crate 局部脚本 | `crates/<crate>/scripts/` | 已有约定，例：`crates/router/scripts/` |
| 文档 | `docs/` | 所有 `.md`（根目录 README 例外） |
| 代码 | `crates/` 或 `src/` | 按 workspace 既有结构 |

**根目录允许列表**（工具链强制位置，新增前需 PR 评审）：

`Cargo.toml`、`Cargo.lock`、`README.md`、`clippy.toml`、`deny.toml`、`.cargo/`、`.github/`、`.gitignore`、`.gitattributes`、`.env.example`

不在此列表的根目录新增项一律 reject。

