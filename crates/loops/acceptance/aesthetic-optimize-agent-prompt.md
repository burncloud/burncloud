# 审美优化 — 单轮 Agent 任务

完整验收见 **`crates/loops/acceptance/aesthetic-optimize-acceptance.md`**（J0–J7）。  
**前置**: `check-css-all.ps1` 必须通过；否则先走 CSS loop。

## 1. 诊断

```powershell
.\scripts\check-css-all.ps1
.\scripts\check-aesthetic-metrics.ps1
.\scripts\check-aesthetic-review.ps1
```

- **CSS 失败** → 回到 `css-optimize-agent-prompt.md`，不要改审美  
- **J1 度量失败** → 看 `metrics.json` 与 `FAIL-metrics-*` 截图  
- **review 失败** → 读 `*-viewport.png`，更新 `review.json` 并改 UI

## 2. 看图评审（J3）

1. 打开 `data/loops/aesthetic/latest/` 全部 `*-viewport.png`  
2. 每页按 **C / F / D / A / R / P / E** 打 1–5 分（见 acceptance J0）  
3. 任一条 ≤ 2 或 P0 硬伤 → 记入 `p0[]`，本页不通过  
4. 通过线：七维均 ≥ 4，且 `p0` 为空  
5. 并排看控制台页，勾选 `global_j4` 八项一致性  

更新 `review.json`：

- 填真实分数与 `notes`  
- 全部达标后设 `"pass": true` 和 `reviewed_at`  
- 删除占位 `"not-reviewed"`  

## 3. 实现修改（单轮约束 J6）

1. 每轮 **最多 6 个文件**，优先 `review.json` 最低分页面  
2. 优先级：**删** > 对齐/间距 > 字号层级 > 颜色克制 > 新组件  
3. 不新增装饰性插图/渐变/动画（除非用户明确要求）  
4. 不破坏 i18n、四态、五态（`docs/ui/system.md`）  
5. 主色 `#007AFF` 每区域仅一个 Primary CTA  

常见修复：

- 统一 `gap-bc-*`、卡片内边距、PageHeader 间距  
- 减少字号种类，对齐 `large-title` / `title` / `body` / `caption`  
- 去掉多余边框，用留白分层  
- 表格行高、数字右对齐（finance/monitor）  
- 空状态补引导文案与操作  

## 4. 禁止

- 改 guest 营销文案大段重写（content loop）  
- 新增 shadcn、裸 hex、内联颜色 style  
- 改 `.gitignore`；未经要求不 commit  

## 5. 验证

```bash
cd crates/client && cargo check -p burncloud-client-shared
# 触及页面时:
cargo check -p burncloud-client
```

```powershell
.\scripts\check-aesthetic-metrics.ps1
.\scripts\check-aesthetic-review.ps1
```

## 6. 输出

说明：改了哪些文件、哪页哪维仍低分、更新后的 `review.json` 摘要、截图路径。
