# burncloud-router

BurnCloud data-plane routing and upstream execution crate.

## Current role

The router receives unmatched data-plane requests from the unified server fallback, performs request admission/routing, selects upstream candidates, executes provider requests, handles passthrough or conversion branches, tracks response/usage/failure state, and participates in billing/log settlement.

The exact behavior is branch-dependent. Read `src/lib.rs` and the relevant helper module before changing a flow.

## Entry points

`create_router_app()` currently registers three explicit data-plane routes:

- `GET /v1/models`
- `GET /api/v1/usage`
- `GET /api/v1/usage/models`

Other unmatched data-plane paths enter `proxy_handler()` through `.fallback(proxy_handler)`.

Internal operator routes for health, price sync, circuit-breaker trip-all, and metrics are returned separately so the server can merge them before LiveView catch-all behavior.

## Important modules

- `src/lib.rs` — router construction, fallback handler, proxy execution, settlement and internal endpoints.
- `src/model_router.rs` — model/channel candidate loading and ranking.
- `src/passthrough.rs` — passthrough decision logic.
- `src/circuit_breaker.rs` / channel-state related modules — failure/availability state.
- `src/price_sync.rs` — price synchronization.
- adaptor/provider modules — runtime protocol/provider behavior.
- `crates/traffic/router-aws` — AWS-specific signing/support code.

Use source search rather than this list as an exhaustive module index.

## Dependency boundary

Current `Cargo.toml` has two explicitly whitelisted cross-domain implementation dependencies:

- `burncloud-commerce-billing`
- `burncloud-identity-user`

`crates/traffic/router/scripts/check-router-deps.sh` enforces this compatibility whitelist. Adding another cross-domain implementation dependency requires deliberate architecture review and an explicit whitelist update.

## Passthrough and conversion

Passthrough is **conditional**, not a universal “never parse the body” rule. Current code contains native passthrough branches and parsing/conversion branches. Preserve the semantics of the active path and check `src/passthrough.rs` plus the selected branch in `src/lib.rs`.

## Runtime flow

Human-oriented progressive runtime documentation is rendered at:

https://burncloud.github.io/

Use it for navigation, then re-check the current checkout before modifying code.

## Domain Crate 10 问

1. **Owner** — Traffic. Business capability: data-plane routing and upstream execution. The Cargo
   package is `burncloud-router` (`crates/traffic/router`); the Rust import is `burncloud_router`.
   This crate is the converged Traffic Router / Router Log owner.

2. **Responsibility** — receive unmatched data-plane paths through `.fallback(proxy_handler)`, admit
   and route them (availability → `OrderType` filter → affinity → scorer → failover list), execute
   the selected upstream request, choose the passthrough or conversion branch, track
   response/usage/failure state, and persist the raw router logs.
   **Not responsible for**: credential, quota and wallet state (Identity); channel and
   model-capability persistence (Supply); canonical pricing and cost-calculation rules (Commerce);
   cross-domain DTO definitions (`burncloud-*-contracts`); database connection, schema and migration
   infrastructure (Platform). It must not keep a second routing table or a second `ModelRouter`, and
   local-node attachment must make a ready capability visible through the existing Supply
   channel/ability truth.

3. **Data Truth** — the tables this crate owns:
   - `router_logs` (`RouterLog` / `RouterLogModel` / `RouterDatabase::insert_log`): request id, user,
     path, upstream, status, latency, token counts, per-type costs, `cost` in nanodollars,
     `layer_decision`, `traffic_color`, `cost_status`, `error_type`, `created_at`;
   - `router_request_logs` (`RouterRequestLog` / `RouterRequestLogModel`): sanitized bodies, candidate
     list, affinity key/hit, failover history, storage policy;
   - `router_video_tasks` (`RouterVideoTask` / `RouterVideoTaskModel`): `task_id` → `channel_id`
     mapping for `GET /v1/videos/{task_id}`; first mapping wins.
   Traffic additionally owns the **routing/classifier projection** read from the credential row:
   `router_tokens.order_type` (`VARCHAR(16)`, default `'value'`) and
   `router_tokens.price_cap_nanodollars` (`BIGINT`), added by
   `migrations/0011_alter_router_mvp_columns.sql`. `RouterDatabase::validate_token_and_get_info`
   selects exactly those two columns and `OrderType::from_db_row` maps them to
   `OrderType::{Budget, Value, Enterprise}`. The `router_tokens` credential row itself, its
   `quota_limit` / `used_quota`, `user_accounts`, `user_api_keys`, the Supply channel/ability/
   protocol/model tables, the Commerce billing tables and the Platform schema history are **not**
   Traffic truth.

