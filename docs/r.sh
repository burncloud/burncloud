#!/bin/bash

gemini -y "/ralph-loop @docs/task.md @progress.txt \
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
