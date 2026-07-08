# 审美优化 Loop — 验收标准（Jobs / Apple HIG 取向）

**定位**: 在 `css-optimize-acceptance.md`（命名 + 基础视觉）**全部通过** 之后，专门提升**界面美观、气质与工艺感**。  
**权威参考**: `docs/ui/system.md` · `docs/ui/tokens.md` · `docs/ui/pages.md`  
**前置条件**: `check-css-naming` + `check-css-visual` 均已 exit 0。

> 「简单比复杂更难：你必须努力理清思路，才能让它变得简单。」—— 本 loop 不是加装饰，而是**删到只剩必要、再把必要做到极致**。

---

## Loop 退出条件（须全部满足）

| # | 检查项 | 说明 |
|---|--------|------|
| 0 | 基础 CSS 验收 | `check-css-all.ps1` 通过（命名 + V1–V4） |
| 1 | 审美截图集 | `data/loops/aesthetic/latest/` 含 §J2 全部截图 + `manifest.json` |
| 2 | 自动度量 | `check-aesthetic-metrics.ps1` exit 0（§J1，可脚本部分） |
| 3 | 审美评分 | Agent 按 §J3 填写 `review.json`，**每页 ≥ 4/5**，且**无 P0 硬伤** |
| 4 | 跨页一致性 | §J4 全局核对表全部勾选 |

**退出语**: `Done: all pages passed Jobs aesthetic layout.`

**主入口**: `cargo run -p burncloud-loops -- jobs-aesthetic` — 循环直到 9 页全部通过 J1 + J3 + J4。

---

## J0. 审美原则（评审 lens）

评审每一屏时，用下面 7 条打分（1–5），**任一条 ≤ 2 即 P0，必须修完才能过 loop**。

| 代号 | 原则 | Jobs 式问法 | 不及格信号 |
|------|------|-------------|------------|
| **C** | Clarity 清晰 | 用户 3 秒内知道「我在哪、能干什么」？ | 标题弱、信息平铺、主次不分 |
| **F** | Focus 专注 | 这一屏是否只做一件事？ | 同屏 3 个以上竞争主 CTA、装饰抢戏 |
| **D** | Depth 层次 | 是否用字重/字号/留白建立深度，而非靠边框堆叠？ | 满屏线框、卡片套卡片、灰阶糊成一片 |
| **A** | Alignment 对齐 | 所有元素是否落在 4px 栅格上，视觉边是否对齐？ | 1–2px 级错位、列宽不齐、图标与文字基线不齐 |
| **R** | Restraint 克制 | 能否再删掉 20% 仍成立？ | 多余分割线、重复标题、过强阴影/渐变 |
| **P** | Polish 工艺 | 间距、圆角、hover 是否「像一个人调的」？ | 同级元素间距不一、按钮高矮不齐、空状态敷衍 |
| **E** | Emotion 气质 | 是否像 Apple 企业工具，而非「后台模板」？ | Fluent 灰、挤、吵；或过度拟物/花哨 |

**气质锚点**（BurnCloud 控制台）:

- 大量留白，内容「浮」在干净的面上  
- 主色 `#007AFF` 只用于**一个**主行动点，其余中性  
- 字号层级少而准：`large-title` → `title` → `subtitle` → `body` → `caption`  
- 圆角统一 `rounded-bc-*`，阴影轻、少、有目的  
- 动效短、有物理感，解释状态变化，不炫技  

---

## J1. 可自动度量（`check-aesthetic-metrics`）

在 `check-css-visual` 截图基础上，浏览器内追加检测（或独立脚本读同一批截图会话）。

### J1.1 排版与对比

| 检查 | 阈值 | 失败含义 |
|------|------|----------|
| 正文对比度 | ≥ 4.5:1（WCAG AA） | 次要文字过浅、不可读 |
| 大标题对比度 | ≥ 3:1 | 标题与背景对比不足 |
| 页面字号种类（DOM 计算） | ≤ 5 种有效字号 | 字号层级失控 |
| 行高 | 正文 `line-height` 1.4–1.6 | 挤或散 |

### J1.2 间距节奏

| 检查 | 阈值 | 失败含义 |
|------|------|----------|
| 同级卡片 `gap` | 同一列表内偏差 ≤ 1 个 bc 档位 | 间距「手调感」差 |
| 页面水平边距 | 主内容区左右 `padding` 一致 | 内容歪向一侧 |
| 区块间距 | 相邻 `section` 间距属于 `{bc-4, bc-6, bc-8}` 之一 | 节奏混乱 |

### J1.3 色彩克制

| 检查 | 阈值 | 失败含义 |
|------|------|----------|
| 高饱和色块数量 | 单屏 ≤ 2 个非灰非主色大面积块 | 彩虹后台 |
| 主色滥用 | 主色按钮/链接 ≤ 1 个主 CTA 区域 | 满屏蓝按钮 |
| 非 token 色 | DOM 内联 `style` 颜色 = 0 | 硬编码破坏体系 |

