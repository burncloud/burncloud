# Supply Channel

供给及能力的持久化由 ChannelService 统一拥有，其他领域不直接写入这些表。

## 归属条件生命周期操作

- `ChannelService::quarantine_if_tag(db, id, expected_tag)`：保留供给身份，删除能力并设置已有隔离状态 3。
- `ChannelService::delete_if_tag(db, id, expected_tag)`：同一事务中删除能力及供给。

两个入口均要求正数 ID 和非空的精确标签。标签不符返回 InvalidData，不得误删非本机供给；不存在返回 false；只有提交完成返回 true。查询、写入、回滚、提交错误均不得当成不存在。

PostgreSQL 在同一事务中用 FOR UPDATE 锁定归属；SQLite 使用同一事务的读写，并发升级冲突直接返回错误，不自动重试。提交失败可能无法判定结果，调用方应继续禁派并显式清理，不凭错误文本恢复。

既有非条件 delete/quarantine 接口与行为保持不变。

## 全档能力查询

`ChannelService::list_enabled_for_model` 返回指定组与模型的全部已启用能力，按管理员优先级降序返回，并保留每条能力的 `priority` 与 `weight`。它供新调度策略构造全档候选；旧的 `list_enabled_at_highest_priority` 保持最高档查询边界不变。组名和模型别名必须非空，大小写与模型别名按 Supply 原样匹配；查询错误原样传播，空结果是正常结果。SQLite 与 PostgreSQL 使用各自占位符和布尔字面量。
