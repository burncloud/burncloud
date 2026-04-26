# scripts/analysis

只读 ad-hoc 数据分析脚本。所有查询不修改数据库，可直接对生产 / staging DB 安全运行。

## 当前脚本

| 名字 | 用途 | 运行频率 |
|-----|------|---------|
| [`affinity_drift.sql`](./affinity_drift.sql) | 实证 issue #143「5 用户 2 渠道摇摆」P1 痛点是否真实存在 | MVP 实施前**一次性前置**；如样本不足每周复跑 |
| [`AFFINITY_DRIFT_REPORT.md`](./AFFINITY_DRIFT_REPORT.md) | 上述分析的报告模板（方法论 + 决策门槛 + 填空表格） | 跑完 SQL 后填表 |

## 怎么跑

### SQLite（本地 / dev）

```bash
sqlite3 ./data/burncloud.db < scripts/analysis/affinity_drift.sql
```

### PostgreSQL（生产）

打开 `affinity_drift.sql`，把每个查询块的 `---- SQLite ----` 段注释掉，把 `---- PostgreSQL ----` 段取消注释，然后：

```bash
psql -d burncloud -f scripts/analysis/affinity_drift.sql
```

或逐个查询用 psql 交互模式跑（推荐，便于看中间结果）。

## 决策门槛

跑完后把数字填到 `AFFINITY_DRIFT_REPORT.md` 的"待填结果"段。报告里的决策表会告诉你 GO / NO-GO / 灰区。

GO → 子任务 6（MVP 调度器分层改造）解除阻塞。
NO-GO → 转方案 D（customer discovery），不启动 MVP。
