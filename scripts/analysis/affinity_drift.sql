-- ============================================================================
-- 亲和漂移（Affinity Drift）分析查询集
-- ----------------------------------------------------------------------------
-- 用途：实证 issue #143 宣称的「5 用户 2 渠道摇摆」痛点是否真实存在并量化。
--       作为 MVP 调度器分层改造（亲和层 + OrderType + 染色）的前置数据验证。
-- 决策门槛：见 scripts/analysis/AFFINITY_DRIFT_REPORT.md
--
-- 表：router_logs
-- 关键字段：
--   user_id      TEXT NULL     用户标识（NULL 表示匿名/系统请求，需过滤）
--   upstream_id  TEXT NULL     上游 channel 字符串 ID（"channel_id" 在表里叫这个名字）
--   model        TEXT NULL     模型名（亲和必须按 model 维度分析，跨 model 走不同渠道是正常路由）
--   status_code  INTEGER       HTTP 状态码（仅成功请求 2xx 计入摇摆，4xx/5xx 排除）
--   created_at   TEXT/TIMESTAMP 时间戳（sqlite TEXT，postgres TIMESTAMP）
--
-- 已有索引：idx_router_logs_user_id, idx_router_logs_model, idx_router_logs_created_at
--
-- 使用方法：
--   sqlite3   ./data/burncloud.db < scripts/analysis/affinity_drift.sql
--   psql -d burncloud -f scripts/analysis/affinity_drift.sql
--
-- 约定：N = 7（最近 7 天）。如需调整窗口，搜索 "WINDOW = 7" 改两处。
-- ============================================================================


-- ============================================================================
-- Q1：基础规模 — 这次分析覆盖多少数据？
-- ----------------------------------------------------------------------------
-- 解读：如果 distinct_users < 10 或 total_requests < 1000，说明样本量太小，
--       本次分析结论仅供参考，应先放量收集更多数据再决定 MVP 是否启动。
-- ============================================================================

-- ---- SQLite ----
SELECT
    COUNT(*)                                    AS total_requests,
    COUNT(DISTINCT user_id)                     AS distinct_users,
    COUNT(DISTINCT upstream_id)                 AS distinct_upstreams,
    COUNT(DISTINCT model)                       AS distinct_models,
    MIN(created_at)                             AS window_start,
    MAX(created_at)                             AS window_end
FROM router_logs
WHERE created_at >= datetime('now', '-7 days')  -- WINDOW = 7
  AND status_code >= 200 AND status_code < 300
  AND user_id IS NOT NULL
  AND upstream_id IS NOT NULL
  AND model IS NOT NULL;

-- ---- PostgreSQL ----
-- SELECT
--     COUNT(*)                                    AS total_requests,
--     COUNT(DISTINCT user_id)                     AS distinct_users,
--     COUNT(DISTINCT upstream_id)                 AS distinct_upstreams,
--     COUNT(DISTINCT model)                       AS distinct_models,
--     MIN(created_at)                             AS window_start,
--     MAX(created_at)                             AS window_end
-- FROM router_logs
-- WHERE created_at >= NOW() - INTERVAL '7 days'    -- WINDOW = 7
--   AND status_code BETWEEN 200 AND 299
--   AND user_id IS NOT NULL
--   AND upstream_id IS NOT NULL
--   AND model IS NOT NULL;


-- ============================================================================
-- Q2：核心查询 — 每 (user_id, model) 被路由到了几个 channel？
-- ----------------------------------------------------------------------------
-- 这是 issue 描述中 SELECT user_id, channel_id, COUNT(*) GROUP BY 的精确版本。
-- 输出：每行是一个 (user, model, channel) 三元组及命中次数。
-- 后续查询从这张表派生。
--
-- 体量提示：行数约等于 distinct_users × avg_models_per_user × avg_channels_per_user_model。
--           几千行内可直接看；上万行后应改用 Q3-Q5 的聚合视图。
-- ============================================================================

-- ---- SQLite ----
SELECT
    user_id,
    model,
    upstream_id                                 AS channel_id,
    COUNT(*)                                    AS hits
FROM router_logs
WHERE created_at >= datetime('now', '-7 days')
  AND status_code >= 200 AND status_code < 300
  AND user_id IS NOT NULL
  AND upstream_id IS NOT NULL
  AND model IS NOT NULL
