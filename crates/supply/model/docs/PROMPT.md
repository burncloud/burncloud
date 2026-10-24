{
    "target": "在burncloud-supply-model/中完成当前阶段的 Model Resolver 功能：复用 RAM、VRAM 和 Runtime 过滤规则，新增模型工件磁盘空间过滤，返回保持清单顺序的候选模型结果",
    "completed_work": "阶段二已经完成模型清单、Variant、Resolver 契约及相关基础结构。阶段三只在现有边界内补充当前 Issue 明确要求的解析和兼容性决策能力",
    "scope": [
        "Resolver 只负责模型解析和过滤；阶段三只复用阶段二 RAM、VRAM、Runtime 规则，新增磁盘过滤并直接返回全部候选，不进行打分、排序或推荐选择",
        "不负责下载模型、写入缓存、启动 Runtime、管理进程或执行其他外部动作",
        "不新增 ModelService CRUD、数据库访问层、Repository、Factory 或其他提前设计的抽象",
        "不修改 ModelRouter、CLI、Client、Node 编排代码或无关业务逻辑",
        "不修改阶段二已有的 RAM、VRAM 和 Runtime 过滤规则语义，磁盘过滤使用独立步骤实现",
        "如果新增或修改公开 API，必须同步补充中文 /// 文档注释、参数校验、测试和 README",
        "公开 API 默认保持私有；只有当前 Issue 或既有公共契约明确要求时才使用 pub",
        "测试代码必须放在 tests/ 或独立测试文件中，不得与新增业务实现混写"
    ],
    "rules": [
        "使用中文回复，注释必须使用中文",
        "公开 API 的 /// 必须说明用途、参数、返回值、/// # Examples 和注意事项",
        "只允许使用 Rust 编写代码",
        "禁止使用 unsafe",
        "禁止对外部输入使用 unwrap 或 expect",
        "禁止静默吞错，必须传播或明确处理错误",
        "不为后续阶段提前增加复杂架构",
        "公有接口和公有函数必须校验空字符串、非法枚举值、越界资源值和其他不合法输入",
        "私有函数不重复增加无意义的参数校验",
        "测试代码与业务源码分离，公开 API 必须有对应测试，且覆盖正常、边界和错误情况",
        "修复问题必须增加回归测试",
        "修改公开 API 后必须同步更新本文件和 README，README 只描述已经实现的能力",
        "完成修改后运行 cargo fmt --all -- --check、cargo clippy -p burncloud-supply-model --all-targets --all-features -- -D warnings 和 cargo test -p burncloud-supply-model"
    ]
}
