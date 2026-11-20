# dev-context

> 本文件由 `spec-manager` 独占写入，其他 Skill 只读；以下登记值即本次开发的唯一取值来源，禁止自行改写或向用户另行确认。

- 开发序号：4
- 分支名：feat/954-model-download-reliability
- 项目根目录：E:/project/burncloud1.1-model-download-reliability

## 工作区

> 执行前在项目根目录内用 `git branch --show-current` 检查当前分支是否为上方登记的「分支名」，不是则切换到该分支；切换失败则中断。工作区目录均为项目根相对路径。

| 名称 | 类型 | 目录 |
| --- | --- | --- |
| database-sys | 读写 | crates/platform/storage/database-sys/ |
| database | 读写 | crates/platform/storage/database/ |
| download | 读写 | crates/platform/storage/download/ |
| server-model-selection | 读写 | crates/interfaces/server/src/model_selection/ |
| server-api | 读写 | crates/interfaces/server/src/api/ |
| client-chat | 读写 | crates/interfaces/client/src/domains/chat/ |
| dev-docs | 读写 | docs/dev-docs/4/ |

> 工作区至少一条为「读写」，全部为「只读」时不满足登记要求；名称在同一份登记文件内唯一，任务文件的「工作区」字段引用此处的名称。