4. **Public contract** — crate root `lib.rs`:
   - application assembly: `create_router_app(db, jwt_secret)` and
     `create_router_app_with_route_miss(db, jwt_secret, RouteMissResponder)`. The data-plane app
     serves `GET /v1/models`, `GET /api/v1/usage`, `GET /api/v1/usage/models` plus
     `.fallback(proxy_handler)`; the internal app serves `INTERNAL_PREFIX = "/console/internal"`
     routes for health, price sync and circuit-breaker trip-all;
   - `pub use storage::{…}`: `RouterDatabase`, `RouterLog`, `RouterLogModel`, `RouterRequestLog`,
     `RouterRequestLogModel`, `RouterVideoTask`, `RouterVideoTaskModel`, the `RouterToken*` /
     `TokenRotationResult` / `TokenValidationInfo` Identity compatibility re-exports,
     `StoragePolicy`, `UsageStats`, `ModelUsageStats`, `BillingSummary`, `BillingModelSummary`,
     `CandidateInfo`, `FailoverAttempt`, `BalanceModel` and the usage/billing query helpers;
   - `pub use log_service::{RouterLogService, UsageStatsService, BalanceService, BillingService}`,
     `pub use scheduler::SchedulingRequest` and `pub use state::{AppState, RouteMissResponder}`;
   - public modules: `affinity`, `channel_state`, `exchange_rate`, `local_attachment`, `log_service`,
     `model_router`, `order_type`, `passthrough`, `price_sync`, `rate_budget`, `response_parser`,
     `response_quality`, `storage`, `stream_parser`, `token_counter`, `channel_health_manager`,
     `health_probe` and `smart_circuit_breaker`. `adaptor`, `aimd_limiter`, `balancer`,
     `circuit_breaker`, `config`, `limiter`, `scheduler`, `state` and `stream_peek` are private, and
     `proxy_handler` / `proxy_logic` are reached only through the fallback;
   - cross-domain boundary types: `order_type::OrderType` (`from_db_row`, `as_label`,
     `filter_candidates`, `tier_of`, `redundancy`), `model_router::{ModelRouter, RouteInputs,
     RoutingDecision, NoAvailableChannelsError}`, `local_attachment::{LocalRouteAttacher,
     LocalRouteAttachment, LocalRouteAttachmentId, LocalRouteAttachmentError,
     ExistingRouterLocalAttacher}`, `rate_budget::{InMemoryBudget, BudgetBackend, BudgetGuard,
     ConsumeOutcome, ChannelReservation}` and `RouterDatabase`'s Identity delegations (`list_tokens`,
     `create_token`, `delete_token`, `update_token_status`, `validate_token`,
     `validate_token_detailed`, `validate_token_and_get_info`, `update_token_accessed_time`,
     `check_quota`, `deduct_quota`).

5. **Allowed dependencies** — contract crates `burncloud-supply-contracts`,
   `burncloud-commerce-contracts`, `burncloud-traffic-contracts`; platform `burncloud-database`; and
   the cross-domain implementation crates `burncloud-commerce-billing`, `burncloud-identity-user`
   (import name `burncloud_service_user`), `burncloud-identity-token`, `burncloud-supply-channel` and
   `burncloud-supply-model`. `crates/traffic/router/scripts/check-router-deps.sh` enforces the
   compatibility whitelist `ALLOWED_CROSS_DOMAIN_CRATES=( burncloud-commerce-billing
   burncloud-identity-user )` over `burncloud-router` dependencies whose name starts with
   `burncloud-service-` or equals `burncloud-identity-user`; adding another cross-domain
   implementation dependency requires architecture review and an explicit whitelist update.

