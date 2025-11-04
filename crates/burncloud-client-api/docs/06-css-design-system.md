# CSS 设计系统文档

## 设计系统概述

BurnCloud Client API 采用模块化、原子化的 CSS 设计系统，提供一致性的视觉体验和高效的样式管理。设计系统基于实用优先（Utility-First）的方法，结合现代化的设计原则。

## 核心设计原则

### 1. 原子化设计
- **单一职责**: 每个 CSS 类专注于单一样式属性
- **组合优先**: 通过组合多个类实现复杂样式
- **可预测性**: 类名直观反映其功能

### 2. 一致性标准
- **统一的间距系统**: 基于 8px 基准的间距规范
- **标准化颜色**: 语义化的颜色变量系统
- **响应式设计**: 移动优先的响应式布局

### 3. 可维护性
- **模块化结构**: 按功能分组的样式模块
- **语义化命名**: 清晰的类名约定
- **文档化**: 完整的样式文档和示例

## 颜色系统

### 1. 主色调定义

```css
/* 主要颜色 */
:root {
  --color-primary: #1a73e8;        /* 主要品牌色 */
  --color-primary-hover: #1557b0;  /* 主色悬停态 */
  --color-primary-light: #e8f0fe;  /* 主色浅色版 */

  --color-secondary: #666;         /* 次要文本色 */
  --color-tertiary: #999;          /* 三级文本色 */

  --color-success: #2e7d2e;        /* 成功状态 */
  --color-warning: #f57c00;        /* 警告状态 */
  --color-error: #d32f2f;          /* 错误状态 */
  --color-info: #1976d2;           /* 信息状态 */
}
```

### 2. 文本颜色类

```css
/* 文本颜色 */
.text-primary {
  color: var(--color-primary);
}

.text-secondary {
  color: var(--color-secondary);
}

.text-tertiary {
  color: var(--color-tertiary);
}

.text-success {
  color: var(--color-success);
}

.text-warning {
  color: var(--color-warning);
}

.text-error {
  color: var(--color-error);
}
```

### 3. 背景颜色类

```css
/* 背景颜色 */
.bg-primary {
  background-color: var(--color-primary);
}

.bg-primary-light {
  background-color: var(--color-primary-light);
}

.bg-success-light {
  background-color: #e8f5e8;
}

.bg-warning-light {
  background-color: #fff3e0;
}

.bg-error-light {
  background-color: #ffebee;
}
```

## 排版系统

### 1. 字体家族

```css
/* 字体定义 */
:root {
  --font-family-primary: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
  --font-family-mono: 'Fira Code', 'Consolas', 'Monaco', monospace;
}

body {
  font-family: var(--font-family-primary);
  line-height: 1.5;
  color: #333;
}
```

### 2. 文本尺寸类

```css
/* 文本尺寸 */
.text-xs {
  font-size: 0.75rem;   /* 12px */
  line-height: 1.2;
}

.text-sm {
  font-size: 0.875rem;  /* 14px */
  line-height: 1.25;
}

.text-base {
  font-size: 1rem;      /* 16px */
  line-height: 1.5;
}

.text-lg {
  font-size: 1.125rem;  /* 18px */
  line-height: 1.4;
}

.text-xl {
  font-size: 1.25rem;   /* 20px */
  line-height: 1.4;
}

.text-subtitle {
  font-size: 1.25rem;   /* 20px */
  line-height: 1.4;
}

.text-large-title {
  font-size: 2.5rem;    /* 40px */
  line-height: 1.2;
}

.text-caption {
  font-size: 0.875rem;  /* 14px */
  line-height: 1.3;
}
```

### 3. 字重类

```css
/* 字重 */
.font-thin {
  font-weight: 100;
}

.font-light {
  font-weight: 300;
}

.font-normal {
  font-weight: 400;
}

.font-medium {
  font-weight: 500;
}

.font-semibold {
  font-weight: 600;
}

.font-bold {
  font-weight: 700;
}

.font-extrabold {
  font-weight: 800;
}

.font-black {
  font-weight: 900;
}
```

## 间距系统

### 1. 间距标准

```css
/* 间距变量 */
:root {
  --spacing-xs: 0.25rem;   /* 4px */
  --spacing-sm: 0.5rem;    /* 8px */
  --spacing-md: 1rem;      /* 16px */
  --spacing-lg: 1.5rem;    /* 24px */
  --spacing-xl: 2rem;      /* 32px */
  --spacing-2xl: 3rem;     /* 48px */
  --spacing-3xl: 4rem;     /* 64px */
}
```

### 2. 外边距类

