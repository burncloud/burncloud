#!/bin/bash
set -e

TASK_FILE="./docs/task.md"

# 加载 .env 文件（如果存在）
if [[ -f "./.env" ]]; then
    set -a
    source "./.env"
    set +a
fi

# ============ Telegram 通知配置 ============
# 从 .env 或环境变量读取
TELEGRAM_BOT_TOKEN="${TELEGRAM_BOT_TOKEN:-}"
TELEGRAM_CHAT_ID="${TELEGRAM_CHAT_ID:-}"

# 发送 Telegram 消息
send_telegram() {
    local message="$1"

    # 如果没有配置，跳过
    if [[ -z "$TELEGRAM_BOT_TOKEN" ]] || [[ -z "$TELEGRAM_CHAT_ID" ]]; then
        echo "⚠️  Telegram not configured (set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID)"
        return 0
    fi

    local escaped_message
    escaped_message=$(echo "$message" | jq -Rs .)

    curl -s -X POST \
        "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
        -H "Content-Type: application/json" \
        -d "{\"chat_id\": \"${TELEGRAM_CHAT_ID}\", \"text\": ${escaped_message}, \"parse_mode\": \"HTML\"}" \
        > /dev/null 2>&1 || echo "⚠️  Failed to send Telegram notification"
}

# 检查 jq 是否安装
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Please install it."
    exit 1
fi

# 格式化时间显示函数
format_duration() {
    local seconds=$1
    local minutes=$((seconds / 60))
    local secs=$((seconds % 60))
    local hours=$((minutes / 60))
    local mins=$((minutes % 60))

    if [ $hours -gt 0 ]; then
        echo "${hours}h ${mins}m ${secs}s"
    elif [ $mins -gt 0 ]; then
        echo "${mins}m ${secs}s"
    else
        echo "${secs}s"
    fi
}

# 时间统计变量
SCRIPT_START_TIME=$(date +%s)
declare -a TASK_TIMES=()
declare -a TASK_NAMES=()
COMPLETED_COUNT=0

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

    # 4. 启动 Claude (非交互模式)
    TASK_START_TIME=$(date +%s)
    OUTPUT=$(claude --dangerously-skip-permissions --print "$PROMPT" 2>&1)
    TASK_EXIT_CODE=$?
    TASK_END_TIME=$(date +%s)

    echo "$OUTPUT"

    # 5. 检查 Claude 是否声称完成了任务
    if [[ $TASK_EXIT_CODE -eq 0 ]] && [[ "$OUTPUT" == *"<promise>TASK_DONE</promise>"* ]]; then
        echo "✅ Task reported done by Claude."

        # 计算任务耗时
        TASK_DURATION=$((TASK_END_TIME - TASK_START_TIME))
        TASK_TIMES+=($TASK_DURATION)
        TASK_NAMES+=("[$CATEGORY] $DESCRIPTION")
        COMPLETED_COUNT=$((COMPLETED_COUNT + 1))

        # 显示当前任务耗时
        echo "⏱️  Task completed in $(format_duration $TASK_DURATION)"

        # 8. 发送 Telegram 通知
        send_telegram "✅ <b>Task Completed</b>

<b>Category:</b> $CATEGORY
<b>Description:</b> $DESCRIPTION
<b>Duration:</b> $(format_duration $TASK_DURATION)
<b>Completed:</b> $COMPLETED_COUNT tasks so far"

        # 6. 更新 task.md 文件 (将 passes 改为 true)
        # 使用临时文件以防 jq 写入错误
        tmp=$(mktemp)
        jq ".[$REAL_INDEX].passes = true" "$TASK_FILE" > "$tmp" && mv "$tmp" "$TASK_FILE"

        # 7. Git 提交 (存档)
        # 检查是否有文件需要提交，避免空提交导致 set -e 退出
        git add .
        if git diff --cached --quiet 2>/dev/null; then
            echo "📝 No code changes (task may have been already done)."
        else
            git commit -m "feat($CATEGORY): $DESCRIPTION"
            echo "💾 Progress saved to Git."
        fi

    else
        echo "❌ Task failed! (exit code: $TASK_EXIT_CODE)"
        echo "Options: [Enter]=retry, [s]=skip, [q]=quit"
        read -r CHOICE
        case "$CHOICE" in
            q|quit) exit 1 ;;
            s|skip) echo "⏭️  Skipping..." ;;
            *) echo "🔄 Retrying..."; continue ;;
        esac
    fi

    # 休息一下，防止 API 速率限制
    sleep 2
done

# 显示统计报告
SCRIPT_END_TIME=$(date +%s)
TOTAL_DURATION=$((SCRIPT_END_TIME - SCRIPT_START_TIME))

echo ""
echo "=================================================="
echo "📊 TASK COMPLETION REPORT"
echo "=================================================="

# 发送最终报告到 Telegram
send_telegram "🎉 <b>All Tasks Completed!</b>

<b>Total tasks:</b> $COMPLETED_COUNT
<b>Total time:</b> $(format_duration $TOTAL_DURATION)
$(if [ $COMPLETED_COUNT -gt 0 ]; then echo "<b>Average per task:</b> $(format_duration $((TOTAL_DURATION / COMPLETED_COUNT)))"; fi)"
echo ""
echo "📋 Total tasks completed: $COMPLETED_COUNT"
echo ""
echo "⏱️  Individual task times:"
for i in "${!TASK_NAMES[@]}"; do
    printf "   %d. %s\n      └─ %s\n" $((i+1)) "${TASK_NAMES[$i]}" "$(format_duration ${TASK_TIMES[$i]})"
done
echo ""
echo "🕐 Total time: $(format_duration $TOTAL_DURATION)"
if [ $COMPLETED_COUNT -gt 0 ]; then
    echo "📈 Average time per task: $(format_duration $((TOTAL_DURATION / COMPLETED_COUNT)))"
fi
echo ""
echo "=================================================="
