---
doc_id: ui.page-contract.buyer-playground
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: playground
---

# Buyer Playground Page Contract v1.0

## Purpose

Playground 是 Buyer 的真实模型试用与接入验证入口，不是与生产系统脱节的 Demo。

## User Goal

> 我想快速确认某个模型和性能档位是否满足需求，并把成功的测试变成真实 API 调用。

## Primary Question

> 这个模型现在能不能满足我的需求？

## Core Flow

```text
Select Model
→ Select Tier
→ Enter Request
→ Send Real Request
→ Inspect Result
→ See Latency / Usage / Cost
→ Copy API Example
```

## Primary Information

- Model
- Tier: Economy / Standard / Performance，默认 Standard
- Request input
- Real response
- Outcome
- Latency
- Token usage
- Estimated / actual request cost when available

## Primary Action

`Send Test Request`

发送前若缺少余额、模型不可用或账户不具备调用条件，应明确阻止并给出唯一可执行下一步。

## Secondary Actions

- Change model
- Change tier
- Reset request
- Copy API example
- Open model in Marketplace
- Create / choose API Key when needed

## Production Truth Rule

Playground 必须调用与真实 Buyer API 同一服务能力或等价受控路径。禁止出现“Playground 成功，但生产 API 因完全不同的路由或能力而不可用”的假成功。

## Empty / First-use State

首次进入时给出最小可运行示例，而不是空白编辑器。若用户还没有模型选择，推荐从 Marketplace 选择或提供清晰默认模型。

## Loading

长响应或流式响应应展示真实执行状态。发送期间避免重复提交；流式内容应逐步呈现而不是冻结整页。

## Error / Needs Attention

错误优先表达 Buyer 可理解的结论：

```text
Request could not be completed.
The selected model is temporarily unavailable.
BurnCloud is routing traffic to healthy capacity; try again shortly.
```

不要要求 Buyer 选择 GPU、Supplier、Worker 或内部 Route。

## Autopilot

Provider / GPU / Deployment / Route 选择由 BurnCloud 自动完成。Playground 只允许 Buyer 选择产品级 Model 和 Tier。

## Advanced

可逐步提供：

- Model parameters
- Response headers relevant to Buyer
- Streaming toggle
- Structured output / tool options when supported
- Request / response JSON
- API snippets for supported SDKs

Advanced 不暴露内部供应商凭证和调度拓扑。

## Intentionally Hidden

- GPU model / count
- Supplier identity
- IDC
- Deployment topology
- Scheduler score
- Internal route IDs
- Supplier credentials

## Success Condition

Buyer 能在几分钟内完成一次真实模型调用，理解结果、成本和性能，并直接获得可迁移到生产代码的 API 示例。

## Verification Checklist

- [ ] Model 与 Tier 可明确选择，默认 Standard
- [ ] Send Test Request 是唯一明显 Primary CTA
- [ ] 请求走真实可用能力
- [ ] 余额不足 / 模型不可用会阻止不可能流程
- [ ] 成功后展示 Outcome、Latency、Usage
- [ ] API 示例与当前选择一致
- [ ] 错误不会泄露内部 GPU / Supplier 信息
- [ ] Partial / streaming response 状态正确
- [ ] Unknown 不显示为成功

## Product Gate

以下变化必须 Product Gate：把 GPU/Supplier 选择暴露给 Buyer；使用与生产完全不同的模拟后端；新增与 Marketplace 重复的采购功能；改变 Tier Mental Model。

## Final Rule

Playground 永远应该让 Buyer 验证“模型 API 是否满足需求”，而不是让 Buyer 调试 BurnCloud 的底层基础设施。