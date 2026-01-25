[
    {
        "category": "ui",
        "description": "实现模型部署(Deploy)页面的E2E测试",
        "steps": [
            "创建 deploy.spec.ts",
            "访问 /deploy 路径，验证页面标题或面包屑包含'Deploy'",
            "验证表单初始状态：在未填写Model ID或未选择Source时，'Deploy'按钮必须为Disabled状态",
            "输入 Model ID (如 'gpt2') 并选择 Source (如 'HuggingFace')，验证'Deploy'按钮变为Enabled",
            "点击部署按钮，验证页面跳转至 /models",
            "验证出现'Deployment Successful'或类似成功的Toast提示消息",
            "只修改e2e单元测试，不能修改其它代码"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现模型列表(Models)及管理的E2E测试",
        "steps": [
            "创建 models.spec.ts",
            "Mock模型数据列表，访问 /models 页面",
            "验证表格包含必要列：'Status', 'Name', 'Replicas', 'Actions'",
            "验证特定模型(如 'gpt2') 状态显示为 'Running' (绿色指示灯)",
            "点击 'Stop' 按钮，验证状态变更为 'Stopped' 或出现加载状态",
            "点击 'Delete' 按钮，必须弹出确认Modal对话框",
            "在Modal中点击确认，验证该模型行从表格中消失",
            "只修改e2e单元测试，不能修改其它代码"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现Playground页面的E2E测试",
        "steps": [
            "创建 playground.spec.ts",
            "访问 /playground，验证模型选择下拉框默认选中或可选择",
            "在输入框输入测试文本 'Hello World'，验证发送按钮可用",
            "点击发送，验证输入框被清空，且对话区域出现用户消息气泡",
            "等待并验证AI回复气泡出现，且内容不为空",
            "验证对话历史中包含刚才的交互记录",
            "只修改e2e单元测试，不能修改其它代码"
        ],
        "passes": true
    },
    {
        "category": "ui",
        "description": "实现日志(Logs)页面的E2E测试",
        "steps": [
            "创建 logs.spec.ts",
            "访问 /logs 页面，验证日志容器(Container)内已有日志条目加载",
            "在搜索框输入特定关键词(如 'ERROR')，记录当前行数",
            "执行搜索过滤，验证显示行数减少，且剩余每一行都包含 'ERROR'",
            "清除搜索条件，验证日志列表恢复显示所有条目",
            "只修改e2e单元测试，不能修改其它代码"
        ],
        "passes": false
    },
    {
        "category": "ui",
        "description": "实现账单(Billing)页面的E2E测试",
        "steps": [
            "创建 billing.spec.ts",
            "访问 /settings/billing (或对应路径)",
            "验证余额(Balance)组件可见，且包含货币符号(如 '$' 或 '￥')",
            "点击充值(Recharge)按钮，验证充值弹窗或表单展开",
            "输入金额并提交，验证余额数值发生变化或显示'充值成功'提示",
            "验证交易记录表格(Transaction History)至少包含一行数据",
            "只修改e2e单元测试，不能修改其它代码"
        ],
        "passes": false
    },
    {
        "category": "ui",
        "description": "实现开发者设置(API Keys)的E2E测试",
        "steps": [
            "创建 api_keys.spec.ts",
            "访问开发者设置页面，验证 'Generate New Key' 按钮存在",
            "点击生成，输入Key名称，提交",
            "验证列表中新增一行，且Key Secret部分默认被掩码(如 'sk-***')或仅显示前缀",
            "点击该Key的删除/撤销按钮，必须弹出二次确认警告",
            "确认删除后，验证该Key从列表中移除",
            "只修改e2e单元测试，不能修改其它代码"
        ],
        "passes": false
    }
]