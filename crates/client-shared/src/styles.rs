pub const FLUENT_CSS: &str = r#"
/* Fluent Design System Variables */
:root {
    /* Colors */
    --accent-color: #0078d4;
    --accent-light1: #106ebe;
    --accent-light2: #2b88d8;
    --accent-light3: #71afe5;
    --accent-dark1: #005a9e;
    --accent-dark2: #004578;
    --accent-dark3: #002f52;

    /* Neutrals */
    --neutral-primary: #ffffff;
    --neutral-secondary: #f3f2f1;
    --neutral-tertiary: #edebe9;
    --neutral-quaternary: #e1dfdd;
    --neutral-quaternary-alt: #d2d0ce;
    --neutral-quinary: #c8c6c4;
    --neutral-senary: #a19f9d;

    /* Text colors */
    --text-primary: #323130;
    --text-secondary: #605e5c;
    --text-tertiary: #a19f9d;
    --text-disabled: #c8c6c4;
    --text-on-accent: #ffffff;

    /* Background colors */
    --bg-canvas: #faf9f8;
    --bg-card: #ffffff;
    --bg-card-hover: #f3f2f1;
    --bg-card-selected: #edebe9;
    --bg-layer: rgba(255, 255, 255, 0.7);
    --bg-smoke: rgba(0, 0, 0, 0.4);

    /* Elevations */
    --elevation-card: 0 2px 4px rgba(0, 0, 0, 0.14), 0 0px 2px rgba(0, 0, 0, 0.12);
    --elevation-flyout: 0 8px 16px rgba(0, 0, 0, 0.14), 0 0px 2px rgba(0, 0, 0, 0.12);
    --elevation-dialog: 0 32px 64px rgba(0, 0, 0, 0.14), 0 0px 2px rgba(0, 0, 0, 0.12);

    /* Border radius */
    --radius-small: 2px;
    --radius-medium: 4px;
    --radius-large: 8px;
    --radius-xlarge: 12px;

    /* Spacing */
    --spacing-xs: 4px;
    --spacing-sm: 8px;
    --spacing-md: 12px;
    --spacing-lg: 16px;
    --spacing-xl: 20px;
    --spacing-xxl: 24px;
    --spacing-xxxl: 32px;

    /* Typography */
    --font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Segoe UI Variable', system-ui, ui-sans-serif, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji';
    --font-size-caption: 12px;
    --font-size-body: 14px;
    --font-size-subtitle: 16px;
    --font-size-title: 20px;
    --font-size-large-title: 28px;
    --font-size-display: 40px;

    /* Animation */
    --animation-fast: 100ms;
    --animation-normal: 200ms;
    --animation-slow: 300ms;
    --animation-curve: cubic-bezier(0.33, 0, 0.67, 1);
}

/* Base styles */
* {
    box-sizing: border-box;
}

html, body {
    margin: 0;
    padding: 0;
    font-family: var(--font-family);
    font-size: var(--font-size-body);
    color: var(--text-primary);
    background-color: var(--bg-canvas);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    overscroll-behavior: none;
    -ms-scroll-chaining: none;
}

/* Acrylic backdrop effect */
.acrylic {
    background: rgba(255, 255, 255, 0.8);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.2);
}

.acrylic-dark {
    background: rgba(32, 32, 32, 0.8);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.1);
}

/* Card styles */
.card {
    background: var(--bg-card);
    border-radius: var(--radius-large);
    box-shadow: var(--elevation-card);
    border: 1px solid var(--neutral-quaternary);
    transition: all var(--animation-normal) var(--animation-curve);
}

.card:hover {
    background: var(--bg-card-hover);
    box-shadow: var(--elevation-flyout);
}

.card-interactive {
    cursor: pointer;
}

.card-interactive:hover {
    transform: translateY(-1px);
}

.card-interactive:active {
    transform: translateY(0);
}

/* Button styles */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-lg);
    border: 1px solid transparent;
    border-radius: var(--radius-medium);
    font-family: var(--font-family);
    font-size: var(--font-size-body);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--animation-fast) var(--animation-curve);
    text-decoration: none;
    white-space: nowrap;
    min-height: 32px;
}

.btn:focus-visible {
    outline: 2px solid var(--accent-color);
    outline-offset: 2px;
}

.btn-primary {
    background: var(--accent-color);
    color: var(--text-on-accent);
}

.btn-primary:hover {
    background: var(--accent-dark1);
}

.btn-primary:active {
    background: var(--accent-dark2);
}

.btn-secondary {
    background: transparent;
    color: var(--text-primary);
    border-color: var(--neutral-quaternary);
}

.btn-secondary:hover {
    background: var(--bg-card-hover);
    border-color: var(--neutral-quinary);
}

