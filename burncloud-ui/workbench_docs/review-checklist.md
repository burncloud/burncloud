---
doc_id: ui.review-checklist
doc_type: verification-standard
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
---

# BurnCloud UI Review Checklist v1.0

本清单用于设计评审、Agent 自检、PR Review 和 Golden Page 验收。不是所有项都必须手工检查；可执行的规则应逐步转成 automated gate。

## A. Product Contract

- [ ] 已确认当前 Role：Buyer / Supplier / Admin
- [ ] 已阅读对应 Page Contract
- [ ] 页面能快速回答该契约的 Primary Question
- [ ] 没有增加未经批准的一级概念/菜单
- [ ] 没有复制其它一级页面的完整职责
- [ ] Buyer 没有被迫理解 GPU/Supplier/IDC
- [ ] Supplier 没有获得手工模型部署/Traffic 控制
- [ ] Admin 没有被迫逐 GPU 做日常调度

## B. Information Hierarchy

- [ ] 结论优先于原始数据
- [ ] Primary Action 唯一、明确
- [ ] Secondary/Danger 不与 Primary 竞争
- [ ] 首屏信息数量与角色密度匹配
- [ ] Advanced 信息默认折叠
- [ ] Card 只用于明确逻辑边界

## C. States

- [ ] Initial/First-use 状态完整
- [ ] Loading 保持布局稳定
- [ ] Empty 解释原因和下一步
- [ ] Partial failure 不无条件拖垮整页
- [ ] Error 说明影响与下一步
- [ ] Recovered/Success 说明真实结果
- [ ] Unknown 不伪装成 0 / Healthy / Paid / Ready

## D. Autopilot

- [ ] 低风险正常动作尽量自动完成
- [ ] 已自动恢复的事情不会制造多余人工工单
- [ ] 高风险 Proposal 显示原因、成本、收益、影响
- [ ] Approve/Reject 有明确对象
- [ ] 执行后有 Verify 阶段，不是“API 返回 200 就算成功”

## E. Financial Truth

- [ ] 金额有币种和时间范围
- [ ] Revenue / Earnings / Balance / Settlement 术语正确
- [ ] Estimated / Final 分离
- [ ] Contribution 与 Revenue Share 分离
- [ ] Payable / Paid 分离
- [ ] Gross Margin 在成本不完整时不显示虚假精确值

## F. Visual / Design System

- [ ] Black/White/Gray 为主体，Accent 克制
- [ ] 状态色只用于状态语义
- [ ] 没有大量彩色 KPI 卡
- [ ] Density 合理
- [ ] Typography/spacing 与 Golden Page 一致
- [ ] 图表每张回答明确问题
- [ ] 没有装饰性 Pie/Donut/Radar/Gauge/3D
- [ ] 优先复用现有共享组件

## G. Table / Data

- [ ] 普通页面只提供必要 Search/Filter/Sort
- [ ] Admin Bulk Action 有选择数量/影响预览/确认
- [ ] 内部 ID 默认不占主要列
- [ ] 单位、币种、时间范围明确
- [ ] 排序/过滤不会改变数据语义

## H. Content

- [ ] 文案顺序：What happened → Impact → BurnCloud did → User action
- [ ] Button 使用明确动词
- [ ] 核心状态词与 Content Standard 一致
- [ ] 不使用“Connected/Healthy/Paid”等过度承诺
- [ ] 技术错误码在 Details，不作为主文案

## I. Accessibility / Responsive

- [ ] 键盘可访问
- [ ] 焦点可见
- [ ] 状态不只靠颜色
- [ ] Alert/Loading 有语义
- [ ] 窄屏仍保留结论/状态/Primary CTA
- [ ] 表格不会被压缩到不可读

## J. Security / Privacy

- [ ] Secret 不长期明文显示
- [ ] Buyer 不获得 Supplier credential
- [ ] 列表不默认暴露敏感请求内容
- [ ] 高风险资金/数据动作有确认和审计
- [ ] 跨租户/跨 Supplier 数据权限正确

## K. Verification

- [ ] 编译/类型检查通过
- [ ] 功能路径真实可执行
- [ ] 适用 UI/Product gate 通过
- [ ] Visual QA 已检查主要 viewport
- [ ] Error/Empty/Loading 已实际验证
- [ ] 最终 Diff 没有无关 UI 漂移
- [ ] 改变的产品真相已同步对应文档

## Definition of Done

页面不是因为“代码写完、编译通过、截图好看”而完成。只有当对应 Page Contract 的 Success Condition 可以由真实行为证明、适用 Review Checklist 通过、且没有越过 Product Gate 时，才可声明 UI Done。