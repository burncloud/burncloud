1. 分页显示模型列表

  基础分页查询

  # 获取前10个模型
  curl "https://huggingface.co/api/models?limit=10"

  # 获取第2页的10个模型
  curl "https://huggingface.co/api/models?limit=10&skip=10"

  # 带过滤条件的分页查询
  curl "https://huggingface.co/api/models?limit=20&filter=text-class
  ification&sort=downloads&direction=desc"

  高级过滤示例

  # 按任务类型过滤
  curl "https://huggingface.co/api/models?pipeline_tag=text-generati
  on&limit=15"

  # 按库类型过滤
  curl
  "https://huggingface.co/api/models?library=transformers&limit=10"

  # 按标签过滤
  curl
  "https://huggingface.co/api/models?tags=pytorch&tags=en&limit=10"

  # 搜索特定模型名称
  curl "https://huggingface.co/api/models?search=bert&limit=20"


2. 显示模型详情页

  获取基本模型信息

  # 获取GPT-2模型详情
  curl "https://huggingface.co/api/models/openai-community/gpt2"


  # 获取GPT-2模型包含完整文件信息
  curl "https://huggingface.co/api/models/openai-community/gpt2?expand[]=siblings"

  # 获取模型配置信息
  curl "https://huggingface.co/api/models/facebook/bart-large?expand[]=config"

3. 显示仓库文件

  https://huggingface.co/api/models/LiquidAI/LFM2-8B-A1B-GGUF/tree/main

4. 特殊例子，GGUF多文件读取

  https://huggingface.co/huihui-ai/Huihui-Qwen3-VL-30B-A3B-Instruct-abliterated/tree/main/GGUF


  filename，size