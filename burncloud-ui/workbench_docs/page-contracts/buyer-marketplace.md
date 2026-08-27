---
doc_id: ui.page-contract.buyer-marketplace
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: marketplace
---

# Buyer Marketplace Page Contract v1.0

## Purpose

Marketplace 是 BurnCloud 面向 Buyer 的模型商品发现入口。商品单位是 **Model API Capacity**，不是 GPU、Key、IDC 或 Supplier。

## User Goal

> 我想快速找到适合业务的模型，理解价格和服务质量，并开始使用。

## Primary Question

> BurnCloud 有哪些模型可以用，哪个适合我？

## Default Experience

默认像克制的模型商店，而不是云基础设施控制台。列表/卡片优先展示：

- Model name
- 一句话能力说明
- Price
- Availability
- Supported tiers
- Context / key capability summary when useful
- `Use Model` CTA

存在官方默认 API 定价时跟随官方默认定价；不存在时展示 BurnCloud 统一定价。价格来源应可解释，但默认不需要展开底层成本。

## Primary Action

`Use Model`

进入模型详情后可直接跳 Playground 或接入流程。不要先要求 Buyer 配置 GPU / Provider / Route。

## Discovery

允许：

- Search
- 简单 Filter（能力、模态、价格范围等）
- Sort（价格、推荐、可用性等）
- Recommended / Popular 等克制策展

默认不要堆大量筛选器。

## Model Detail

默认详情优先回答：

```text
What is it good at?
How much does it cost?
How reliable is it?
Which tier should I use?
How do I start?
```

支持 Tier：Economy / Standard / Performance，默认 Standard。

## Advanced

点击 `Advanced` 才允许展开：

- Real-time latency
- Historical availability
- Current load
- Context length
- Model version
- Benchmark
- Region
- Tier differences
- Compatibility notes

即使 Advanced 也默认不展示供应商商业身份。

## Empty / Unavailable

无搜索结果时提供清除筛选或相近模型建议。模型暂不可售时明确区分：`Unavailable`、`Capacity constrained`、`Deprecated`，不能伪装为可购买。

## Autopilot

Buyer 选择 Model/Tier 后，具体 GPU、Supplier、Deployment、Route 由 BurnCloud 自动决定。自动路由失败时 Marketplace 不要求用户转入底层供应商选择。

## Intentionally Hidden

- GPU model / count
- Supplier count / commercial identity
- IDC name
- Worker inventory
- Internal deployment topology
- Internal routing weights
- Supplier revenue share
- External rental details

## Success Condition

新 Buyer 能在 1–2 分钟内找到模型、理解价格与 Tier，并进入真实测试或 API 使用流程，而不需要理解任何 GPU 编排概念。

## Verification Checklist

- [ ] 商品单位是 Model API Capacity
- [ ] 默认展示价格、Availability、Tier 和能力，而非 GPU
- [ ] Standard 是默认 Tier
- [ ] `Use Model` 是详情页主要 CTA
- [ ] Search / Filter / Sort 简洁可用
- [ ] Advanced 信息与默认层分离
- [ ] 不暴露 Supplier 商业身份和内部拓扑
- [ ] 不可售状态真实且可理解
- [ ] 官方定价 / BurnCloud 定价来源语义一致

## Product Gate

以下变化必须 Product Gate：把 Marketplace 改成 GPU Marketplace；让 Supplier 自行向 Buyer 定价；默认暴露 Supplier；修改 Tier 体系；改变“Buyer 购买模型能力”的产品定义。

## Final Rule

Marketplace 应让 Buyer 感觉自己在选择稳定、清晰定价的大模型 API，而不是在逛 GPU 服务器市场。