6. **Allowed consumers** — `burncloud-server` (`crates/interfaces/server`) builds the app with
   `create_router_app_with_route_miss`, initializes `RouterDatabase` and consumes the log, billing
   and local-attachment types; `burncloud-service` (`crates/interfaces/service`) re-exports the crate
   as `router_log`; `burncloud-tests` (`crates/interfaces/tests`) lists it as a dev-dependency; and
   `crates/trust/inference` declares the dependency without referencing `burncloud_router` in its
   `src/`. New consumers outside these should depend on `burncloud-*-contracts` for values and
   require architecture review before depending on this implementation crate.

7. **Internal structure** — request path: `lib.rs` (fallback, `proxy_handler` / `proxy_logic`,
   settlement, sanitization, internal handlers), `model_router.rs`, `order_type.rs`, `affinity.rs`,
   private `scheduler/`, `balancer/`, `channel_state.rs`, `circuit_breaker.rs`,
   `smart_circuit_breaker.rs`, `health_probe.rs`, `channel_health_manager.rs`. Admission:
   `rate_budget.rs`, `aimd_limiter.rs`, `limiter.rs`, `state.rs`. Protocol/parsing: `passthrough.rs`,
   `adaptor/`, `response_parser.rs`, `response_quality.rs`, `stream_parser.rs`, `stream_peek.rs`,
   `token_counter.rs`. Persistence: `storage/mod.rs`, `storage/log.rs` (all
   `router_logs` / `router_request_logs` statements) and `storage/router_video_task.rs`, behind the
   `log_service.rs` facade. Pricing/currency: `price_sync.rs`, `exchange_rate.rs`. Node attachment:
   `local_attachment.rs` (public contract) plus the private `local_attachment_adapter.rs`.

8. **Errors / logs / security** — persistence APIs return `burncloud_database::Result`
   (`DatabaseError`); crate entry points return `anyhow::Result`; domain errors are `thiserror`
   types (`model_router::NoAvailableChannelsError`, `local_attachment::LocalRouteAttachmentError`,
   `pub(crate) scheduler::ScheduleError`). SQL is dialect-adapted (`adapt_sql`, `ph`, `phs`) with all
   values `.bind(...)`-ed. Logging is `tracing` (info/warn/debug/error), including the explicit
   *billing log channel full or closed* and *request settlement failed* failures. Security:
   bearer-token extraction and validation happen before routing; request/response bodies are
   truncated at 64 KB (`MAX_LOG_BODY_SIZE`) and redacted before persistence via `SENSITIVE_HEADERS`
   (`authorization`, `api-key`, `x-api-key`, `x-auth-token`, `cookie`, `set-cookie`) and
   `SENSITIVE_FIELDS` (`api_key`, `apiKey`, `api-key`, `key`, `token`, `password`, `secret`,
   `authorization`); `REQUEST_LOG_STORAGE_POLICY` (`full` / `summary` / `none`, default `summary`)
   controls `router_request_logs`. Upstream `api_key` values are attached to outbound requests and
   must never be logged. `/console/internal/*` carries no auth in this crate and is documented as
   assuming a firewall.

9. **Proof** — `cargo test -p burncloud-router` executes unit tests in `src/` (routing, order type,
   scheduling, rate budget, affinity, circuit breaking, health, parsing, price sync, exchange rate,
   token counting and the protocol adaptors) plus the SQLite integration suites under `tests/`,
   each owning a temporary SQLite file created by `tests/common.rs::setup_db` (`boundary_tests`,
   `pricing_tests`, `e2e_billing_tests`, `cost_roundtrip`, `usage_stats_tests`, `quota_tests`,
   `token_expiry_tests`, `token_paths`, `billing_invariants` with the local-attachment contracts,
   `billing_observability_tests`, `missing_price_fails_closed`, `log_and_balance`,
   `settlement_and_routing_gaps`, `settlement_once`, `l6_observability_tests`, `order_type_e2e`,
   `shaper_tests`, `rate_budget_permits`, `retry_budget`, `failover_tests`,
   `failover_error_reasons`, `price_sync_tests`, `balancer_tests`, `vertex_full_test`,
   `temp_file_leak`, `local_attachment_contract`, `node_existing_router_integration`).
   `adaptor_tests`, `auth_tests` and `token_counting_tests` are environment-gated and print a skip
   message when their keys are unset. PostgreSQL executes
   `tests/postgres_dialect_contract.rs` —
   `router_log_round_trip_and_time_window_use_the_postgres_timestamp_branch`,
   `legacy_log_insert_does_not_write_identity_credential_spend_on_postgres`,
   `token_info_join_quotes_group_and_preserves_the_left_join_on_postgres` and
   `video_task_conflict_keeps_the_first_mapping_on_postgres` — when `BURNCLOUD_TEST_POSTGRES_URL`
   is set, and `github_actions_runs_real_postgres_dialect_contracts` starts a disposable
   `postgres:16` container in CI and re-invokes those contracts, so the dialect branches are
   actually executed rather than counted as skips.

