{
    "target": "在burncloud-supply-model/中完成当前阶段的 Model Resolver 功能：复用 RAM、VRAM 和 Runtime 过滤规则，新增模型工件磁盘空间过滤，返回保持清单顺序的候选模型结果",
    "completed_work": "阶段二已经完成模型清单、Variant、Resolver 契约及相关基础结构。阶段三只在现有边界内补充当前 Issue 明确要求的解析和兼容性决策能力",
    "scope": [
        "Resolver 只接收上游准备好的 ModelManifest，负责模型解析和过滤；阶段三只复用阶段二 RAM、VRAM、Runtime 规则，新增磁盘过滤并直接返回全部候选，不进行打分、排序或推荐选择",
        "Resolver 不递归扫描 GGUF 文件，不筛选模型文件，不构建 artifact URI；artifact_uri 和 checksum 由上游 Manifest 提供；每个 Variant 的 backend 必须在 supported_backends 中声明",
        "不负责模型文件发现、下载模型、写入缓存、启动 Runtime、管理进程或执行其他外部动作",
        "不新增 ModelService CRUD、数据库访问层、Repository、Factory 或其他提前设计的抽象",
        "不修改 ModelRouter、CLI、Client、Node 编排代码或无关业务逻辑",
        "不修改阶段二已有的 RAM、VRAM 和 Runtime 过滤规则语义，磁盘过滤使用独立步骤实现",
        "如果新增或修改公开 API，必须同步补充中文 /// 文档注释、参数校验、测试和 README",
        "公开 API 默认保持私有；只有当前 Issue 或既有公共契约明确要求时才使用 pub",
        "测试代码必须放在 tests/ 或独立测试文件中，不得与新增业务实现混写"
    ],
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}
