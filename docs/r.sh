#!/bin/bash
if [ -z "$1" ]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

# 【重要技巧】：使用 export 导出变量，防止后面被 script 命令的引号解析冲突
export PROMPT="/ralph-loop @docs/task.md @progress.txt \
1. 读取 docs/task.md（JSON 格式）。【重要限制】请彻底忽略所有 'passes': true 的历史任务，不要对它们进行任何处理。在剩余 'passes': false 的任务中，选择当前优先级最高的一个。本次循环【仅限】处理这唯一的一个任务。 \
2. 根据 JSON 中的 'description' 和 'steps' 编写代码。本项目以 Rust 为主，E2E 测试使用 TypeScript。请编写相应的 Rust 后端代码或 .spec.ts 测试文件。 \
3. 运行代码检查和测试以确保质量（Rust: 'cargo clippy' 和 'cargo test'；前端/E2E: 'npm run typecheck' 和 'npm run test'）。 \
4. 【关键步骤】使用文件编辑工具修改 docs/task.md 的 JSON 内容，将你刚刚完成的这个任务的 'passes' 字段从 false 修改为 true。 \
5. 将本次工作进度和留给下一个人的笔记追加写入到 progress.txt 文件中。 \
6. 为这个单独的功能创建一个 git commit。 \
注意：每次只允许做一个功能！ \
当你检查 docs/task.md 发现所有任务的 'passes' 字段都已经是 true（即没有 false 剩余）时，请输出 <promise>COMPLETE</promise>。 \
--max-iterations 20 \
--completion-promise <promise>COMPLETE</promise>"

TEMP_LOG="iter.log"
START_TIME=$SECONDS # 记录总开始时间

for ((i=1; i<=$1; i++)); do
  ITER_START=$SECONDS # 记录单次迭代开始时间
  echo "========================================"
  echo "🚀 [Start] Iteration $i"
  echo "========================================"

  # 【核心改动】：使用 script 命令替代 tee
  # -q : 静默模式，不打印 script 启动信息
  # -e : 继承 gemini 命令的报错退出码
  # -c : 在虚拟终端中执行命令，并将原汁原味的带颜色输出保存到 TEMP_LOG
  script -q -e -c 'gemini -y "$PROMPT"' "$TEMP_LOG"

  # 计算单次耗时
  ITER_DUR=$((SECONDS - ITER_START))
  printf "⏱️  [Time] Iteration %d took %d min %d sec\n\n" $i $((ITER_DUR/60)) $((ITER_DUR%60))

  # 精简逻辑：直接用 grep 检索临时文件
  if grep -q "<promise>COMPLETE</promise>" "$TEMP_LOG"; then
    TOTAL_DUR=$((SECONDS - START_TIME))
    printf "✅ [SUCCESS] PRD complete in %d iterations. Total time: %d min %d sec\n" $i $((TOTAL_DUR/60)) $((TOTAL_DUR%60))
    if command -v tt >/dev/null; then
        tt notify "CVM PRD complete after $i iterations"
    fi
    rm -f "$TEMP_LOG"
    exit 0
  fi
done

TOTAL_DUR=$((SECONDS - START_TIME))
printf "🏁 [END] Reached max iterations. Total time: %d min %d sec\n" $((TOTAL_DUR/60)) $((TOTAL_DUR%60))
rm -f "$TEMP_LOG"
