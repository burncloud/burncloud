---
doc_id: agent.issue-standard
doc_type: agent-protocol
truth: normative
status: active
---

# BurnCloud Issue 标准

Issue 是进入 BurnCloud 工程执行系统的第一份**任务权限合同**。它不是简单描述“想做什么”，而是把一个需求压缩成 **单一、可验证、可审查、可停止、可转换成 Task Contract 的工程任务**。

Issue 本身不是事实来源。当前行为必须由源码、测试、运行证据或已接受规范证明；证据不足时必须标记 `UNKNOWN`，不能把计划、聊天记录或 AI 推断写成当前事实。

最重要的规则：

> **PLANNED 不等于可以编码。Codex 只能实现已经通过 READY Gate 的 Issue。**

## 1. 核心原则

每个工程 Issue 必须满足：

1. **Single Outcome** — 一个 Issue 只交付一个主要可观察结果。
2. **Evidence First** — 当前行为必须引用证据，不能依赖聊天记忆。
3. **Known Entry** — 必须知道从哪个真实入口或源码起点开始调查。
4. **Reuse Before Create** — 必须先声明应复用的现有组件，再允许新增抽象。
5. **Bounded Scope** — 明确允许修改的领域，以及默认禁止跨越的边界。
6. **Explicit Contract** — 对跨组件能力明确输入、输出和稳定语义，而不是让实现者自行命名和解释。
7. **Explicit Failure** — 失败行为必须定义；不得用静默 fallback 掩盖失败。
8. **Invariant Aware** — 列出会受到影响的不变量；如果不知道，先调查。
9. **Stop Instead of Widen** — 一旦需要越权修改，停止并报告，不得为了完成 Issue 自动扩大范围。
10. **Verifiable Done** — 完成条件必须可以通过测试、运行路径或明确检查验证。
11. **No Hidden Architecture Change** — 架构或 invariant 变化必须独立暴露，不得伪装成普通功能实现。
12. **No Bundle Issues** — 不把多个独立能力、多个 Phase 或“顺手重构”塞进同一个 Issue。

## 2. 三层职责：Plan、Issue、Task Contract

三层不能互相替代：

```text
Implementation Plan
“未来准备做什么”
      ↓
Evidence Audit
      ↓
READY Engineering Issue
“批准实现什么、边界是什么、什么情况下必须停止”
      ↓
Task Contract
“基于当前 main，具体从哪里改、真实执行路径和验证目标是什么”
      ↓
Codex / Coding Agent
      ↓
Candidate Patch
      ↓
Pull Request
      ↓
Verification / Review
```

### Implementation Plan

可以包含长期目标、类别、依赖和 `PLANNED` 能力，但不能被当作当前实现事实，也不能直接授权 Codex 编码。

### Engineering Issue

定义任务目标、权限边界、稳定合同、失败语义、验证目标和停止条件。只有 `READY` Issue 才具有实现授权。

### Task Contract

Codex 在真正修改代码前，必须基于**当前 `main`**重新核实入口、执行路径、源码证据和测试位置。Issue 中的事实如果已经过期，必须停止或更新合同，而不是强行实现旧计划。

## 3. Issue 必填结构

每个非平凡工程 Issue 至少包含以下部分。

### 3.1 标题

格式：

```text
[domain] observable outcome
```

示例：

```text
[node] establish canonical HardwareProfile
[node] detect NVIDIA GPU and VRAM
[node] register READY local runtime as existing Channel
```

标题描述结果，不描述文件名，也不要使用 `misc`、`cleanup`、`improve` 等不可验证词语。

### 3.2 目标（Goal）

说明用户、运维人员或系统最终能够观察到什么。只允许一个主要结果。

### 3.3 当前事实与证据（Current Evidence）

必须引用当前源码、测试、运行路径或已接受规范。

统一证据分类：

- `STATIC CONFIRMED`
- `DYNAMIC`
- `INFERRED`
- `UNKNOWN`
- `RUNTIME VERIFIED`

任何 `INFERRED` / `UNKNOWN` 都不能被写成锁定架构事实。

### 3.4 入口 / 调查起点（Entry / Starting Point）

必须给 Codex 一个真实起点，例如：

```text
CLI: burncloud node
Route: POST /v1/chat/completions
Source: src/main.rs :: main
Source: crates/server/src/lib.rs :: start_server
```

这里的目的不是提前写死完整调用链，而是防止 Agent 从全仓库漫游并自行发明入口。

### 3.5 复用目标（Reuse Targets）

明确本 Issue 应优先复用哪些现有能力，例如：