GROUP BY user_id, model, upstream_id
ORDER BY user_id, model, hits DESC;

-- ---- PostgreSQL ----
-- 同上，把 datetime('now', '-7 days') 换成 NOW() - INTERVAL '7 days'。


-- ============================================================================
-- Q3：(user, model) 命中 channel 数的直方图 — 摇摆"广度"
-- ----------------------------------------------------------------------------
-- 解读：
--   channels_used = 1 → 完美亲和（这部分用户已经稳定在单 channel）
--   channels_used = 2 → 轻度摇摆（issue 描述场景）
--   channels_used ≥ 3 → 严重摇摆
--
-- 决策门槛（见 REPORT.md）：
--   若 channels_used ≥ 2 的 (user, model) 对占比 > 30%，确认 P1 痛点存在
--   若占比 < 10%，P1 不成立，MVP 中亲和层 ROI 可能不如预期
-- ============================================================================

-- ---- SQLite ----
WITH per_user_model AS (
    SELECT user_id, model, COUNT(DISTINCT upstream_id) AS channels_used
    FROM router_logs
    WHERE created_at >= datetime('now', '-7 days')
      AND status_code >= 200 AND status_code < 300
      AND user_id IS NOT NULL
      AND upstream_id IS NOT NULL
      AND model IS NOT NULL
    GROUP BY user_id, model
)
SELECT
    channels_used,
    COUNT(*)                                                            AS user_model_pairs,
    ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 2)                  AS pct_of_total
FROM per_user_model
GROUP BY channels_used
ORDER BY channels_used;

-- ---- PostgreSQL ----
-- 同上，把 datetime('now', '-7 days') 换成 NOW() - INTERVAL '7 days'。


-- ============================================================================
-- Q4：(user, model) 主导 channel 占比分布 — 摇摆"深度"
-- ----------------------------------------------------------------------------
-- 解读：dominant_share = 主导 channel 命中数 / 总命中数。
--   1.0  → 完美亲和
--   ≥0.9 → issue MVP 验证目标（"10 次调用命中同 channel ≥ 9 次"）
--   <0.5 → 严重摇摆（用户在 2 个 channel 间几乎均分）
--
-- 输出五分位（P25 / P50 / P75 / P90）+ 命中目标的 (user, model) 对占比。
-- 决策门槛：P50 dominant_share < 0.7 → 强烈支持 MVP 启动
--          P50 dominant_share > 0.95 → 当前调度器已经"自然亲和"，MVP ROI 低
-- ============================================================================

-- ---- SQLite ----
WITH per_triple AS (
    SELECT user_id, model, upstream_id, COUNT(*) AS hits
    FROM router_logs
    WHERE created_at >= datetime('now', '-7 days')
      AND status_code >= 200 AND status_code < 300
      AND user_id IS NOT NULL
      AND upstream_id IS NOT NULL
      AND model IS NOT NULL
    GROUP BY user_id, model, upstream_id
),
per_pair AS (
    SELECT
        user_id,
        model,
        SUM(hits)                                       AS total_hits,
        MAX(hits)                                       AS dominant_hits,
        CAST(MAX(hits) AS REAL) / SUM(hits)             AS dominant_share
    FROM per_triple
    GROUP BY user_id, model
    HAVING SUM(hits) >= 5  -- 排除请求量过低的样本
)
SELECT
    COUNT(*)                                                AS analyzed_pairs,
    ROUND(AVG(dominant_share), 4)                           AS mean_share,
    -- SQLite 没有 PERCENTILE_CONT，用窗口手算（小样本时也准确）
    -- 若数据量大，建议改用 Q4-LARGE（PG 的 PERCENTILE_CONT 写法）
    ROUND((
        SELECT dominant_share FROM per_pair
        ORDER BY dominant_share LIMIT 1
        OFFSET (SELECT CAST(COUNT(*) * 0.50 AS INT) FROM per_pair)
    ), 4)                                                   AS p50_share,
    ROUND((
        SELECT dominant_share FROM per_pair
        ORDER BY dominant_share LIMIT 1
        OFFSET (SELECT CAST(COUNT(*) * 0.25 AS INT) FROM per_pair)
    ), 4)                                                   AS p25_share,
    SUM(CASE WHEN dominant_share >= 0.9 THEN 1 ELSE 0 END)  AS pairs_meeting_mvp_target,
    ROUND(100.0 * SUM(CASE WHEN dominant_share >= 0.9 THEN 1 ELSE 0 END) / COUNT(*), 2)
                                                            AS pct_meeting_mvp_target,
    SUM(CASE WHEN dominant_share < 0.5 THEN 1 ELSE 0 END)   AS pairs_severely_split