.btn-secondary:active {
    background: var(--bg-card-selected);
}

.btn-subtle {
    background: transparent;
    color: var(--text-secondary);
}

.btn-subtle:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
}

.btn-subtle:active {
    background: var(--bg-card-selected);
}

/* Input styles */
.input {
    display: block;
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--neutral-quaternary);
    border-radius: var(--radius-medium);
    font-family: var(--font-family);
    font-size: var(--font-size-body);
    color: var(--text-primary);
    background: var(--bg-card);
    transition: all var(--animation-fast) var(--animation-curve);
}

.input:hover {
    border-color: var(--neutral-quinary);
}

.input:focus {
    outline: none;
    border-color: var(--accent-color);
    box-shadow: 0 0 0 1px var(--accent-color);
}

.input:disabled {
    color: var(--text-disabled);
    background: var(--neutral-secondary);
    cursor: not-allowed;
}

/* Progress bar styles */
.progress {
    width: 100%;
    height: 4px;
    background: var(--neutral-quaternary);
    border-radius: var(--radius-small);
    overflow: hidden;
}

.progress-fill {
    height: 100%;
    background: var(--accent-color);
    border-radius: var(--radius-small);
    transition: width var(--animation-normal) var(--animation-curve);
}

/* Status indicators */
.status-indicator {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-caption);
    font-weight: 600;
}

.status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
}

.status-running .status-dot {
    background: #107c10;
    animation: pulse 2s infinite;
}

.status-stopped .status-dot {
    background: #d13438;
}

.status-pending .status-dot {
    background: #ff8c00;
}

@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}

/* Layout helpers */
.flex {
    display: flex;
}

.flex-col {
    flex-direction: column;
}

.flex-1 {
    flex: 1;
}

.w-full {
    width: 100%;
}

.h-full {
    height: 100%;
}

.grid {
    display: grid;
}

.overflow-hidden {
    overflow: hidden;
}

.overflow-x-auto {
    overflow-x: auto;
}

.overflow-y-auto {
    overflow-y: auto;
}

.border-t {
    border-top: 1px solid var(--neutral-quaternary);
}

.border-b {
    border-bottom: 1px solid var(--neutral-quaternary);
}

.text-left {
    text-align: left;
}

.text-center {
    text-align: center;
}

.text-right {
    text-align: right;
}

.cursor-pointer {
    cursor: pointer;
}

.min-height-200 {
    min-height: 200px;
}

.items-center {
    align-items: center;
}

.justify-between {
    justify-content: space-between;
}

.justify-center {
    justify-content: center;
}

.gap-xs { gap: var(--spacing-xs); }
.gap-sm { gap: var(--spacing-sm); }
.gap-md { gap: var(--spacing-md); }
.gap-lg { gap: var(--spacing-lg); }
.gap-xl { gap: var(--spacing-xl); }

.p-xs { padding: var(--spacing-xs); }
.p-sm { padding: var(--spacing-sm); }
.p-md { padding: var(--spacing-md); }
.p-lg { padding: var(--spacing-lg); }
.p-xl { padding: var(--spacing-xl); }

.m-xs { margin: var(--spacing-xs); }
.m-sm { margin: var(--spacing-sm); }
.m-md { margin: var(--spacing-md); }
.m-lg { margin: var(--spacing-lg); }
.m-xl { margin: var(--spacing-xl); }
.m-0 { margin: 0; }

.mb-xs { margin-bottom: var(--spacing-xs); }
.mb-sm { margin-bottom: var(--spacing-sm); }
.mb-md { margin-bottom: var(--spacing-md); }
.mb-lg { margin-bottom: var(--spacing-lg); }
.mb-xl { margin-bottom: var(--spacing-xl); }
.mb-xxl { margin-bottom: var(--spacing-xxl); }
.mb-xxxl { margin-bottom: var(--spacing-xxxl); }

.mt-xs { margin-top: var(--spacing-xs); }
.mt-sm { margin-top: var(--spacing-sm); }
.mt-md { margin-top: var(--spacing-md); }
.mt-lg { margin-top: var(--spacing-lg); }
.mt-xl { margin-top: var(--spacing-xl); }
.mt-xxl { margin-top: var(--spacing-xxl); }
.mt-xxxl { margin-top: var(--spacing-xxxl); }
.mt-auto { margin-top: auto; }

/* Typography */
.text-caption { font-size: var(--font-size-caption); }
.text-body { font-size: var(--font-size-body); }
.text-subtitle { font-size: var(--font-size-subtitle); }
.text-title { font-size: var(--font-size-title); }
.text-large-title { font-size: var(--font-size-large-title); }
.text-display { font-size: var(--font-size-display); }

.text-xxxl { font-size: 48px; }