```text
Reuse:
- existing burncloud-server startup
- existing ModelRouter
- existing DownloadManager

Do not recreate:
- second HTTP server
- second router
- second downloader
```

如果调查后发现现有组件不能承担职责，必须先报告原因；不得直接创建第二套实现。

### 3.6 期望行为（Expected Behavior）

描述完成后的外部行为或稳定内部合同。

### 3.7 行为合同（Behavior Contract）

对跨组件能力必须明确：

```text
Inputs
Outputs
Ownership
Side effects
```

这里定义**语义**，不规定具体 Rust struct、函数名或文件布局，除非已有接受的合同已经锁定这些名称。

例如 Model Resolver：

```text
Inputs:
- canonical model identity
- HardwareProfile
- ModelManifest
- RuntimeCapabilities

Output semantics:
- selected variant
- runtime requirement
- artifact reference
- resource requirements
- selection reason

Side effects:
- none
```

### 3.8 失败行为（Failure Behavior）

必须明确失败时系统做什么，以及禁止做什么。

例如：

```text
No compatible variant:
- return explicit structured failure

Forbidden fallback:
- do not choose an arbitrary artifact
- do not silently route to Provider
- do not trigger download unless this Issue explicitly owns preparation
```

不得把“让流程继续跑”作为默认正确行为。

### 3.9 范围（Scope）

必须同时写：

```text
Allowed
Avoid
```

`Allowed` 是预期权限边界，不代表可以任意修改列出的所有代码。

`Avoid` 是默认不可跨越的领域。如果源码证据证明必须跨越，不得直接扩大 Diff；先触发 Stop Condition，再更新 Issue / Task Contract 或拆分新的 Issue。

### 3.10 影响面（Impact）

至少判断：

- persistence
- external calls
- billing / usage / quota
- auth / authorization
- routing / provider
- concurrency / transactions
- public API / CLI
- process / runtime lifecycle

没有影响时明确写 `none`，不能省略。

### 3.11 Invariants / Architecture

列出相关 `INV-*`。

如果 Issue 需要改变 invariant 或架构边界，必须写：

```text
ARCHITECTURE / INVARIANT CHANGE REQUIRED
```

这种变化不能被普通 Feature / Bug Issue 自行批准。

### 3.12 依赖与阻塞（Dependencies / Blockers）

列出前置 Issue、架构决策、外部环境或测试资产。

进入 `READY` 前，硬依赖必须已经完成或被明确豁免；不能让 Codex 一边实现当前 Issue，一边顺手实现前置 Issue。

### 3.13 停止条件（Stop Conditions）

这是 Codex 的硬边界。至少考虑：

```text
STOP IF:
- current source disproves a material Issue assumption
- implementation requires changing an Avoid domain
- implementation requires architecture / invariant change not declared by the Issue
- implementation requires a new duplicate source of truth or duplicate subsystem
- a required dependency is not actually available
- required verification cannot be meaningfully performed
```

触发 Stop Condition 时：

```text
Do not widen scope.
Do not repair unrelated modules.
Do not rewrite the requirement to fit the patch.
Report the conflict and the evidence.
```

### 3.14 验证目标（Verification Targets）

至少定义：

```text
Targeted
Regression
Runtime / E2E (when applicable)
Protected behavior
```

不能只写“tests pass”。

规划阶段如果无法安全确定具体命令，可以先锁定验证目标；Task Contract 再根据当前仓库确定真实命令和测试路径。

### 3.15 完成条件（Done When）

必须使用独立可观察、可验证条件，例如：

```text
- HardwareProfile contains GPU model and VRAM on a supported NVIDIA host.
- Existing CPU / RAM / Disk reporting remains unchanged.
- Resolver consumes HardwareProfile rather than probing GPU independently.
```

`Done When` 不是实现步骤清单，而是验收合同。

## 4. Issue 大小标准

一个 Issue 应尽量满足：

```text
one primary capability
one primary owner/domain
one reviewable behavior change
one independently verifiable completion point
```

出现以下情况应拆分：

- 同时新增 API + 数据库迁移 + Runtime；
- 同时实现两个相互独立的组件；
- 一个 Issue 存在多个互不依赖的主要结果；
- 完成一半后已经形成独立可验证价值；
- 标题必须使用“以及 / and”才能描述两个主要能力；
- Agent 必须获得第二个领域的架构权限才能把第一个能力做完。

不要为了追求小 Issue 而按文件或函数机械拆分。拆分单位是**行为和责任边界**。

## 5. Issue 状态与 READY Gate

状态语义：

