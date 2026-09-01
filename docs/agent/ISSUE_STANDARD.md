---
doc_id: agent.issue-standard
doc_type: agent-protocol
truth: normative
status: active
---

# Engineering Issue Enforcement

BurnCloud 的 Issue 语义标准不在本仓库重复定义。

Canonical Standard：

https://burncloud.github.io/burncloud-node/implementation-plan/issue-standard/

本文件只负责把 Canonical Standard 接入 `burncloud/burncloud` 的 Agent / Codex 执行流程，避免出现第二套 Source of Truth。

## Repository enforcement

本仓库通过以下机制执行 Canonical Standard：

- `.github/ISSUE_TEMPLATE/engineering_task.yml` — 收集 READY Engineering Issue 所需字段；
- `docs/agent/TASK_CONTRACT.md` — Codex 修改代码前的 current-main preflight；
- `AGENTS.md` / `START_HERE.md` / `TASK_ROUTER.md` — 将 Agent 路由到真实源码、测试和 Invariants；
- CI / Review / Definition of Done — 验证 Candidate Patch 是否满足合同；
- Pull Request — 所有实现进入 `main` 的唯一正常路径。

## Execution rule

```text
Implementation Plan (PLANNED)
        ↓
Evidence Audit
        ↓
READY Engineering Issue
        ↓
Task Contract against current main
        ↓
Codex / Coding Agent
        ↓
Candidate Patch
        ↓
Pull Request
        ↓
Verification / Review
        ↓
main
```

Codex 不得直接实现 `PLANNED` 页面。

GitHub Issue 必须满足 Canonical Standard 的 READY Gate；随后 Codex 仍必须通过 Task Contract 核实当前 `main`。

## Conflict rule

如果本仓库的 Issue Form、Task Contract、Agent Rule 或 CI 与 Canonical Standard 发生语义冲突：

1. 不得自行选择更宽松的解释；
2. 不得为了完成任务扩大权限；
3. 停止实现并报告冲突；
4. 通过独立 Pull Request 修正执行层或 Canonical Standard。

本仓库的执行文件可以强化约束，但不能另行定义一套冲突的 Issue 语义。