```css
/* 外边距 - 全方向 */
.m-0 { margin: 0; }
.m-xs { margin: var(--spacing-xs); }
.m-sm { margin: var(--spacing-sm); }
.m-md { margin: var(--spacing-md); }
.m-lg { margin: var(--spacing-lg); }
.m-xl { margin: var(--spacing-xl); }

/* 外边距 - 单方向 */
.mt-0 { margin-top: 0; }
.mt-xs { margin-top: var(--spacing-xs); }
.mt-sm { margin-top: var(--spacing-sm); }
.mt-md { margin-top: var(--spacing-md); }
.mt-lg { margin-top: var(--spacing-lg); }

.mb-0 { margin-bottom: 0; }
.mb-xs { margin-bottom: var(--spacing-xs); }
.mb-sm { margin-bottom: var(--spacing-sm); }
.mb-md { margin-bottom: var(--spacing-md); }
.mb-lg { margin-bottom: var(--spacing-lg); }

.ml-0 { margin-left: 0; }
.ml-sm { margin-left: var(--spacing-sm); }
.ml-md { margin-left: var(--spacing-md); }

.mr-0 { margin-right: 0; }
.mr-sm { margin-right: var(--spacing-sm); }
.mr-md { margin-right: var(--spacing-md); }
```

### 3. 内边距类

```css
/* 内边距 - 全方向 */
.p-0 { padding: 0; }
.p-xs { padding: var(--spacing-xs); }
.p-sm { padding: var(--spacing-sm); }
.p-md { padding: var(--spacing-md); }
.p-lg { padding: var(--spacing-lg); }
.p-xl { padding: var(--spacing-xl); }

/* 内边距 - 单方向 */
.pt-sm { padding-top: var(--spacing-sm); }
.pb-sm { padding-bottom: var(--spacing-sm); }
.pl-md { padding-left: var(--spacing-md); }
.pr-md { padding-right: var(--spacing-md); }

/* 内边距 - 轴向 */
.px-md {
  padding-left: var(--spacing-md);
  padding-right: var(--spacing-md);
}

.py-sm {
  padding-top: var(--spacing-sm);
  padding-bottom: var(--spacing-sm);
}
```

## 布局系统

### 1. Flexbox 布局

```css
/* Flex 容器 */
.flex {
  display: flex;
}

.inline-flex {
  display: inline-flex;
}

/* Flex 方向 */
.flex-row {
  flex-direction: row;
}

.flex-col {
  flex-direction: column;
}

.flex-row-reverse {
  flex-direction: row-reverse;
}

.flex-col-reverse {
  flex-direction: column-reverse;
}

/* Flex 换行 */
.flex-wrap {
  flex-wrap: wrap;
}

.flex-nowrap {
  flex-wrap: nowrap;
}

/* 主轴对齐 */
.justify-start {
  justify-content: flex-start;
}

.justify-center {
  justify-content: center;
}

.justify-end {
  justify-content: flex-end;
}

.justify-between {
  justify-content: space-between;
}

.justify-around {
  justify-content: space-around;
}

.justify-evenly {
  justify-content: space-evenly;
}

/* 交叉轴对齐 */
.items-start {
  align-items: flex-start;
}

.items-center {
  align-items: center;
}

.items-end {
  align-items: flex-end;
}

.items-stretch {
  align-items: stretch;
}

.items-baseline {
  align-items: baseline;
}
```

### 2. Grid 布局

```css
/* Grid 容器 */
.grid {
  display: grid;
}

/* Grid 列数 */
.grid-cols-1 {
  grid-template-columns: repeat(1, minmax(0, 1fr));
}

.grid-cols-2 {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.grid-cols-3 {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.grid-cols-4 {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

/* Grid 间距 */
.gap-0 { gap: 0; }
.gap-xs { gap: var(--spacing-xs); }
.gap-sm { gap: var(--spacing-sm); }
.gap-md { gap: var(--spacing-md); }
.gap-lg { gap: var(--spacing-lg); }
.gap-xl { gap: var(--spacing-xl); }
```

### 3. 定位系统

```css
/* 定位 */
.static {
  position: static;
}

.relative {
  position: relative;
}

.absolute {
  position: absolute;
}

.fixed {
  position: fixed;
}

.sticky {
  position: sticky;
}

/* 定位偏移 */
.top-0 { top: 0; }
.right-0 { right: 0; }
.bottom-0 { bottom: 0; }
.left-0 { left: 0; }

.inset-0 {
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
}
```

## 组件样式

### 1. 卡片组件

```css
.card {
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  overflow: hidden;
  transition: box-shadow 0.2s ease;
}

.card:hover {
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.card-header {
  padding: var(--spacing-lg);
  border-bottom: 1px solid #eee;
  background: #fafafa;
}

.card-body {
  padding: var(--spacing-lg);
}

.card-footer {
  padding: var(--spacing-md) var(--spacing-lg);
  border-top: 1px solid #eee;
  background: #fafafa;
}
```

### 2. 按钮组件

