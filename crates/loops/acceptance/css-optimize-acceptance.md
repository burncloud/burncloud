# CSS 优化 Loop — 修复范围与验收标准

**权威规范**: `docs/ui/naming.md` · `docs/ui/system.md`  
**自动验收（须全部通过）**:
- `cargo run -p burncloud-loops -- gate css-naming` — 规则 A1–D4、E1（class 静态扫描）
- `cargo run -p burncloud-loops -- gate css-visual` — 规则 V1–V4（截图 + 浏览器内排版检测）  
**单轮执行**: `crates/loops/acceptance/css-optimize-agent-prompt.md`

---

## 扫描范围

| 包含 | 排除 |
|------|------|
| `crates/client/crates/client-*/src`（除 api） | `crates/client-api` |
| `crates/client/src/**` | Guest：`home.rs`、`login.rs`、`forgot_password.rs`、`reset_password.rs` |

仅检查含 `class:` 的 RSX 行；跳过 `//` 注释行。

---

## 验收标准（CI / loop 退出条件）

**命名 + 视觉两项均 exit 0** 后 loop 才结束。

1. `check-css-naming` — 静态 class 规则  
2. `check-ui-conventions` — 按钮规范（已包含在 naming 脚本内）  
3. `check-css-visual` — 截图与运行时排版（见下文 §V）

### A. 间距

| 违规 | 修复为 | 依据 |
|------|--------|------|
| `gap-md`、`p-lg`、`mb-sm` 等 09 短名 | `gap-bc-3`、`p-bc-4`、`mb-bc-2` 等 | naming §5 |
| `gap-xxl`、`gap-xxxl`、`mb-xxxl`、`py-xxxl` 等 | `gap-bc-6`/`gap-bc-8`、`mb-bc-8` 等 | naming §5 |
| `bc-gap-6`、`bc-pl-6` 等 25 工具类 | `gap-bc-6`、`pl-bc-6` | naming §5 |
| `gap-3`、`p-6`、`px-2` 等 Tailwind 数字刻度 | `gap-bc-3`、`p-bc-6`、`px-bc-2` | naming §3.1 |
| 允许保留 | `m-0`、`mb-0`、`p-0` | 零边距例外 |

### B. 颜色与边框

| 违规 | 修复为 |
|------|--------|
| `border-[var(--bc-border)]` | `border-bc-border` |
| `hover:bg-[var(--bc-bg-hover)]` 等 | `hover:bg-bc-hover`（若有对应 Tailwind 映射） |
| `text-gray-*`、`bg-gray-*` | `text-bc-*` / `bg-bc-*` |
| shadcn：`text-muted-foreground`、`bg-card`、`bg-muted`、`text-foreground`、`border-border` | `text-bc-text-secondary`、`bg-bc-card` 等 |

### C. 圆角与阴影

| 违规 | 修复为 |
|------|--------|
| `rounded-md`、`rounded-lg`、`rounded-xl` | `rounded-bc-sm` / `rounded-bc-md` / `rounded-bc-lg` |
| `shadow-sm`（非组件 CSS 内） | `shadow-bc-sm` |
| `rounded-[32px]`、`shadow-[rgba(...)]` 任意值 | `rounded-bc-lg` + `shadow-bc-*` 或组件 CSS 变量 |

### D. 字号排版（控制台）

| 违规 | 修复为 |
|------|--------|
| `text-2xl`、`text-3xl`、`text-base`、`text-lg`（Tailwind 默认） | `text-title`、`text-large-title`、`text-body`、`text-bc-sm` 等 |
| `text-xxs` | `text-caption` 或 `text-bc-sm` |
| `text-display` | `text-large-title` |
| `text-[11px]` 等任意字号 | `text-caption` 或 `text-bc-sm` |

语义类 `text-caption` / `text-subtitle` / `text-body` 与 `text-bc-text-*` 颜色类组合使用；避免再叠 `font-bold`（除非组件规范要求）。

### E. 按钮

| 违规 | 修复为 |
|------|--------|
| `<button class="btn btn-primary">` | `BCButton { variant: ... }` |

