# 路径请求为 /openai/deployments/{deployment-id}/chat/completions?api-version={version} 判定为azure
# 路径请求为 /v1beta/models/gemini-pro:generateContent 判定为google aistudio
# 路径请求为 /v1/projects/{PROJECT_ID}/locations/{REGION}/publishers/google/models/{MODEL_ID}:{ACTION} 判定为vertex

# service-models 删除模型
- 删除模型需要清理对应的文件，包含清理.1,.2,.3的后缀
- 例Qwen2.5-7B-Instruct-GGUF.1，Qwen2.5-7B-Instruct-GGUF.2，Qwen2.5-7B-Instruct-GGUF.3。
- 直接先去掉文件Qwen2.5-7B-Instruct-GGUF.guff的后缀，再删除Qwen2.5-7B-Instruct-GGUF*文件
# [complete] service-models 下载功能
# [complete] service-models 列出所有GUFF文件
# [complete] service-models 递归获取所有文件
# [complete] service-models 设置url host
# [complete] client-models 弹出功能修改
# [complete] client-models 添加模型页面
# [complete] service-models 增加 https://huggingface.co/api/models 对接
# [complete] service-ip cache 读取
# [complete] service-setting 初始化
# [complete] database-setting 
# [complete] service-ip 判断用户当前处于哪个网络环境
# [complete] client-models页面