### J1.4 密度与留白

| 检查 | 阈值 | 失败含义 |
|------|------|----------|
| 首屏信息块 | Dashboard 首屏主要模块 ≤ 6 个 | 仪表盘变 Excel |
| 表格行高 | ≥ 40px（触摸友好） | 挤成数据墙 |
| 空白占比 | 主内容区非空元素面积占比 40%–75% | 过空像未完成 / 过满像监控墙 |

> 注：J1 为**辅助红线**；过不了必须修，但过了 J1 仍可能审美不及格——须配合 §J3 人工/Agent 看图。

---

## J2. 截图覆盖（审美专用）

在 `data/loops/aesthetic/latest/` 生成，**每张额外含**:

- `*-full.png` — 全页（`screenshot --full` 或等价）  
- `*-viewport.png` — 首屏视口  
- `*-dark.png`（若已实现 dark）— 同路由暗色  

| 截图名 | 路由 | 审美关注点 |
|--------|------|------------|
| `aesthetic-home` | `/` | 品牌气质、首屏叙事、单一主 CTA |
| `aesthetic-login` | `/login` | 左右分栏平衡、表单呼吸感、错误态 |
| `aesthetic-dashboard` | `/console/dashboard` | 指标卡片节奏、数据层次、空数据优雅 |
| `aesthetic-models` | `/console/models` | CRUD 壳、表格密度、筛选区整洁 |
| `aesthetic-access` | `/console/access` | 凭证卡片、敏感信息展示克制 |
| `aesthetic-settings` | `/console/settings` | 表单分组、Tab 清晰、无配置墙 |
| `aesthetic-finance` | `/console/finance` | 数字排版、金额对齐、图表留白 |
| `aesthetic-monitor` | `/console/monitor` | 告警色克制、状态语义不只靠颜色 |
| `aesthetic-playground` | `/console/playground` | 对话区层次、输入区固定、配置侧栏 |

`manifest.json` 字段:

```json
{
  "status": "pass|fail",
  "screenshots": ["..."],
  "review_json": "review.json",
  "avg_score": 4.2,
  "p0_count": 0
}
```

---

## J3. 逐页审美评审表（Agent 填 `review.json`）

每页按 **C / F / D / A / R / P / E** 七维 1–5 分，并记录 P0 与修改建议。

**通过线**: 七维均 ≥ 4，且**无 P0**。

### P0 硬伤（出现任一项 = 本页不通过）

- [ ] 主 CTA 找不到或与其他按钮视觉权重相同  
- [ ] 正文/说明文字对比度不达标（J1.1 失败）  
- [ ] 明显错位：相邻列/卡片未对齐、图标与文字中线不齐  
- [ ] 空状态只有「暂无数据」无引导（违反 system.md 四态）  
- [ ] 错误态只有错误码/红字，无「发生了什么 + 怎么办」  
- [ ] 单屏 3 种以上互斥视觉风格（如 shadcn 灰卡 + Fluent 蓝 + 自定义渐变）  
- [ ] 装饰性动画/闪烁/无意义渐变背景  
- [ ] 破坏性操作无确认、无撤销提示  

### 各页加分项（Jobs bar）

**Dashboard**

- [ ] 首屏只强调 1 个「今天最重要」指标或行动  
- [ ] 统计卡片的数字、标签、趋势三层清晰  
- [ ] 图表区留白 ≥ 卡片内边距，不顶格贴边  

**列表 / CRUD（models、users、logs）**

- [ ] 筛选区与表格区视觉分离（留白或轻分割，非重边框）  
- [ ] 行 hover 轻柔，选中态明确但不刺眼  
- [ ] 批量操作收起，不常驻抢主操作  

**表单（settings、deploy、access）**

- [ ] 字段分组有副标题，组间距 > 组内间距  
- [ ] 标签与输入左对齐，必填/选填一眼可辨  
- [ ] 主按钮右下或表单末尾唯一 Primary  

**Finance / Monitor**

- [ ] 数字等宽或右对齐，小数位一致  
- [ ] 告警红/黄仅用于真正异常，正常态中性  
- [ ] 图表配色 ≤ 3 色，图例不压数据  

**Playground**

- [ ] 对话气泡留白舒适，用户/助手角色一眼可分  
- [ ] 输入区固定底部，发送按钮唯一强调  
- [ ] 配置侧栏可折叠，默认不抢对话焦点  

**Login / Home（Guest）**

- [ ] 品牌区与表单区 50/50 或黄金比例，不挤  
- [ ] 一句价值主张 ≤ 2 行，副文案 ≤ 3 行  
- [ ] 登录按钮是唯一深色块，OAuth 次级  

---

## J4. 跨页一致性（全局核对）

全部页面截图并排查看时，须满足：