FROM per_pair;

-- ---- PostgreSQL（推荐：原生 PERCENTILE_CONT 更准确） ----
-- WITH per_triple AS (
--     SELECT user_id, model, upstream_id, COUNT(*) AS hits
--     FROM router_logs
--     WHERE created_at >= NOW() - INTERVAL '7 days'
--       AND status_code BETWEEN 200 AND 299
--       AND user_id IS NOT NULL
--       AND upstream_id IS NOT NULL
--       AND model IS NOT NULL
--     GROUP BY user_id, model, upstream_id
-- ),
-- per_pair AS (
--     SELECT
--         user_id, model,
--         SUM(hits) AS total_hits,
--         MAX(hits) AS dominant_hits,
--         MAX(hits)::REAL / SUM(hits) AS dominant_share
--     FROM per_triple
--     GROUP BY user_id, model
--     HAVING SUM(hits) >= 5
-- )
-- SELECT
--     COUNT(*)                                                            AS analyzed_pairs,
--     ROUND(AVG(dominant_share)::numeric, 4)                              AS mean_share,
--     ROUND(PERCENTILE_CONT(0.25) WITHIN GROUP (ORDER BY dominant_share)::numeric, 4) AS p25_share,
--     ROUND(PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY dominant_share)::numeric, 4) AS p50_share,
--     ROUND(PERCENTILE_CONT(0.75) WITHIN GROUP (ORDER BY dominant_share)::numeric, 4) AS p75_share,
--     ROUND(PERCENTILE_CONT(0.90) WITHIN GROUP (ORDER BY dominant_share)::numeric, 4) AS p90_share,
--     SUM((dominant_share >= 0.9)::int)                                   AS pairs_meeting_mvp_target,
--     ROUND(100.0 * SUM((dominant_share >= 0.9)::int) / COUNT(*), 2)      AS pct_meeting_mvp_target,
--     SUM((dominant_share < 0.5)::int)                                    AS pairs_severely_split
-- FROM per_pair;


-- ============================================================================
-- Q5：HHI 集中度指数（Herfindahl-Hirschman Index）— 综合分散度
-- ----------------------------------------------------------------------------
-- HHI = SUM((channel_share)^2) for each (user, model)。
--   HHI = 1.0   → 全部流量集中在一个 channel（完美亲和）
--   HHI = 0.5   → 流量在 2 个 channel 间均分（典型摇摆）
--   HHI ≤ 1/n   → 在 n 个 channel 间完全均匀分散
--
-- 经济学/反垄断里 HHI > 0.25 视为高集中度市场。
-- 这里反过来用：HHI < 0.5 视为"摇摆中"（要靠亲和层去解决）。
-- ============================================================================

-- ---- SQLite ----
WITH per_triple AS (
    SELECT user_id, model, upstream_id, COUNT(*) AS hits
    FROM router_logs
    WHERE created_at >= datetime('now', '-7 days')
      AND status_code >= 200 AND status_code < 300
      AND user_id IS NOT NULL
      AND upstream_id IS NOT NULL
      AND model IS NOT NULL
    GROUP BY user_id, model, upstream_id
),
per_pair_share AS (
    SELECT
        user_id,
        model,
        upstream_id,
        CAST(hits AS REAL) / SUM(hits) OVER (PARTITION BY user_id, model) AS share
    FROM per_triple
),
per_pair_hhi AS (
    SELECT
        user_id,
        model,
        SUM(share * share) AS hhi
    FROM per_pair_share
    GROUP BY user_id, model
    HAVING COUNT(*) >= 1
)
SELECT
    COUNT(*)                                                AS analyzed_pairs,
    ROUND(AVG(hhi), 4)                                      AS mean_hhi,
    SUM(CASE WHEN hhi = 1.0 THEN 1 ELSE 0 END)              AS pairs_perfectly_affine,
    SUM(CASE WHEN hhi >= 0.8 AND hhi < 1.0 THEN 1 ELSE 0 END) AS pairs_mostly_affine,
    SUM(CASE WHEN hhi >= 0.5 AND hhi < 0.8 THEN 1 ELSE 0 END) AS pairs_drifting,
    SUM(CASE WHEN hhi < 0.5 THEN 1 ELSE 0 END)              AS pairs_severely_drifting
