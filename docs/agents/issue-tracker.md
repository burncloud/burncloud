# Issue tracker: GitHub

本项目的 Issue 与需求规格以 `burncloud/burncloud` 的 GitHub Issues 为准。
使用 `gh` CLI 操作；Issue 与标签命令显式指定 `--repo burncloud/burncloud`。

## 操作约定

以下 Issue 命令均需附加上述 `--repo` 参数。

- 首次写入前：执行 `gh repo view burncloud/burncloud --json nameWithOwner`，核对目标仓库。
- 创建：`gh issue create --title "..." --body-file "<正文文件>"`。
  多行正文通过 UTF-8 文件传入，避免 shell 转义破坏内容。
- 读取：`gh issue view <编号> --json number,title,body,labels,comments`。
- 列表：`gh issue list --state open --json number,title,body,labels`；
  按需要添加标签和状态筛选。
- 评论：`gh issue comment <编号> --body-file "<正文文件>"`。
- 更新正文：`gh issue edit <编号> --body-file "<正文文件>"`。
- 添加或移除标签：`gh issue edit <编号> --add-label "<标签>"` 或
  `gh issue edit <编号> --remove-label "<标签>"`。
  分流标签使用 `triage-labels.md` 中的映射。
- 关闭：先发表评论说明处理结果，再执行 `gh issue close <编号>`，
  保证说明不会因 Issue 已关闭而丢失。
- 权限不足时停止对应写操作并报告，不擅自更换目标仓库或标签。

## 父子关系与阻塞关系

- 关联子 Issue：`gh issue edit <父编号> --add-sub-issue <子编号>`。
- 添加阻塞项：`gh issue edit <被阻塞编号> --add-blocked-by <阻塞项编号>`。
- 优先使用 GitHub 原生关系。功能不受支持时，在子 Issue 正文顶部记录
  `Part of #<父编号>` 或 `Blocked by: #<编号>, #<编号>`；
  父 Issue 同时维护子项任务列表。
- 所有阻塞项关闭后，任务才视为解除阻塞。权限错误不视为功能不受支持。

## Pull requests as a triage surface

**PRs as a request surface: no.**

## 技能指令映射

- “发布到 Issue 跟踪器”：创建 GitHub Issue。
- “获取相关任务”：读取对应 Issue 的正文、标签和评论。

## Wayfinding

仅在使用 `/wayfinder` 时应用以下约定：

- 地图是一个带 `wayfinder:map` 标签的 Issue，保存探索说明、已确认决策和待解问题。
- 子任务关联为地图的子 Issue，使用 `wayfinder:<type>` 标签；
  type 为 `research`、`prototype`、`grilling` 或 `task`。
  辅助标签按需创建，地图与探索子任务不混用分流标签。
- 可领取任务必须未关闭、无未关闭的阻塞项且无人认领，按地图中的顺序选择。
- 领取时先执行 `gh issue edit <编号> --add-assignee "@me"`。
- 解决后发表评论记录答案，关闭子任务，再把结论摘要与链接追加到地图的决策区。