.text-primary { color: var(--text-primary); }
.text-secondary { color: var(--text-secondary); }
.text-tertiary { color: var(--text-tertiary); }
.text-disabled { color: var(--text-disabled); }

.font-normal { font-weight: 400; }
.font-medium { font-weight: 500; }
.font-semibold { font-weight: 600; }
.font-bold { font-weight: 700; }

/* App-specific styles */
.app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-canvas);
    overflow: hidden;
    overscroll-behavior: none;
}

.app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
}

.sidebar {
    width: 250px;
    background: var(--bg-card);
    border-right: 1px solid var(--neutral-quaternary);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
}

.main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.title-bar {
    height: 48px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--neutral-quaternary);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--spacing-lg);
    flex-shrink: 0;
    -webkit-app-region: drag;
}

.title-bar button {
    -webkit-app-region: no-drag;
}

.page-header {
    padding: var(--spacing-xl) var(--spacing-xxl);
    background: var(--bg-card);
    border-bottom: 1px solid var(--neutral-quaternary);
    flex-shrink: 0;
}

.page-content {
    flex: 1;
    padding: var(--spacing-xxl);
    overflow-y: auto;
}

.nav-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md) var(--spacing-lg);
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: var(--radius-medium);
    margin: 0 var(--spacing-sm);
    transition: all var(--animation-fast) var(--animation-curve);
    user-select: none;
}

.nav-item:hover {
    background: var(--bg-card-hover);
    color: var(--text-primary);
    transform: translateX(4px);
}

.nav-item:active {
    background: var(--bg-card-selected);
    transform: translateX(2px) scale(0.98);
}

.nav-item.active {
    background: var(--accent-color);
    color: var(--text-on-accent);
    box-shadow: var(--elevation-card);
}

.nav-item .icon {
    font-size: 16px;
    width: 16px;
    text-align: center;
}

/* Model card styles */
.model-card {
    padding: var(--spacing-lg);
}

.model-card-static:hover {
    background: var(--bg-card);
    box-shadow: var(--elevation-flyout);
    transform: translateY(-2px);
}

.model-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-md);
}

.model-title {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    font-size: var(--font-size-subtitle);
    font-weight: 600;
}

.model-actions {
    display: flex;
    gap: var(--spacing-sm);
}

.model-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: var(--spacing-md);
    color: var(--text-secondary);
    font-size: var(--font-size-caption);
}

/* System monitor styles */
.metric-card {
    padding: var(--spacing-lg);
    background: var(--bg-card);
    border-radius: var(--radius-large);
    border: 1px solid var(--neutral-quaternary);
}

.metric-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-md);
}

.metric-value {
    font-size: var(--font-size-title);
    font-weight: 600;
    color: var(--text-primary);
}

.metric-label {
    font-size: var(--font-size-caption);
    color: var(--text-secondary);
}

/* Log viewer styles */
.log-viewer {
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: 'Cascadia Code', 'Fira Code', 'Monaco', 'Consolas', monospace;
    padding: var(--spacing-lg);
    border-radius: var(--radius-medium);
    height: 300px;
    overflow-y: auto;
    font-size: 13px;
    line-height: 1.4;
}

.log-entry {
    margin-bottom: 2px;
}

.log-timestamp {
    color: #808080;
}

.log-level-info {
    color: #4fc1ff;
}

.log-level-warn {
    color: #ffcc02;
}

.log-level-error {
    color: #f85149;
}

.log-level-debug {
    color: #a5a5a5;
}

/* Scrollbar styles */
::-webkit-scrollbar {
    width: 8px;
    height: 8px;
}

::-webkit-scrollbar-track {
    background: var(--neutral-secondary);
}

::-webkit-scrollbar-thumb {
    background: var(--neutral-quinary);
    border-radius: var(--radius-medium);
}

::-webkit-scrollbar-thumb:hover {
    background: var(--neutral-senary);
}

/* Dark theme support */
@media (prefers-color-scheme: dark) {
    :root {
        --neutral-primary: #2d2d30;
        --neutral-secondary: #3c3c3c;
        --neutral-tertiary: #484848;
        --neutral-quaternary: #5a5a5a;
        --neutral-quaternary-alt: #6d6d6d;
        --neutral-quinary: #808080;
        --neutral-senary: #a6a6a6;

        --text-primary: #ffffff;
        --text-secondary: #cccccc;
        --text-tertiary: #a6a6a6;
        --text-disabled: #6d6d6d;

        --bg-canvas: #1e1e1e;
        --bg-card: #2d2d30;
        --bg-card-hover: #3c3c3c;
        --bg-card-selected: #484848;
        --bg-layer: rgba(45, 45, 48, 0.7);
        --bg-smoke: rgba(0, 0, 0, 0.6);
    }
}
"#;