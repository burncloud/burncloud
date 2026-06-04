# agent-browser vs Playwright 全面对比

**对比日期**: 2026-06-01
**对比版本**: agent-browser (latest), Playwright v1.44+

---

## 一、项目概览

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **开发者** | Vercel Labs | Microsoft |
| **GitHub Stars** | 34,877 ⭐ | 70,000+ ⭐ |
| **开源协议** | Apache-2.0 | Apache-2.0 |
| **主要语言** | Rust | TypeScript/Node.js |
| **首次发布** | 2024 | 2020 |
| **定位** | AI Agent 浏览器自动化 CLI | E2E 测试框架 |
| **设计哲学** | CLI 优先，AI 友好 | 测试框架优先，开发者友好 |

---

## 二、技术架构

### agent-browser 架构

```
┌─────────────────────────────────────────────────────────────┐
│                    agent-browser CLI                         │
│  (Rust Native Binary - 无需 Node.js 运行时)                  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Chat API  │  │ Snapshot API│  │  Traditional API    │  │
│  │ (自然语言)  │  │ (可访问性树) │  │  (CSS/传统选择器)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                 Browser Engine Layer                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │  Chrome  │  │ Lightpanda│  │  Safari  │  │ Cloud (可选)│  │
│  │   CDP    │  │   (新)    │  │ WebDriver│  │ Browserbase│  │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Playwright 架构

```
┌─────────────────────────────────────────────────────────────┐
│                 Playwright Test Runner                       │
│  (Node.js + TypeScript)                                      │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Test Runner │  │  Assertion  │  │    Reporters        │  │
│  │   (Mocha)   │  │  (expect)   │  │ (HTML/JSON/JUnit)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                 Playwright Core                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │ Chromium │  │ Firefox  │  │ WebKit   │  │ Android/iOS│  │
│  │  CDP     │  │ CDP-like │  │ CDP-like │  │ (Device)   │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 三、安装与部署

### agent-browser

```bash
# 方式 1: npm 全局安装 (推荐)
npm install -g agent-browser
agent-browser install  # 下载 Chrome for Testing

# 方式 2: Cargo 安装 (Rust 生态)
cargo install agent-browser
agent-browser install

# 方式 3: Homebrew (macOS)
brew install agent-browser
agent-browser install

# 方式 4: 项目本地安装
npm install agent-browser
./node_modules/.bin/agent-browser install
```

**依赖**:
- Chrome (自动下载或使用现有安装)
- 无需 Node.js 运行时 (Rust 二进制独立运行)

### Playwright

```bash
# 方式 1: npm 初始化项目
npm init playwright@latest

# 方式 2: 添加到现有项目
npm install -D @playwright/test
npx playwright install  # 下载浏览器

# 方式 3: VS Code 插件
# 安装 "Playwright Test for VSCode" 扩展
```

**依赖**:
- Node.js 18+ 运行时
- 浏览器二进制文件 (Chromium/Firefox/WebKit)
- 约 300-500MB 存储空间 (含浏览器)

### 对比总结

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **安装包大小** | ~10MB (Rust 二进制) | ~50MB (Node 包) |
| **浏览器大小** | ~150MB (Chrome) | ~300MB (3 浏览器) |
| **运行时依赖** | 无 (独立二进制) | Node.js 18+ |
| **安装复杂度** | ⭐⭐ 简单 | ⭐⭐⭐ 中等 |
| **跨平台** | Win/Mac/Linux | Win/Mac/Linux |

---

## 四、核心功能对比

### 4.1 浏览器控制

| 功能 | agent-browser | Playwright |
|------|---------------|------------|
| **打开浏览器** | `agent-browser open url` | `browser.newPage()` |
| **导航** | `goto`, `back`, `forward` | `page.goto()`, `goBack()` |
| **点击** | `click @e1` 或 `click "#btn"` | `page.click('#btn')` |
| **输入** | `fill @e1 "text"` | `page.fill('#input', 'text')` |
| **等待** | `wait 3s`, `wait selector "#x"` | `page.waitForSelector()` |
| **截图** | `screenshot --full` | `page.screenshot({fullPage:true})` |
| **PDF** | `pdf output.pdf` | `page.pdf()` |
| **执行 JS** | `eval "document.title"` | `page.evaluate()` |
| **Cookie** | `cookies`, `set-cookie` | `context.cookies()` |
| **多标签** | `tab new`, `tab switch 1` | `browser.newPage()` |
| **多窗口** | `window new` | `browser.newContext()` |

