---
doc_id: ui.interaction-rules
doc_type: interaction-standard
truth: target
status: approved
version: 1.0
parent: docs/ui/product-standard.md
---

# BurnCloud Interaction Rules v1.0

## 1. Simple by Default

默认只展示完成当前任务所需的信息和操作。专业能力进入 `Advanced / Details / Inspect`，不能因为后端有字段就直接暴露控件。

## 2. One Primary Action

每个页面/主要状态尽量只有一个明显 Primary CTA。若状态改变（例如余额 Critical），Primary CTA 可以改变，但不要并列多个同等级按钮。

## 3. State Completeness

所有重要页面/组件必须考虑：Initial、Loading、Ready、Empty、Partial failure、Error、Success/Recovered、Disabled/Unavailable（如适用）。Happy path 不是完整 UI。

## 4. Empty State

Empty State 必须解释：为什么为空 + 下一步做什么。禁止只写 `No data`。新用户与系统异常不能共用同一个空状态。

## 5. Loading

保持布局稳定，优先 Skeleton 或真实阶段。长任务显示真实进度阶段，例如 Benchmarking → Preparing runtime → Deploying → Verifying，而不是无限 Spinner。

## 6. Partial Failure

一个子 API 失败时，已确认的其它数据应尽量保留。Unknown / unavailable 不得转成 0、Healthy、Success。页面要说明哪部分不可用。

## 7. Error Copy

��序：发生了什么 → 影响什么 → BurnCloud 已经做了什么 → 用户是否需要行动。内部错误码放 Details，不作为主要文案。

## 8. Autopilot Interaction

正常低风险行为：自动执行并报告结果。

```text
Observe → Predict → Decide → Act → Verify → Report
```

不要把每个自动动作都变成确认弹窗。

高风险行为（大额外租、支付、合同/分成、危险数据操作、安全策略等）才进入 Human Gate，并显示：原因、预期收益、成本/风险、影响范围、可否回滚。

## 9. Approval

Approve/Reject 必须是对明确 proposal 的决策，不是模糊的“Allow AI”。批准后记录 actor/time/input/expected impact/result。

## 10. Dangerous Actions

Delete / destructive reset / force offline / high-risk financial action 必须：Danger Zone、清晰对象、不可逆影响、必要确认、避免误点。可 Graceful 的操作优先 Graceful，而不是 Force。

## 11. Graceful Resource Offline

Supplier Resource 正常下线：Request → Drain → Stop new work → Finish work → Release deployment → Offline。UI 应显示阶段；Force/Unexpected offline 是不同路径并可能影响 Reliability。

## 12. Forms

只要求完成任务需要的字段；Advanced 字段折叠。Server validation 是最终真相；本地验证只做快速反馈。���存失败必须指出哪些值未生效。

## 13. Tables

Search/Filter/Sort 不应打断任务流。Bulk actions 仅 Admin 高级场景；批量高风险动作必须先显示 selection count / impact / total amount 等预览。

## 14. Navigation

任务完成后应回到用户 Mental Model：Buyer 回模型/API/消费，Supplier 回资源/收益，Admin 回 Supply/Capacity/Demand/Economics。内部实现对象不能成为默认导航目的地。

## 15. Cross-page Linking

页面只保留自己的职责，复杂问题通过深链接到 owner page。例如 Overview → Capacity risk → Admin Capacity；Usage anomaly → Logs；Supplier earning question → Earnings/Settlements。

## 16. Notifications

通知只用于真正需要关注或确认的状态变化。Autopilot 已成功恢复的事件可作为信息记录，不应持续制造红色告警。

## 17. Success

成功反馈应说明结果，而不是只显示 `Success`。例如：`Capacity restored. BurnCloud added temporary capacity; estimated margin impact -1.8%.`

## 18. Interaction Gate

新增全局交互 Pattern、改变危险确认方式、把自动动作改为人工/把人工高风险动作改为自动，都需要 Design/Product/Risk Gate。