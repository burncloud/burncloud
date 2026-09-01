---
doc_id: agent.issue-standard
doc_type: agent-protocol
truth: normative
status: active
---

# BurnCloud Issue 标准

Issue 是进入 BurnCloud 工程执行系统的第一份边界合同。它的职责不是描述“想做什么”这么简单，而是把一个需求压缩成 **单一、可验证、可审查、可转换成 Task Contract 的工程任务**。

Issue 本身不是事实来源。当前行为必须由源码、测试、运行证据或现有规范证明；如果证据不足，必须标记为 `UNKNOWN`，不能把猜测写成当前事实。

## 1. 核心原则

每个工程 Issue 必须满足：

1. **Single Outcome** — 一个 Issue 只交付一个主要可观察结果。
2. **Evidence First** — 当前行为必须引用证据，不能依赖聊天记忆。
3. **Bounded Scope** — 明确允许修改的领域，以及默认禁止跨越的边界。
4. **Invariant Aware** — 列出会受到影响的不变量；如果不知道，先调查。
5. **Verifiable Done** — 完成条件必须可以通过测试、运行路径或明确检查验证。
6. **No Hidden Architecture Change** — Issue 不得把架构变更伪装成普通实现任务。
7. **No Bundle Issues** — 不把多个独立能力、多个 Phase 或“顺手重构”打包进同一个 Issue。

## 2. Issue 与 Task Contract 的关系

流程固定为：

```text
Implementation Plan
      ↓
Engineering Issue
      ↓
Task Contract
      ↓
Source / Tests / Invariants
      ↓
Implementation
      ↓
Pull Request
      ↓
Verification
```

Issue 定义 **任务边界和验收目标**；`TASK_CONTRACT.md` 在真正修改代码前，把 Issue 再转换成当前代码证据下的执行合同。

如果代码调查发现 Issue 的假设不成立：

- 不得为了“完成 Issue”强行修改系统；
- 更新 Task Contract；
- 必要时回到 Issue 重新修订目标或拆分任务。

## 3. 必填结构

每个非平凡工程 Issue 至少包含以下部分。

### 3.1 标题

格式：

```text
[domain] observable outcome
```

BurnCloud Node 示例：

```text
[node] establish canonical HardwareProfile
[node] detect NVIDIA GPU and VRAM
[node] register READY local runtime as existing Channel
```

标题描述结果，不描述文件名，也不要使用 `misc`、`cleanup`、`improve` 这类不可验证词语。

### 3.2 目标（Goal）

说明用户、运维人员或系统最终能够观察到什么。

### 3.3 当前事实（Current Evidence）

必须引用当前源码、测试、运行路径或规范。

证据使用统一分类：

- `STATIC CONFIRMED`
- `DYNAMIC`
- `INFERRED`
- `UNKNOWN`
- `RUNTIME VERIFIED`

### 3.4 期望行为（Expected Behavior）

描述完成后的外部行为或稳定内部合同。

### 3.5 范围（Scope）

必须同时写：

```text
Allowed
Avoid
```

`Allowed` 表示预期工作边界，不等于可以任意修改其中所有代码。

`Avoid` 表示默认不可跨越的边界。如果调查证明根因必须跨越，应先更新 Task Contract，而不是静默扩张 Diff。

### 3.6 影响面（Impact）

至少判断：

- persistence
- external calls
- billing / usage / quota
- auth / authorization
- routing / provider
- concurrency / transactions
- public API / CLI
- process / runtime lifecycle

没有影响时明确写 `none`。

### 3.7 Invariants

列出相关 `INV-*`；如果 Issue 提议改变 invariant，必须明确标记：

```text
ARCHITECTURE / INVARIANT CHANGE REQUIRED
```

这种 Issue 不得作为普通功能 Issue 静默实现。

### 3.8 验证（Verification）

至少定义：

- targeted test
- regression check
- runtime / E2E check（适用时）

不能只写“测试通过”。

### 3.9 完成条件（Done When）

使用可观察条件，例如：

```text
- HardwareProfile contains GPU model and VRAM on a supported NVIDIA host.
- Existing CPU / RAM / Disk reporting remains unchanged.
- Resolver consumes HardwareProfile rather than probing GPU independently.
```

## 4. Issue 大小标准

一个 Issue 应当尽量满足：

```text
one primary capability
one primary owner/domain
one reviewable behavior change
one independently verifiable completion point
```

出现以下情况应拆分：

- 同时新增 API + 数据库迁移 + Runtime；
- 同时实现两个相互独立的组件；
- 一个 Issue 需要多个互不依赖的“Done When”；
- 完成一半后系统已经形成独立可验证价值；
- Issue 名称需要使用“以及 / and”连接两个主要能力。

不要为了追求小 Issue 而按文件或函数机械拆分。拆分单位是 **行为与责任边界**。

## 5. Issue 状态语义

建议使用以下状态理解，而不是把 Issue 当事实：

```text
PLANNED      已进入实施计划，但尚未开始
READY        目标、边界、依赖、验收条件足够明确
IN PROGRESS  已有执行分支 / PR
BLOCKED      被外部依赖或架构决策阻塞
DONE         对应 PR 已合并且完成验证
SUPERSEDED   被新的 Issue / 决策替代
```

`PLANNED` 不代表代码已经存在；`DONE` 才能转化为“已实现”的候选事实，并且仍应由代码/测试/运行证据确认。

## 6. Pull Request 规则

所有实现 Issue 必须通过 Pull Request 进入 `main`。

```text
Issue
  ↓
feature/fix branch
  ↓
Pull Request
  ↓
Review + CI + Verification
  ↓
main
```

禁止把实现代码直接提交到 `main` 后再补 Issue 或补 PR。

PR 应在正文中关联 Issue，并说明：

- 实际改变了什么行为；
- 是否扩大了 Issue Scope；
- 是否改变 invariant / architecture / API；
- 执行了哪些验证；
- 哪些验证无法执行。

## 7. BurnCloud Node Issue 额外规则

BurnCloud Node 的所有实施 Issue 还必须遵守：

1. 优先复用现有 BurnCloud Server、Router、Database、Model Service、Download、Monitor 能力。
2. 不得创建第二套 Gateway、Router、Downloader、Database 或模型系统，除非先通过独立架构 Issue 证明必要性。
3. Local Model 必须通过现有 Router 的 Channel / Ability 体系进入数据面，不创建旁路路由。
4. Resolver 负责选择，不负责下载和启动进程。
5. Runtime 负责构造运行方式，Process Manager 负责实际进程生命周期。
6. `Process Spawned != Model READY`；只有 readiness / health 成功后才能接入真实流量。
7. 每个 Node Issue 必须链接 BurnCloud Node 实施计划中的对应子页面。

## 8. Issue Form

仓库 `.github/ISSUE_TEMPLATE/engineering_task.yml` 是本标准的交互式入口。

Issue Form 负责强制收集最小信息；本文件负责定义语义。Issue Form 不能替代源码调查和 Task Contract。