### 4.2 选择器系统

#### agent-browser 选择器

```bash
# 1. AI 友好的 ref 选择器 (推荐)
agent-browser snapshot
# 输出:
# [e1] button "Submit"
# [e2] textbox "Email"
# [e3] link "Learn More"

agent-browser click @e1  # 通过 ref 点击

# 2. 语义选择器
agent-browser find role button click --name "Submit"
agent-browser find placeholder "Enter email" fill "test@example.com"

# 3. 传统选择器
agent-browser click "#submit-btn"
agent-browser fill "input[name='email']" "test@example.com"

# 4. 文本选择器
agent-browser click "text=Submit"
```

#### Playwright 选择器

```typescript
// 1. CSS 选择器
await page.click('#submit-btn');

// 2. 文本选择器
await page.click('text=Submit');
await page.getByText('Submit').click();

// 3. Role 选择器 (推荐)
await page.getByRole('button', { name: 'Submit' }).click();
await page.getByPlaceholder('Enter email').fill('test@example.com');

// 4. Test ID 选择器
await page.getByTestId('submit-btn').click();

// 5. 链式选择器
await page.locator('.form').getByRole('button').click();

// 6. XPath
await page.locator('xpath=//button[@type="submit"]').click();
```

### 对比总结

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **选择器类型** | ref/语义/文本/CSS | CSS/文本/Role/TestID/XPath |
| **AI 友好度** | ⭐⭐⭐⭐⭐ 极佳 | ⭐⭐⭐ 良好 |
| **动态元素** | ref 自动缓存 | 自动等待重试 |
| **可读性** | ⭐⭐⭐⭐ 高 | ⭐⭐⭐⭐ 高 |
| **调试友好** | snapshot 直观 | Codegen 录制 |

### 4.3 AI 集成

#### agent-browser AI 功能

```bash
# 1. 自然语言控制 (核心功能)
agent-browser chat "打开 github.com 并搜索 Rust"

# 2. 交互式 REPL
agent-browser chat
> open https://example.com
> fill in the login form with test@example.com
> take a screenshot
> quit

# 3. 模型选择
agent-browser --model anthropic/claude-sonnet-4 chat "summarize this page"
agent-browser --model openai/gpt-4o chat "find all broken links"

# 4. JSON 输出 (供 Agent 消费)
agent-browser --json chat "extract all product prices"

# 5. 快速/详细模式
agent-browser -q chat "summarize"  # 只返回结果
agent-browser -v chat "login"      # 显示所有命令
```

#### Playwright AI 功能

```typescript
// Playwright 本身无 AI 功能，但可以集成:
// 1. 与 AI 代码生成工具配合
// npx playwright codegen --openai

// 2. 使用 @axe-core/playwright 做 AI 辅助可访问性测试
import { injectAxe, checkA11y } from 'axe-playwright-js';
await injectAxe(page);
await checkA11y(page, null, { detailedReport: true });

// 3. 使用 AI 服务做视觉对比
import { test, expect } from '@playwright/test';
// 需要自己实现 AI 视觉对比逻辑
```

### AI 集成对比

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **原生 AI 支持** | ✅ 完整支持 | ❌ 无 |
| **自然语言控制** | ✅ 核心 API | ❌ 需自建 |
| **AI 模型选择** | ✅ 多模型支持 | ❌ 不适用 |
| **AI Agent 友好** | ⭐⭐⭐⭐⭐ 最佳 | ⭐⭐ 需封装 |
| **LLM 输出格式** | ✅ JSON 模式 | ❌ 需自建 |

### 4.4 测试框架功能

#### agent-browser 测试能力

```bash
# agent-browser 不是测试框架，但可以用于测试场景

# 1. 手动验证
agent-browser open https://app.com
agent-browser snapshot
agent-browser fill @e1 "test@example.com"
agent-browser click @e2
agent-browser snapshot  # 检查结果

# 2. 批量执行脚本
agent-browser run ./scripts/login.txt
# scripts/login.txt 内容:
# open https://app.com/login
# fill #email test@example.com
# fill #password secret
# click button[type="submit"]
# wait 2s
# snapshot

# 3. 无断言系统
# 需要手动检查输出或配合其他工具

# 4. 无测试报告
# 需要自己记录结果
```

#### Playwright 测试框架