```css
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-sm) var(--spacing-md);
  border: none;
  border-radius: 6px;
  font-size: 0.875rem;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s ease;
  min-height: 36px;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* 按钮变体 */
.btn-primary {
  background-color: var(--color-primary);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--color-primary-hover);
}

.btn-secondary {
  background-color: transparent;
  color: var(--color-primary);
  border: 1px solid var(--color-primary);
}

.btn-secondary:hover:not(:disabled) {
  background-color: var(--color-primary-light);
}

/* 按钮尺寸 */
.btn-sm {
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: 0.75rem;
  min-height: 28px;
}

.btn-lg {
  padding: var(--spacing-md) var(--spacing-lg);
  font-size: 1rem;
  min-height: 44px;
}
```

### 3. 状态指示器

```css
.status-indicator {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border-radius: 16px;
  font-size: 0.875rem;
  font-weight: 500;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: currentColor;
}

/* 状态变体 */
.status-running {
  background-color: #e8f5e8;
  color: var(--color-success);
}

.status-stopped {
  background-color: #ffebee;
  color: var(--color-error);
}

.status-warning {
  background-color: #fff3e0;
  color: var(--color-warning);
}

.status-maintenance {
  background-color: #e3f2fd;
  color: var(--color-info);
}
```

## 实用工具类

### 1. 显示控制

```css
.block {
  display: block;
}

.inline-block {
  display: inline-block;
}

.inline {
  display: inline;
}

.hidden {
  display: none;
}

.invisible {
  visibility: hidden;
}

.visible {
  visibility: visible;
}
```

### 2. 溢出控制

```css
.overflow-auto {
  overflow: auto;
}

.overflow-hidden {
  overflow: hidden;
}

.overflow-scroll {
  overflow: scroll;
}

.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

### 3. 边框样式

```css
/* 边框宽度 */
.border {
  border-width: 1px;
  border-style: solid;
  border-color: #e5e5e5;
}

.border-0 {
  border-width: 0;
}

.border-t {
  border-top-width: 1px;
  border-top-style: solid;
  border-top-color: #e5e5e5;
}

.border-b {
  border-bottom-width: 1px;
  border-bottom-style: solid;
  border-bottom-color: #eee;
}

/* 边框圆角 */
.rounded {
  border-radius: 4px;
}

.rounded-md {
  border-radius: 6px;
}

.rounded-lg {
  border-radius: 8px;
}

.rounded-full {
  border-radius: 9999px;
}
```

## 响应式设计

### 1. 断点系统

```css
/* 断点定义 */
:root {
  --breakpoint-sm: 640px;
  --breakpoint-md: 768px;
  --breakpoint-lg: 1024px;
  --breakpoint-xl: 1280px;
}
```

### 2. 响应式类示例

```css
/* 响应式隐藏 */
@media (max-width: 639px) {
  .sm\:hidden {
    display: none;
  }
}

@media (min-width: 768px) {
  .md\:block {
    display: block;
  }

  .md\:flex {
    display: flex;
  }

  .md\:text-lg {
    font-size: 1.125rem;
  }
}

@media (min-width: 1024px) {
  .lg\:grid-cols-3 {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
```

## 动画系统

### 1. 过渡动画

```css
/* 过渡时间 */
.transition {
  transition-property: color, background-color, border-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform, filter, backdrop-filter;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
}

.transition-all {
  transition-property: all;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
}

.transition-colors {
  transition-property: color, background-color, border-color;
  transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
  transition-duration: 150ms;
}
```

### 2. 变换动画

```css
/* 悬停效果 */
.hover\:scale-105:hover {
  transform: scale(1.05);
}

.hover\:shadow-lg:hover {
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.15);
}

.hover\:bg-gray-50:hover {
  background-color: #f9fafb;
}
```

## 使用示例

### 1. 组合使用示例

```rust
// Dioxus 组件中使用设计系统
rsx! {
    div {
        class: "card shadow-lg rounded-lg overflow-hidden",

        div {
            class: "card-header bg-primary-light",
            h2 {
                class: "text-xl font-semibold text-primary m-0",
                "API 端点管理"
            }
        }

        div {
            class: "card-body p-lg",
            div {
                class: "flex items-center justify-between p-md border-b",
                div {
                    class: "flex flex-col gap-xs",
                    span {
                        class: "font-medium text-base",
                        "/v1/chat/completions"
                    }
                    span {
                        class: "text-sm text-secondary",
                        "对话完成接口"
                    }
                }
                span {
                    class: "status-indicator status-running",
                    span { class: "status-dot" }
                    "正常运行"
                }
            }
        }
    }
}
```

### 2. 主题定制示例

```css
/* 暗色主题 */
[data-theme="dark"] {
  --color-primary: #60a5fa;
  --color-secondary: #94a3b8;
  --color-bg-primary: #0f172a;
  --color-bg-secondary: #1e293b;
}

[data-theme="dark"] .card {
  background-color: var(--color-bg-secondary);
  color: #f1f5f9;
}
```

---

*本文档详细描述了 BurnCloud Client API 的完整 CSS 设计系统，为界面开发提供了标准化的样式指导。*