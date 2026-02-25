#!/bin/bash
set -e

TASK_FILE="./docs/task.md"

# 检查 jq 是否安装
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Please install it."
    exit 1
fi

# 开始循环，直到没有未完成的任务
while true; do
    echo "=================================================="
    echo "🔍 Scanning for the next pending task..."

    # 1. 查找第一个 'passes: false' 的任务索引
    # 如果返回 null，说明所有任务都做完了
    TASK_INDEX=$(jq '[.[] | select(.passes == false)] | if length > 0 then 0 else null end' "$TASK_FILE")
    
    # 这里的逻辑是：我们需要找到在原数组中的真实索引，以便稍后更新
    # 更严谨的做法是直接获取原数组中第一个 false 的索引
    REAL_INDEX=$(jq 'map(.passes == false) | if any then index(true) else null end' "$TASK_FILE")

    if [ "$REAL_INDEX" == "null" ]; then
        echo "🎉 All tasks completed! Exiting."
        break
    fi

    # 2. 提取任务详情
    CATEGORY=$(jq -r ".[$REAL_INDEX].category" "$TASK_FILE")
    DESCRIPTION=$(jq -r ".[$REAL_INDEX].description" "$TASK_FILE")
    STEPS=$(jq -r ".[$REAL_INDEX].steps[]" "$TASK_FILE")
    
    echo "🚀 Found Task [$CATEGORY]: $DESCRIPTION"
    echo "📋 Steps to execute:"
    echo "$STEPS"
    echo "--------------------------------------------------"

    # 3. 构造 Claude 的专属提示词 (Prompt Injection)
    # 我们只把当前这一项任务喂给它，保持上下文极其干净
    PROMPT="
    Role: You are a focused expert developer.
    Context: We are working on a project task list.
    
    YOUR CURRENT ASSIGNMENT:
    Category: $CATEGORY
    Goal: $DESCRIPTION
    
    Execution Steps:
    $STEPS
    
    INSTRUCTIONS:
    1. Only implement the code for THIS specific task.
    2. Do not touch other parts of the system unrelated to this task.
    3. Run tests to verify your work.
    4. When finished, output exactly: <promise>TASK_DONE</promise>
    "

    # 4. 启动 Claude (拉尔夫模式：执行完即退出)
    # 使用 echo "/exit" 这种 hack 确保它如果卡在交互界面能退出 (视你的 docker/cli 行为而定)
    # 或者如果 claude-code 有非交互模式，最好用非交互模式
    
    # 这里假设你是在 Docker 里跑，且需要捕获输出
    OUTPUT=$(echo "/exit" | claude --dangerously-skip-permissions -p "$PROMPT")
    
    echo "$OUTPUT"

    # 5. 检查 Claude 是否声称完成了任务
    if [[ "$OUTPUT" == *"<promise>TASK_DONE</promise>"* ]]; then
        echo "✅ Task reported done by Claude."
        
        # 6. 更新 task.md 文件 (将 passes 改为 true)
        # 使用临时文件以防 jq 写入错误
        tmp=$(mktemp)
        jq ".[$REAL_INDEX].passes = true" "$TASK_FILE" > "$tmp" && mv "$tmp" "$TASK_FILE"
        
        # 7. Git 提交 (存档)
        git add .
        git commit -m "feat($CATEGORY): $DESCRIPTION"
        echo "💾 Progress saved to Git."
        
    else
        echo "❌ Task failed or timed out. Please check logs."
        exit 1
    fi

    # 休息一下，防止 API 速率限制
    sleep 2
done