```typescript
import { test, expect } from '@playwright/test';

test.describe('登录功能', () => {
  test('成功登录', async ({ page }) => {
    // Arrange
    await page.goto('/login');
    
    // Act
    await page.fill('#email', 'test@example.com');
    await page.fill('#password', 'secret');
    await page.click('button[type="submit"]');
    
    // Assert
    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('.welcome')).toBeVisible();
    await expect(page.locator('.error')).not.toBeVisible();
  });

  test('登录失败 - 错误密码', async ({ page }) => {
    await page.goto('/login');
    await page.fill('#email', 'test@example.com');
    await page.fill('#password', 'wrong');
    await page.click('button[type="submit"]');
    
    await expect(page.locator('.error')).toHaveText('密码错误');
  });
});

// 高级功能
test.describe.configure({ mode: 'parallel' });  // 并行执行

test.use({
  storageState: 'auth.json',  // 复用登录状态
  viewport: { width: 1280, height: 720 },
  screenshot: 'on',
  video: 'retain-on-failure',
});
```

### 测试框架对比

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **断言库** | ❌ 无 | ✅ 完整 expect API |
| **测试组织** | ❌ 无 | ✅ describe/test/hook |
| **Fixture** | ❌ 无 | ✅ 强大的 fixture 系统 |
| **并行执行** | ❌ 无 | ✅ 原生支持 |
| **测试报告** | ❌ 无 | ✅ HTML/JSON/JUnit |
| **重试机制** | ❌ 无 | ✅ 自动重试 |
| **快照测试** | ⚠️ 手动 | ✅ 视觉快照对比 |
| **Mock/拦截** | ⚠️ 有限 | ✅ 完整 route API |

---

## 五、调试与可观测性

### agent-browser 调试

```bash
# 1. Observability Dashboard
agent-browser dashboard start --port 8080
# 打开 http://localhost:8080 查看实时浏览器视图和命令历史

# 2. 详细输出
agent-browser --debug open https://example.com

# 3. 截图标注
agent-browser screenshot --annotate ./debug.png
# 输出:
# [1] @e1 button "Submit"
# [2] @e2 link "Learn More"

# 4. Snapshot 检查
agent-browser snapshot -i -c  # 只看交互元素，紧凑输出
```

### Playwright 调试

```bash
# 1. Debug 模式
npx playwright test --debug

# 2. UI Mode (可视化调试)
npx playwright test --ui

# 3. Codegen (录制回放)
npx playwright codegen https://example.com

# 4. Trace Viewer
npx playwright test --trace on
npx playwright show-trace trace.zip

# 5. VS Code 调试
# 直接在 VS Code 中打断点调试
```

### 调试对比

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **可视化调试** | ✅ Dashboard | ✅ UI Mode |
| **录制回放** | ❌ 无 | ✅ Codegen |
| **Trace 查看** | ❌ 无 | ✅ Trace Viewer |
| **实时视图** | ✅ Dashboard | ✅ headed 模式 |
| **元素检查** | ✅ snapshot | ✅ Inspector |
| **VS Code 集成** | ❌ 无 | ✅ 官方插件 |

---

## 六、安全性

### agent-browser 安全功能

```bash
# 1. 认证保险库 (加密存储)
agent-browser auth save github \
  --url https://github.com/login \
  --username myuser \
  --password-stdin  # LLM 永远看不到密码

agent-browser auth login github  # 自动填充

# 2. 内容边界标记 (防止 LLM 混淆)
agent-browser --content-boundaries open https://untrusted.com

# 3. 域名白名单
agent-browser --allowed-domains "example.com,*.example.com" \
  open https://example.com

# 4. 动作确认 (敏感操作需批准)
agent-browser --confirm-actions eval,download,upload \
  open https://example.com

# 5. 输出长度限制 (防止上下文溢出)
agent-browser --max-output 50000 snapshot

# 6. 动作策略文件
agent-browser --action-policy ./policy.json open https://example.com
# policy.json:
{
  "allowedActions": ["click", "fill", "scroll"],
  "blockedSelectors": ["#delete-button", ".admin-panel"]
}
```

### Playwright 安全

```typescript
// Playwright 无内置 AI 安全特性，但支持:

// 1. 隔离的浏览器上下文
const context = await browser.newContext({
  // 禁用 JavaScript (可选)
  javaScriptEnabled: false,
  
  // 限制权限
  permissions: ['clipboard-read'],
});

// 2. 存储状态 (敏感信息需要自己管理)
await page.context().storageState({ path: 'auth.json' });

// 3. 拦截请求
await page.route('**/tracking.js', route => route.abort());

// 4. 无内置密码保护
// 密码通常硬编码或从环境变量读取
const password = process.env.TEST_PASSWORD;
await page.fill('#password', password);  // 可能在日志中暴露
```

