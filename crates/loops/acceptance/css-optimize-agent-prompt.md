# CSS 优化 — 单轮 Agent 任务

完整验收见 **`crates/loops/acceptance/css-optimize-acceptance.md`**（A–E 静态 + **V 视觉**）。

## 1. 诊断

```powershell
.\scripts\check-css-naming.ps1
.\scripts\check-css-visual.ps1   # 命名已通过后再跑
```

- **命名失败** → 按 §A–E 改 RSX class  
- **视觉失败** → 打开 `data/loops/css-visual/latest/` 看 `FAIL-*` 与页截图，按 §V 修排版

## 2. 静态修复（控制台）

只改扫描范围内 RSX（排除 `client-api`、guest 四页）。每轮 **最多 8 个文件**。

映射见 `css-optimize-acceptance.md` §A–D。

## 3. 视觉修复（命名已通过时）

1. 查看 `data/loops/css-visual/latest/manifest.json` 与 PNG  
2. 对照 acceptance §V3 检查：间距节奏、字号层级、圆角阴影、对齐、主色  
3. 若 JS 报 `legacy-dom-class` — 浏览器里仍有违规 class，继续改 RSX  
4. 若 `horizontal-overflow` — 查 flex/grid、`min-w-0`、`overflow`  
5. 若 `missing-content-shell` — 页面缺 `page-content` 或标准壳层  

## 4. 禁止

- 改 guest / 扩展 client-api  
- 新增 shadcn、裸 hex、静态 `style:` 间距/颜色  
- 改 `.gitignore`；未经要求不 commit  

## 5. 验证

```bash
cd crates/client && cargo check -p burncloud-client-shared
```

```powershell
.\scripts\check-css-naming.ps1
.\scripts\check-css-visual.ps1
```

## 6. 输出

说明：改了哪些文件、命名/视觉哪一项仍失败、截图路径。