---

## V. 视觉与排版验收（`check-css-visual`）

**前置**: `npm install -g agent-browser && agent-browser install`；仓库已 `cargo build --bin burncloud`（或设置 `E2E_BASE_URL` 指向运行中实例）。

**产物**: `data/loops/css-visual/latest/*.png` + `manifest.json`

### V1. 截图覆盖（12 张控制台 + 1 张首页）

| 截图名 | 路由 | 说明 |
|--------|------|------|
| `visual-home` | `/` | 营销首页 token |
| `visual-dashboard` | `/console` | 仪表盘 |
| `visual-models` | `/console/models` | CRUD 壳 |
| `visual-access` | `/console/access` | |
| `visual-deploy` | `/console/deploy` | |
| `visual-monitor` | `/console/monitor` | |
| `visual-logs` | `/console/logs` | admin |
| `visual-users` | `/console/users` | admin |
| `visual-settings` | `/console/settings` | |
| `visual-finance` | `/console/finance` | |
| `visual-connect` | `/console/connect` | |
| `visual-playground` | `/console/playground` | admin |

失败时额外生成 `FAIL-load-*` / `FAIL-layout-*` 截图。

### V2. 浏览器内布局规则（每页 JS 检测）

| 检查 | 失败含义 |
|------|----------|
| 无横向溢出 | `scrollWidth > innerWidth` |
| 无遗留 class 出现在 DOM | `gap-md`、`text-muted-foreground`、`bg-card`、`rounded-xl` 等 |
| 存在内容壳 | `.page-content` 或 `.animate-fade-in` 或 `.stats-grid` |
| `--bc-primary` | 须为 Apple `#007AFF` |

### V3. 人工/Agent 视觉核对（看截图）

- 间距节奏一致：同级卡片/列表 `gap-bc-*` 统一，无挤在一起或过大留白  
- 字号层级 ≤ 5 级，标题用 `text-title` / `text-subtitle`，非 `text-2xl`  
- 圆角/阴影统一 `rounded-bc-*` / `shadow-bc-*`，无 shadcn 灰卡片感  
- 侧边栏与主内容区对齐，无元素重叠、裁切  
- 主色为 Apple 蓝，无 Fluent `#0078d4` 色块  

### V4. 失败时 Agent 动作

1. 打开 `data/loops/css-visual/latest/` 中 `FAIL-*` 与对应页截图  
2. 对照 §V3 修复 RSX/CSS，每轮最多 8 个文件  
3. 重跑 `check-css-visual.ps1`

---

## 单轮 Agent 约束

1. 每轮最多 **8 个文件**，优先 `check-css-naming` 报错最多的文件。
2. 不改 Guest 域、不扩展 `client-api`。
3. 不新增 shadcn 类、裸 hex、`style:` 写间距/颜色（`--bc-dynamic-*` 动态色除外）。
4. 不改根目录 `.gitignore`；不 `git commit` 除非用户要求。
5. 改后执行 `cargo check -p burncloud-client-shared`（触及 client 主 crate 时加 `-p burncloud-client`）。

---

## 有意不在本 loop 验收内（后续 PR）

- `bc-grid-*`、`bc-font-*` 布局/字号捷径（待 components.md 白名单或 Tailwind 化）
- `client-api` 全 crate 迁移
- `home.rs` Fluent 文案 → Apple HIG 文案
- Guest 页 `landing-*` / `mb-xxxl`
- `glass_card` 毛玻璃 `bg-white/70` 体系化（需 token 扩展）
- i18n 硬编码英文（system.md R1，非 CSS loop）

---

## 运行

```powershell
# 完整 loop（命名 + 视觉）
cargo run -p burncloud-loops -- css-optimize

# 仅静态命名
.\scripts\check-css-naming.ps1

# 仅视觉截图 + 排版
.\scripts\check-css-visual.ps1

# 两项一起
.\scripts\check-css-all.ps1
```

Loop 在 **命名 + 视觉** 均通过后打印 `Done: all CSS acceptance checks passed.` 并退出。