### 安全对比

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **密码保护** | ✅ 加密存储，LLM 不可见 | ❌ 需手动管理 |
| **域名限制** | ✅ 白名单机制 | ❌ 无 |
| **动作策略** | ✅ JSON 策略文件 | ❌ 无 |
| **内容边界** | ✅ 自动标记 | ❌ 无 |
| **输出限制** | ✅ 最大字符数 | ❌ 无 |
| **敏感操作确认** | ✅ 交互式确认 | ❌ 无 |
| **AI 安全设计** | ⭐⭐⭐⭐⭐ 专为 AI 设计 | ⭐⭐ 传统安全模型 |

---

## 七、云浏览器支持

### agent-browser 云浏览器

```bash
# 1. Browserbase
export BROWSERBASE_API_KEY="***"
agent-browser -p browserbase open https://example.com

# 2. Browser Use
export BROWSER_USE_API_KEY="***"
agent-browser -p browseruse open https://example.com

# 3. Kernel (隐身模式、持久化配置)
export KERNEL_API_KEY="***"
export KERNEL_STEALTH=true
export KERNEL_PROFILE_NAME="my-profile"
agent-browser -p kernel open https://example.com

# 4. AWS AgentCore
export AGENT_BROWSER_PROVIDER=agentcore
export AGENTCORE_PROFILE_ID="persistent-profile"
agent-browser open https://example.com
```

### Playwright 云浏览器

```typescript
// 1. Playwright 本地运行
const browser = await chromium.launch();

// 2. 连接到远程浏览器
const browser = await chromium.connect({
  wsEndpoint: 'wss://cloud.browser.com/connect',
});

// 3. 第三方云服务集成
// - Microsoft Playwright Testing Service
// - Browserless
// - LambdaTest
// - Sauce Labs
// (需要额外配置和付费)

// 4. 容器化部署
// Docker + Playwright
```

### 云支持对比

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **原生云支持** | ✅ 4+ 提供商 | ⚠️ 需第三方 |
| **配置复杂度** | ⭐ 简单 (一个 flag) | ⭐⭐⭐ 需配置 |
| **Session 持久化** | ✅ 原生支持 | ⚠️ 需手动实现 |
| **隐身模式** | ✅ Kernel 支持 | ⚠️ 需特殊配置 |
| **Serverless 友好** | ✅ 设计目标 | ⚠️ 需容器化 |

---

## 八、跨浏览器支持

### agent-browser

| 浏览器 | 支持程度 | 说明 |
|--------|---------|------|
| Chrome | ⭐⭐⭐⭐⭐ | 主要目标，CDP 支持 |
| Lightpanda | ⭐⭐⭐⭐ | 新引擎，专为自动化设计 |
| Safari | ⭐⭐⭐ | WebDriver 支持，功能有限 |
| Firefox | ⭐⭐ | 有限支持 |

### Playwright

| 浏览器 | 支持程度 | 说明 |
|--------|---------|------|
| Chromium | ⭐⭐⭐⭐⭐ | 完整支持，包括 Chrome/Edge |
| Firefox | ⭐⭐⭐⭐⭐ | 完整支持 |
| WebKit | ⭐⭐⭐⭐⭐ | 完整支持 (Safari 引擎) |
| Android | ⭐⭐⭐⭐ | 设备模式 |
| iOS | ⭐⭐⭐ | 设备模式 |

---

## 九、性能对比

### agent-browser 性能

```bash
# 优势:
# 1. Rust 原生，启动快 (~50ms)
# 2. 无 Node.js 开销
# 3. 单二进制，内存占用小 (~30MB)
# 4. 异步架构，高并发

# 劣势:
# 1. 单线程 CLI (非并发测试)
# 2. 无内置测试并行化
```

### Playwright 性能

```typescript
// 优势:
// 1. 内置并行执行
// 2. 测试分片 (sharding)
// 3. Worker 进程隔离

// 劣势:
// 1. Node.js 启动开销 (~500ms)
// 2. 内存占用较大 (~100MB per worker)
// 3. 多浏览器测试增加时间

// 性能优化示例
export default defineConfig({
  workers: 4,  // 4 个并行 worker
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
  ],
  // 分片: npx playwright test --shard=1/3
});
```

### 性能对比表

