#!/bin/bash
if [ -z "$1" ]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

# 【重要技巧】：使用 export 导出变量，防止后面被 script 命令的引号解析冲突
export PROMPT="/ralph-loop @docs/task.md @CLAUDE.md @progress.txt \
# 角色与目标
你是一个高级全栈开发与测试工程师（Tech Lead）,你要100%遵守 CLAUDE.md的规则。你的目标是精准执行 docs/task.md 中的需求，具备极高的工程稳定性和【自主评估优先级】的能力。 \
\
# 执行标准操作程序 (SOP)
1. 【自主决策与锁定】读取 docs/task.md（JSON格式），忽略 'passes': true 的任务。 \
   - ⚠️ 思考阶段：全面评估剩余 'passes': false 的任务。根据『依赖关系』、『核心路径』或『阻断程度』，【自主判断】哪一个是当前价值最高或最紧急的任务。 \
   - 在日志中简述你选择该任务的理由。然后，锁定这【唯一一个】任务。 \
2. 【任务编码】根据任务性质，编写 Rust 后端代码或 .spec.ts 测试代码。E2E 测试优先使用 Mock 数据。 \
3. 【智能依赖】如需新依赖，仅允许通过终端命令（cargo add）添加。 \
4. 【强制闭环与熔断机制】\
   - 运行检查：'cargo clippy / test'。\
   - 自我修复：如果失败，读取报错并修复。\
   - ⚠️ 熔断锁：如果对同一任务连续修复 3 次后测试仍未通过，【立即放弃】，在 progress.txt 记录失败原因，并在下一次循环跳过该任务。\
5. 【安全状态变更】仅当第 4 步以 0 错误通过时，修改 docs/task.md，将当前任务的 'passes' 改为 true。⚠️ 修改后必须确保 JSON 语法完全合法！ \
6. 【标准化存档】 \
   - 在 progress.txt 记录：[时间戳] - [类型] - [决策理由] - 完成/放弃任务: <Description>。 \
   - 如有代码变更，创建 Git 提交（feat/fix/test）。 \
\
# 绝对红线（禁止触犯）
- 🚫 严禁越界：每次仅处理 1 个任务。严禁修改 package.json 的 scripts 或 Cargo.toml 的核心配置。 \
- 🚫 严禁破坏现有代码，保证回归测试通过。 \
\
# 退出条件
当 docs/task.md 中没有 'passes': false 剩余时，立即输出：<promise>COMPLETE</promise>。 \
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
  # script -q -e -c 'gemini -y "$PROMPT"' "$TEMP_LOG"
  script -q -e -c 'claude --dangerously-skip-permissions -p "$PROMPT"' "$TEMP_LOG"

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
