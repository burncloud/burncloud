---
doc_id: ui.page-contract.buyer-api-keys
doc_type: page-contract
truth: target
status: approved
version: 1.0
parent:
  - docs/ui/product-standard.md
  - docs/ui/information-architecture.md
role: buyer
page: api-keys
---

# Buyer API Keys Page Contract v1.0

## Purpose

API Keys 管理 Buyer 访问 BurnCloud API 的凭证。Buyer 拿到的是 BurnCloud Credential，永远不是底层 Supplier Key。

## User Goal

> 我想安全地创建、识别、限制、轮换和撤销我的 API 凭证。

## Primary Question

> 我现在有哪些 BurnCloud API Key，它们是否安全可用？

## Primary Information

每个 Key 默认展示：

- Human-readable name
- Opaque management reference / fingerprint
- Created time
- Last used time when available
- Status
- Scope / spend limit when supported

不得通过“看起来像被遮住的 Secret”暗示平台可以再次恢复已隐藏的 bearer secret。

## Primary Action

`Create API Key`

创建完成后，一次性展示 bearer secret，并明确要求用户保存。关闭确认后不得再次显示完整 Secret。

## Secondary Actions

- Rename
- Rotate
- Revoke / Delete
- Configure supported spend limit / scope
- Copy management reference

危险动作必须明确确认。

## Empty State

```text
No API keys yet
Create a BurnCloud API key to call models from your application.
[Create API Key]
```

## Security States

- Active
- Rotating
- Revoked
- Expired（如支持）

创建 Secret 与管理引用必须概念分离。

## Error

如果后端不支持某项能力（例如特定 CIDR、Scope 或创建方式），UI 必须明确不可用，不能展示假控件或本地保存一个服务器不认识的设置。

## Autopilot

底层 Supplier 凭证、模型部署和路由凭证由 BurnCloud 管理；Buyer API Key 只授权 Buyer 访问 BurnCloud 产品能力。

## Intentionally Hidden

- Supplier keys / credentials
- Internal capability signing material
- GPU / Worker identity
- Internal route tokens
- Other customers' credentials

## Success Condition

Buyer 能安全创建一个 Key、只在创建时看到 Secret、理解每个现存 Key 的用途，并能在泄露或轮换场景下快速撤销旧凭证。

## Verification Checklist

- [ ] `Create API Key` 是主要 CTA
- [ ] Secret 只在允许的一次性场景展示
- [ ] Management reference 不伪装成 masked secret
- [ ] Rotate / Delete 有明确危险确认
- [ ] 后端不支持的能力不会出现假控件
- [ ] Status 与服务器真实状态一致
- [ ] 不泄露 Supplier / internal credential
- [ ] 空状态直接引导创建第一个 Key

## Product Gate

以下变化必须 Product Gate / Security Review：让 Buyer 获得 Supplier Key；允许恢复历史完整 Secret；改变 Credential 权限模型；新增高风险 Scope。

## Final Rule

API Keys 页面管理的是 Buyer 对 BurnCloud 的安全访问权，而不是暴露 BurnCloud 如何访问底层供应商。