FROM per_pair_hhi;

-- ---- PostgreSQL ----
-- 同上 SQL 在 PostgreSQL 也能跑（仅需把时间过滤改为 NOW() - INTERVAL '7 days'）。


-- ============================================================================
-- Q6：摇摆用户清单 — 找出"5 用户 2 渠道"的典型实例
-- ----------------------------------------------------------------------------
-- 用途：拿到具体的 user_id + model + channel 分布，作为 issue P1 的实证证据。
-- 输出每个 (user, model) 的渠道命中分布数组（concat 形式），方便手工 sanity check。
--
-- 过滤条件：channels_used >= 2 AND total_hits >= 10（避免随机噪音）。
-- ============================================================================

-- ---- SQLite ----
WITH per_triple AS (
    SELECT user_id, model, upstream_id, COUNT(*) AS hits
    FROM router_logs
    WHERE created_at >= datetime('now', '-7 days')
      AND status_code >= 200 AND status_code < 300
      AND user_id IS NOT NULL
      AND upstream_id IS NOT NULL
      AND model IS NOT NULL
    GROUP BY user_id, model, upstream_id
),
per_pair AS (
    SELECT
        user_id,
        model,
        COUNT(*) AS channels_used,
        SUM(hits) AS total_hits,
        GROUP_CONCAT(upstream_id || ':' || hits, ', ') AS channel_distribution
    FROM per_triple
    GROUP BY user_id, model
    HAVING COUNT(*) >= 2
       AND SUM(hits) >= 10
)
SELECT
    user_id,
    model,
    channels_used,
    total_hits,
    channel_distribution
FROM per_pair
ORDER BY total_hits DESC, channels_used DESC
LIMIT 50;

-- ---- PostgreSQL ----
-- 同上，把 GROUP_CONCAT(...) 换成 STRING_AGG(upstream_id || ':' || hits, ', ' ORDER BY hits DESC)
-- 并把时间过滤改为 NOW() - INTERVAL '7 days'。


-- ============================================================================
-- Q7（可选）：时间窗口对比 — 摇摆是恶化中还是稳定？
-- ----------------------------------------------------------------------------
-- 比较最近 7 天 vs 上 7 天（即 8-14 天前）的平均 dominant_share。
-- 趋势恶化（最近 7 天 share 下降）比稳态摇摆更紧迫，应优先 MVP。
-- ============================================================================

-- ---- SQLite ----
WITH window_data AS (
    SELECT
        user_id, model, upstream_id, COUNT(*) AS hits,
        CASE
            WHEN created_at >= datetime('now', '-7 days') THEN 'recent_7d'
            WHEN created_at >= datetime('now', '-14 days') THEN 'prior_7d'
        END AS window_label
    FROM router_logs
    WHERE created_at >= datetime('now', '-14 days')
      AND status_code >= 200 AND status_code < 300
      AND user_id IS NOT NULL
      AND upstream_id IS NOT NULL
      AND model IS NOT NULL
    GROUP BY user_id, model, upstream_id, window_label
),
per_pair_share AS (
    SELECT
        window_label,
        user_id,
        model,
        CAST(MAX(hits) AS REAL) / SUM(hits) AS dominant_share
    FROM window_data
    WHERE window_label IS NOT NULL
    GROUP BY window_label, user_id, model
    HAVING SUM(hits) >= 5
)
SELECT
    window_label,
    COUNT(*)                                                AS analyzed_pairs,
    ROUND(AVG(dominant_share), 4)                           AS mean_share,
    SUM(CASE WHEN dominant_share < 0.5 THEN 1 ELSE 0 END)   AS pairs_severely_split
FROM per_pair_share
GROUP BY window_label
ORDER BY window_label;

-- ---- PostgreSQL ----
-- 同上，把 datetime('now', '-N days') 全换成 NOW() - INTERVAL 'N days'。


-- ============================================================================
-- 完。结果汇总到 scripts/analysis/AFFINITY_DRIFT_REPORT.md 的"待填表格"。
-- ============================================================================