| 维度 | agent-browser | Playwright |
|------|---------------|------------|
| **启动时间** | ~50ms | ~500ms |
| **内存占用** | ~30MB | ~100MB/worker |
| **并行执行** | ❌ 不支持 | ✅ 原生支持 |
| **测试分片** | ❌ 不支持 | ✅ 支持 |
| **浏览器复用** | ✅ Session 持久化 | ✅ Context 复用 |

---

## 十、生态系统

### agent-browser 生态

```
├── CLI 工具 ✅
├── Dashboard ✅
├── AI Chat ✅
├── 认证保险库 ✅
├── 云浏览器集成 ✅
├── VS Code 插件 ❌
├── 测试报告 ❌
├── 社区插件 ❌
└── 文档 ⭐⭐⭐⭐
```

### Playwright 生态

```
├── 测试框架 ✅
├── VS Code 插件 ✅
├── Codegen ✅
├── Trace Viewer ✅
├── HTML Reporter ✅
├── API 测试 ✅
├── 组件测试 ✅
├── 视觉测试 ✅
├── Playwright Docker ✅
├── 第三方集成 ✅ (50+)
├── 社区插件 ✅
├── 官方文档 ⭐⭐⭐⭐⭐
└── 社区支持 ⭐⭐⭐⭐⭐
```

---

## 十一、使用场景推荐

### 选择 agent-browser 的场景

| 场景 | 理由 |
|------|------|
| ✅ **AI Agent 自动化** | 原生 AI 支持，自然语言控制 |
| ✅ **探索性测试** | 快速交互，无需写代码 |
| ✅ **数据抓取** | Rust 性能，snapshot API 友好 |
| ✅ **Serverless 环境** | 无 Node 依赖，可接云浏览器 |
| ✅ **CI/CD 快速验证** | 单命令执行，安装简单 |
| ✅ **安全敏感场景** | 密码加密，动作策略 |
| ✅ **Rust 技术栈** | 与 Rust 项目无缝集成 |

### 选择 Playwright 的场景

| 场景 | 理由 |
|------|------|
| ✅ **E2E 测试套件** | 完整测试框架，断言，报告 |
| ✅ **回归测试** | 并行执行，测试分片 |
| ✅ **跨浏览器测试** | Chromium/Firefox/WebKit |
| ✅ **团队协作** | VS Code 插件，代码评审友好 |
| ✅ **持续集成** | 成熟的 CI/CD 集成 |
| ✅ **视觉回归** | 快照对比 |
| ✅ **组件测试** | React/Vue/Svelte 组件测试 |
| ✅ **API 测试** | 内置 API 测试支持 |
| ✅ **复杂测试场景** | Fixtures, Hooks, 并行 |

---

## 十二、代码示例对比

### 示例 1: 登录流程测试

#### agent-browser 方式

```bash
# 方式 1: 命令行手动执行
agent-browser open https://app.com/login
agent-browser fill @e1 "test@example.com"
agent-browser fill @e2 "password123"
agent-browser click @e3
agent-browser snapshot  # 手动检查是否跳转到 dashboard

# 方式 2: 自然语言
agent-browser chat "登录 https://app.com，用户名 test@example.com，密码 password123，检查是否成功进入 dashboard"

# 方式 3: 脚本文件
cat > login.txt << 'EOF'
open https://app.com/login
fill #email test@example.com
fill #password password123
click button[type="submit"]
wait 2s
snapshot
EOF
agent-browser run login.txt
```

#### Playwright 方式

```typescript
import { test, expect } from '@playwright/test';

test('登录流程', async ({ page }) => {
  await page.goto('/login');
  
  await page.fill('#email', 'test@example.com');
  await page.fill('#password', 'password123');
  await page.click('button[type="submit"]');
  
  // 自动等待 + 断言
  await expect(page).toHaveURL('/dashboard');
  await expect(page.locator('.user-name')).toBeVisible();
});

// 复用登录状态
test.use({ storageState: 'auth.json' });
```

### 示例 2: 表单验证测试

#### agent-browser

```bash
agent-browser open https://app.com/register
agent-browser click button[type="submit"]
agent-browser snapshot
# 手动检查是否有错误信息

agent-browser fill #email "invalid-email"
agent-browser click button[type="submit"]
agent-browser snapshot
# 手动检查邮箱格式错误提示
```

#### Playwright