| 项 | 标准 |
|----|------|
| 页头 | `PageHeader` 标题字号、副标题、操作区位置一致 |
| 侧栏 | 选中态、图标大小、分组间距全站统一 |
| 卡片 | 同一 `BCCard` 内边距、圆角、阴影等级一致 |
| 按钮 | Primary 全站同色同高；Secondary/Ghost 不冒充主操作 |
| 表格 | 表头字重、行高、斑马纹/分割线策略统一 |
| 空状态 | 插画/图标风格统一，文案语气统一（i18n） |
| 间距档位 | 页面外边距只用 `bc-6` / `bc-8`，区块间只用 `bc-4` / `bc-6` |
| 动效 | 时长只用 `--bc-transition-fast` / `normal`，无自写曲线 |

---

## J5. 反模式速查（「不像 Jobs」）

| ❌ 避免 | ✅ 改为 |
|--------|--------|
| 每个模块一个不同圆角/阴影 | 全站 2 档圆角 + 1 档阴影 |
| 用边框区分一切 | 用留白 + 轻背景区分 |
| 标题下立刻堆表格 | 标题 → 一句说明 → 留白 → 内容 |
| 5 个蓝色按钮一排 | 1 Primary + 其余 Text/Secondary |
| `text-sm` 密密麻麻说明 | `caption` 少量辅助，详情 Progressive disclosure |
| 灰底上灰字 | 提高对比或降信息密度 |
| 图标风格混用（线面混搭） | 统一 20px 线框图标集 |
| Loading 整页白屏 | 骨架屏或局部 spinner + 文案 |

---

## J6. 单轮 Agent 约束

1. **前置**: 先跑 `check-css-all.ps1`；失败则回到 CSS loop，不修审美。  
2. 每轮最多 **6 个文件**，优先 `review.json` 中分数最低页面相关文件。  
3. 改动类型优先级：**删** > **对齐/间距** > **字号层级** > **颜色克制** > 新增组件。  
4. 不新增装饰性资源（插图、渐变、动画）除非用户明确要求。  
5. 不破坏 i18n、四态、五态（system.md Checklist）。  
6. 改后：`cargo check -p burncloud-client-shared`（触及页面时加 `-p burncloud-client`）。  
7. 每轮结束更新 `review.json` 并附本轮修改的 before/after 截图路径。

---

## J7. 建议 Loop 流程

```
1. check-css-all          → 基础过关
2. capture-aesthetic        → 全页截图到 aesthetic-artifacts/latest
3. check-aesthetic-metrics  → J1 自动度量
4. agent aesthetic-review   → 读图填 review.json，按 J3/J4 提修改
5. 实现修改 → 回到 2
6. review.json 全页 ≥4 且无 P0 → 退出
```

**与 CSS loop 的关系**:

| Loop | 回答的问题 |
|------|------------|
| `burncloud-loop css-optimize` | 类名对不对、有没有破版、token 对不对 |
| `aesthetic-optimize-loop` | 好不好看、像不像 Apple 企业产品、细节够不够 |

---

## J8. 有意不在本 loop 内

- 品牌插画 / 营销文案重写（单独 content loop）  
- 暗色模式全量设计（可单独立项，本 loop 仅抽检 `*-dark.png`）  
- 全新组件库设计（走 `components.md` + 设计评审）  
- 性能 / 无障碍审计（走 system.md 体验条，非审美专项）  

---

## 附录：review.json 模板

```json
{
  "version": 1,
  "reviewed_at": "2026-07-07T00:00:00Z",
  "pages": {
    "aesthetic-dashboard": {
      "scores": { "C": 4, "F": 5, "D": 4, "A": 3, "R": 4, "P": 4, "E": 4 },
      "p0": [],
      "notes": "统计卡第三列与第四列间距不一致，建议统一 gap-bc-6",
      "screenshot": "aesthetic-dashboard-viewport.png"
    }
  },
  "global_j4": {
    "page_header_consistent": true,
    "sidebar_consistent": true,
    "card_consistent": false,
    "notes": "settings 与 finance 卡片内边距不一致"
  },
  "pass": false
}
```

---

## 运行

```powershell
# 乔布斯审美排版 loop（推荐，直到全页达标）
cargo run -p burncloud-loops -- jobs-aesthetic

# 仅诊断，不调 agent
cargo run -p burncloud-loops -- jobs-aesthetic --check-only

# 单次完整验收
cargo run -p burncloud-loops -- gates aesthetic-full

# 仅度量 + 截图
cargo run -p burncloud-loops -- gate aesthetic-metrics
```

Agent prompt：`crates/loops/acceptance/jobs-aesthetic-agent-prompt.md`（约束）  
动态 prompt：`burncloud-loop` → `data/loops/jobs-aesthetic/agent-prompt.md`  
Loop 状态：`data/loops/jobs-aesthetic/loop-state.json`