10. **Correct / incorrect calls** — correct: interfaces build the router through
    `create_router_app_with_route_miss`; routing loads candidate channels and prices through the
    owning crates' public capabilities; credential and quota work goes through `RouterDatabase` →
    Identity `TokenService`; Traffic persists only `router_logs`, `router_request_logs` and
    `router_video_tasks`; a Node runtime attaches capabilities through `LocalRouteAttacher` /
    `ExistingRouterLocalAttacher`. Incorrect: issuing raw SQL against `router_logs`,
    `router_request_logs`, `router_video_tasks`, `router_tokens`, `channel_providers`,
    `channel_abilities`, `billing_prices` or `billing_exchange_rates` instead of the owning crate's
    public capability; treating Commerce prices, Identity credential state or Supply channel rows as
    Traffic truth; creating a second routing table or `ModelRouter` for local node routes; or adding
    a cross-domain implementation dependency without a `check-router-deps.sh` whitelist update.

    ```text
    Correct:   Interface/Server -> create_router_app_with_route_miss -> RouterDatabase
               (Identity TokenService + Supply channel capability + Commerce PriceCache/CostCalculator)
    Correct:   Router -> OrderType::from_db_row(router_tokens.order_type, price_cap_nanodollars)
               -> candidate filter -> failover list
    Correct:   Router -> RouterLogModel / RouterRequestLogModel / RouterVideoTaskModel
               -> router_logs / router_request_logs / router_video_tasks
    Correct:   Node runtime -> LocalRouteAttacher (ExistingRouterLocalAttacher)
               -> existing Supply channel/ability truth
    Incorrect: Traffic -> raw SQL against Supply channel/ability, Identity credential,
               or Commerce pricing tables instead of the owning crate's public capability
    Incorrect: Any consumer -> raw SELECT/INSERT/UPDATE on router_logs or router_video_tasks
               instead of the router storage models
    Incorrect: Node runtime -> create a second ModelRouter or a second routing table
    Incorrect: New cross-domain implementation dep without a check-router-deps.sh whitelist update
    ```

## 不拥有

- Commerce price truth (`billing_prices`, `billing_tiered_prices`, `billing_exchange_rates`) and the
  cost-calculation rules in `CostCalculator`
- Identity credential state (`user_accounts`, `user_api_keys`, `router_tokens` `quota_limit` /
  `used_quota`) and wallet balances (`BalanceModel`)
- Supply persistence and SQL for `channel_providers`, `channel_abilities`,
  `channel_protocol_configs` and `model_capabilities`
- Cross-domain DTOs in `burncloud-supply-contracts`, `burncloud-commerce-contracts` and
  `burncloud-traffic-contracts`
- Platform database connection, schema and migration history
- The `router_tokens` table itself; Traffic only owns the `order_type` / `price_cap_nanodollars`
  routing projection read from it

## 本机路由挂载（Ticket 01）

通过 `ExistingRouterLocalAttacher::new(db.clone())` 和 `ModelRouter::new(db)` 复用同一个 `Arc<Database>`；即使连接字符串相同，独立 Database 对象也不是同一实例。

`LocalRouteAttachment { model, base_url }` 仅接受调用方已确认就绪的模型。别名按 1–34 个 Unicode 标量校验，禁止首尾空白、英文逗号及控制字符；保留内部空格、大小写、Unicode 和冒号。地址是 HTTP(S) 回环 IP 的服务根地址，不接受域名、凭证、路径前缀、查询或片段。根斜线与默认端口等价。