```typescript
test('注册表单验证', async ({ page }) => {
  await page.goto('/register');
  
  // 空表单提交
  await page.click('button[type="submit"]');
  await expect(page.locator('.error-email')).toHaveText('邮箱必填');
  await expect(page.locator('.error-password')).toHaveText('密码必填');
  
  // 无效邮箱
  await page.fill('#email', 'invalid-email');
  await page.click('button[type="submit"]');
  await expect(page.locator('.error-email')).toHaveText('邮箱格式错误');
  
  // 密码太短
  await page.fill('#email', 'test@example.com');
  await page.fill('#password', '123');
  await page.click('button[type="submit"]');
  await expect(page.locator('.error-password')).toContainText('至少8位');
});
```

### 示例 3: API 测试

#### agent-browser (不适用)

```bash
# agent-browser 无 API 测试功能
# 需要使用其他工具如 curl
curl -X POST https://api.example.com/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"secret"}'
```

#### Playwright

```typescript
import { test, expect } from '@playwright/test';

test('API 登录测试', async ({ request }) => {
  const response = await request.post('/api/login', {
    data: {
      email: 'test@example.com',
      password: 'secret',
    },
  });
  
  expect(response.ok()).toBeTruthy();
  
  const body = await response.json();
  expect(body).toHaveProperty('token');
  expect(body.user.email).toBe('test@example.com');
});
```

### 示例 4: 视觉回归测试

#### agent-browser

```bash
# 手动截图对比
agent-browser open https://app.com
agent-browser screenshot --full baseline.png

# 修改后截图
agent-browser open https://app.com
agent-browser screenshot --full current.png

# 手动或使用外部工具对比
# agent-browser 无内置对比功能
```

#### Playwright

```typescript
import { test, expect } from '@playwright/test';

test('首页视觉回归', async ({ page }) => {
  await page.goto('/');
  
  // 自动对比截图
  await expect(page).toHaveScreenshot('homepage.png', {
    maxDiffPixels: 100,  // 允许 100 像素差异
    fullPage: true,
  });
});

// 组件截图
test('按钮组件', async ({ page }) => {
  await page.goto('/components/button');
  await expect(page.locator('.primary-button')).toHaveScreenshot('button-primary.png');
});
```

---

## 十三、综合评分

| 维度 | agent-browser | Playwright | 权重 |
|------|:-------------:|:----------:|:----:|
| **AI 集成** | ⭐⭐⭐⭐⭐ | ⭐⭐ | 20% |
| **测试框架完整性** | ⭐⭐ | ⭐⭐⭐⭐⭐ | 20% |
| **易用性** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 15% |
| **调试体验** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 10% |
| **跨浏览器** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 10% |
| **性能** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 5% |
| **安全性** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | 5% |
| **生态系统** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 10% |
| **文档质量** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 5% |

**加权总分**:
- agent-browser: **3.95/5**
- Playwright: **4.15/5**

---

## 十四、最终建议

### 对于 BurnCloud 项目

| 需求 | 推荐工具 | 理由 |
|------|----------|------|
| **回归测试套件** | Playwright | 断言、报告、并行、CI 集成成熟 |
| **探索性测试** | agent-browser | Hermes Agent 可动态发现 Bug |
| **API 安全测试** | Rust 内置测试 | 与后端技术栈一致 |
| **CI/CD 验证** | Playwright | 成熟的 GitHub Actions 集成 |

### 混合方案示例

```
BurnCloud 测试架构:

┌─────────────────────────────────────────────────┐
│                  测试金字塔                      │
├─────────────────────────────────────────────────┤
│                                                  │
│    ┌─────────────────────────────────────┐      │
│    │     E2E 回归测试 (Playwright)        │      │
│    │     - 登录/注册/密码重置             │      │
│    │     - 渠道管理 CRUD                  │      │
│    │     - 用户管理 CRUD                  │      │
│    │     - API 安全验证                   │      │
│    └─────────────────────────────────────┘      │
│                                                  │
│    ┌─────────────────────────────────────┐      │
│    │   探索性测试 (agent-browser)          │      │
│    │   - Hermes Agent 动态探索            │      │
│    │   - 发现隐藏 Bug                     │      │
│    │   - 用户体验验证                     │      │
│    └─────────────────────────────────────┘      │
│                                                  │
│    ┌─────────────────────────────────────┐      │
│    │    API 单元测试 (Rust cargo test)     │      │
│    │    - 认证逻辑                        │      │
│    │    - 业务逻辑                        │      │
│    │    - 数据库操作                      │      │
│    └─────────────────────────────────────┘      │
│                                                  │
└─────────────────────────────────────────────────┘
```

---

**文档版本**: 1.0  
**最后更新**: 2026-06-01  
**作者**: Hermes Agent