```text
PLANNED      已进入实施计划，但尚未获得实现授权
READY        已通过 Evidence Audit 和 READY Gate，可交给 Codex
IN PROGRESS  已有执行分支 / Candidate Patch / PR
BLOCKED      被依赖、证据缺口或架构决策阻塞
DONE         对应 PR 已合并且要求的验证完成
SUPERSEDED   被新的 Issue / 决策替代
```

### READY Gate

Issue 只有同时满足以下条件才能标记 `READY`：

```text
[ ] Single Outcome 明确
[ ] 硬依赖已 DONE 或明确豁免
[ ] Current Evidence 已按当前 main 核实
[ ] Entry / Starting Point 已确定
[ ] Reuse Targets 已确定
[ ] Allowed / Avoid 已确定
[ ] Behavior Contract 已确定（适用时）
[ ] Failure Behavior 已确定
[ ] Impact 已完整判断
[ ] Relevant Invariants 已确定
[ ] Verification Targets 已确定
[ ] Done When 可独立验证
[ ] Stop Conditions 已确定
[ ] 未隐藏架构 / invariant 修改
```

任何一项不满足：

```text
PLANNED / BLOCKED
```

而不是交给 Codex 猜。

## 6. Codex 执行授权

Codex / Coding Agent 只能领取 `READY` Issue。

拿到 Issue 后第一步不是写代码，而是生成或更新 Task Contract，并核实：

1. Issue 的 Current Evidence 在当前 `main` 是否仍成立；
2. Entry 是否对应真实入口；
3. Reuse Targets 是否仍存在且职责匹配；
4. 预计执行路径是否可以被源码证明；
5. Scope 是否足以完成目标；
6. Stop Conditions 是否已经触发；
7. Verification Targets 对应的真实测试 / 命令在哪里。

如果 Issue 与当前源码冲突，Codex 没有权限自行重写架构目标。

正确结果是：

```text
SCOPE / ARCHITECTURE CONFLICT DETECTED
No out-of-scope code changed.
Evidence: ...
Decision required: ...
```

## 7. Pull Request 规则

所有实现 Issue 必须通过 Pull Request 进入 `main`。

```text
READY Issue
  ↓
feature/fix branch
  ↓
Task Contract
  ↓
Candidate Patch
  ↓
Pull Request
  ↓
Review + CI + Verification
  ↓
main
```

禁止把实现代码直接提交到 `main` 后再补 Issue 或补 PR。

PR 正文必须关联 Issue，并说明：

- 实际改变了什么行为；
- 实际修改范围是否超出预期；
- Reuse Targets 是否被复用；
- 是否改变 invariant / architecture / API；
- Failure Behavior 是否与合同一致；
- 执行了哪些验证；
- 哪些验证无法执行；
- 是否触发过 Stop Condition，以及如何处理。

## 8. BurnCloud Node Issue 额外规则

BurnCloud Node 的所有实施 Issue 还必须遵守：

1. 优先复用现有 BurnCloud Server、Router、Database、Model Service、Download、Monitor 能力。
2. 不得创建第二套 Gateway、Router、Downloader、Database 或模型系统，除非先通过独立 Architecture Issue 证明必要性。
3. Local Model 必须通过现有 Router 的 Channel / Ability 体系进入数据面，不创建旁路路由。
4. Resolver 负责选择，不负责下载和启动进程。
5. Runtime 负责定义“如何运行”；Process Manager 负责实际进程生命周期。
6. `Process Spawned != Model READY`；只有 readiness / health 成功后才能接入真实流量。
7. 每个 Node Issue 必须链接 BurnCloud Node 实施计划中的对应子页面。
8. BurnCloud Node 实施计划页面默认是 `PLANNED`；不得直接作为 Codex 实现授权。

## 9. 不要把 Issue 写成实现脚本

Issue 应锁定：

```text
WHAT
WHY
ENTRY
REUSE
BOUNDARY
CONTRACT
FAILURE
STOP CONDITIONS
VERIFICATION
DONE
```

但通常不应提前锁定：

```text
具体文件改第几行
必须创建哪个新 struct
必须新增哪个函数名
具体内部实现算法
```

除非这些内容已经是接受的公共合同或架构决定。

目标是限制 AI 的**架构权限**，不是取消它根据当前源码选择最小实现方式的能力。

## 10. Issue Form

仓库 `.github/ISSUE_TEMPLATE/engineering_task.yml` 是本标准的交互式入口。

Issue Form 负责收集最小结构；本文件定义语义。Issue Form 不能替代源码调查、Evidence Audit 或 Task Contract。