首次 attach 返回实际正数 ID；quarantine 保留身份并禁止新许可；detach 仅摘除该 ID，可靠不存在时成功。隔离前已获许可的一次尝试不被取消，重试必须重新准入。归属由固定标签核对，SQL 由 Supply 条件事务执行。未知提交或管理 future 取消继续禁派，不自动重试、不凭查不到就宣称已回滚。未在本实例成功发布的残留不得直接收养，需显式清理。

挂载同时维护生命周期、运行探测心跳、故障锁和每个实际 ID 的并发槽。管理器用 5 秒周期、2 秒端到端超时探测 `/v1/models`；探测失败立即停止新许可，临时故障或心跳过期须连续两次成功才恢复。真实推理故障、显式隔离、未决提交和旧会话不会被探测成功解锁。

本机聊天最终发送到根地址下的 `/v1/chat/completions`，使用专用直连客户端且不跟随重定向，不转发 BurnCloud 请求凭证。两个真实发送分支都在每次尝试前原子取得 `LocalDispatchPermit`；许可随非流式响应或流式响应体生命周期移动，流结束、上游错误、总预算取消及下游取消都会释放本次槽位。候选存在但本机准入拒绝耗尽、且尚未尝试上游时返回 `503 / no_available_channels`，附 `X-Rejected-By: scheduler` 和 `Retry-After: 60`；真正空路由保持 404，已尝试上游的失败保持 502，预算超时保持 504。

本地验证使用独立数据库、真实 handler 与受控 HTTP 上游；它不是实际模型推理证明，也不代替生产 Server 的完整启动装配验收。

## 启动管理器（Ticket 02）

新增公共类型 `LocalRouteManager`，只有 `new(db, attachments)` 与 `run(self)` 两个入口，所有字段私有。构造完整校验并规范化整份清单，不做网络/SQL；规范化后重复目标或同一 Database 对象已存在管理器时返回 AttachFailed。空清单合法。`ExistingRouterLocalAttacher` 与 Router 必须继续使用同一个 Arc 对象。

运行时每个目标首次立即 GET /v1/models，精确匹配别名；整次探测（包含读体）限 2 秒，响应体最多 1 MiB，直连且不跟随重定向。首检失败每 5 秒复探；首检成功后仅尝试一次生产 attach，发布后继续运行心跳。持久化失败记录错误并停止该启动项，数据库恢复或人工清理不会自动重启该项。

管理器不 spawn 脱离任务；调用方须在 Server 结束时丢弃/取消并等待回收 run future。关闭同步禁新许可、不在 Drop 做 SQL、不删除记录。会话代数在等锁前、写入前、发布时检查，旧会话迟到提交不能借新会话激活成为可调度节点。未启动、已停止或旧代挂载不能通过现有两个真实 send 点的生命周期门禁。

重启不自动收养或删除残留，须核对事务终局后显式 detach，再显式 attach 或重新启动。此方案没有跨进程数据库 fencing；异常退出时“暂时查不到行”不证明旧事务不会迟到提交。正常启动入口由 Server 接线；低层 create_app 不接管管理器生命周期。

## 运行准入与调度策略（Ticket 03–06）

运行管理已补齐持续心跳、临时故障两次成功恢复、推理/端点故障锁、迟到反馈身份检查，以及每挂载独立的 `BURNCLOUD_LOCAL_ROUTE_MAX_CONCURRENCY` 并发上限（默认 `1`，正十进制 `u32`，重启生效）。槽位不是排队：满载候选跳过；流式请求直到 EOF、错误或取消才释放。

`SCHEDULER_POLICIES` 的未配置默认值是 `balanced`；显式 `passthrough`/`combined` 保留旧亲和和评分行为。新策略 `balanced`、`stability_first`、`price_first` 作用于全档合格候选：先取最高可用管理员优先级，再分别按正权重、最高有限健康分或最低严格可比较 nano-USD 参考报价选择。同一选择复用权重随机；不因亲和置顶绕过本机准入，不提前截断新策略候选。价格使用输入价与输出价之和，checked `i128`、直接汇率优先、缺直接汇率才允许反向汇率，缺价不是免费。

本轮验证使用 SQLite 与受控 HTTP 上游，覆盖心跳、容量、流式占槽、SSE 跨 chunk 错误、策略资格与五次实际上游发送上限。它不是实际模型推理证明；真实模型仍需单独部署验收。
