set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

TEMP_OUTPUT="current_iteration.log"
TEMP_RUNNER="run_claude.sh"

for ((i=1; i<=$1; i++)); do
  echo "Iteration $i"
  echo "--------------------------------"
  
  # 第 1 步：将极其复杂的 claude 命令写入一个临时脚本文件
  # 使用 'EOF' (带引号) 防止 bash 提前解析里面的变量或特殊字符
  cat << 'EOF' > "$TEMP_RUNNER"
claude -p "@docs/TASK.md @progress.txt \
1. Find the highest-priority feature to work on and work only on that feature. \
This should be the one YOU decide has the highest priority - not necessarily the first in the list. \
2. Check that the types check via npm run typecheck and that the tests pass via npm run test. \
3. Update the PRD with the work that was done. \
4. Append your progress to the progress.txt file. \
Use this to leave a note for the next person working in the codebase. \
5. Make a git commit of that feature. \
ONLY WORK ON A SINGLE FEATURE. \
If, while implementing the feature, you notice the PRD is complete, output <promise>COMPLETE</promise>. \
"
EOF
  
  # 赋予临时脚本执行权限
  chmod +x "$TEMP_RUNNER"

  # 第 2 步：使用 script 命令在一个完全真实的虚拟终端中运行它
  # -q : 静默模式
  # -e : 继承脚本的退出码
  # -c : 执行命令
  script -q -e -c "./$TEMP_RUNNER" "$TEMP_OUTPUT"

  # 第 3 步：从 script 的日志文件中读取结果
  # 警告：script 会记录一些 ANSI 颜色代码，所以匹配时要用通配符
  result=$(cat "$TEMP_OUTPUT")

  if [[ "$result" == *"<promise>COMPLETE</promise>"* ]]; then
    echo "PRD complete, exiting."
    tt notify "CVM PRD complete after $i iterations"
    # 清理临时文件
    rm -f "$TEMP_OUTPUT" "$TEMP_RUNNER"
    exit 0
  fi
done

# 清理临时文件
rm -f "$TEMP_OUTPUT" "$TEMP_RUNNER"
