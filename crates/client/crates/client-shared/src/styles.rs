/// BurnCloud Design System CSS — Apple-inspired visual language.
///
/// Single source of truth for all design tokens (`--bc-*` CSS variables),
/// base element styles, component styles, and utility classes.
/// Loaded via `<style>{DESIGN_SYSTEM_CSS}</style>` in both guest and authenticated layouts.
pub const DESIGN_SYSTEM_CSS: &str = r#"
/* ═══════════════════════════════════════════════════════════════════
   BurnCloud Design System — Apple-inspired
   ═══════════════════════════════════════════════════════════════════ */

:root {
    /* ── Brand Colors ── */
    --bc-primary: #007AFF;
    --bc-primary-hover: #0077ED;
    --bc-primary-active: #0066D6;
    --bc-primary-light: rgba(0, 122, 255, 0.10);
    --bc-primary-dark: #5856D6;
    --bc-primary-dark-hover: #6E6AE8;

    /* ── Semantic Colors ── */
    --bc-success: #34C759;
    --bc-success-light: rgba(52, 199, 89, 0.10);
    --bc-warning: #FF9500;
    --bc-warning-light: rgba(255, 149, 0, 0.10);
    --bc-danger: #FF3B30;
    --bc-danger-light: rgba(255, 59, 48, 0.10);
    --bc-info: #5AC8FA;
    --bc-info-light: rgba(90, 200, 250, 0.10);

    /* ── Neutral Colors ── */
    --bc-bg-canvas: #F5F5F7;
    --bc-bg-card: rgba(255, 255, 255, 0.85);
    --bc-bg-card-solid: #FFFFFF;
    --bc-bg-elevated: #FFFFFF;
    --bc-bg-hover: rgba(0, 0, 0, 0.04);
    --bc-bg-selected: rgba(0, 0, 0, 0.08);
    --bc-bg-input: rgba(255, 255, 255, 0.90);

    /* ── Text Colors ── */
    --bc-text-primary: #1D1D1F;
    --bc-text-secondary: #86868B;
    --bc-text-tertiary: #AEAEB2;
    --bc-text-on-accent: #FFFFFF;
    --bc-text-disabled: #C7C7CC;

    /* ── Border Colors ── */
    --bc-border: rgba(0, 0, 0, 0.08);
    --bc-border-hover: rgba(0, 0, 0, 0.15);
    --bc-border-focus: rgba(0, 122, 255, 0.50);

    /* ── Radius Scale ── */
    --bc-radius-xs: 4px;
    --bc-radius-sm: 8px;
    --bc-radius-md: 12px;
    --bc-radius-lg: 16px;
    --bc-radius-xl: 24px;
    --bc-radius-2xl: 32px;
    --bc-radius-full: 9999px;

    /* ── Shadow Scale ── */
    --bc-shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.05);
    --bc-shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.08);
    --bc-shadow-md: 0 8px 24px rgba(0, 0, 0, 0.12);
    --bc-shadow-lg: 0 16px 48px rgba(0, 0, 0, 0.16);
    --bc-shadow-xl: 0 24px 64px rgba(0, 0, 0, 0.20);
    --bc-shadow-primary: 0 10px 30px -5px rgba(0, 122, 255, 0.35);

    /* ── Typography Scale ── */
    --bc-font-xs: 10px;
    --bc-font-sm: 12px;
    --bc-font-base: 14px;
    --bc-font-md: 15px;
    --bc-font-lg: 17px;
    --bc-font-xl: 20px;
    --bc-font-2xl: 28px;
    --bc-font-3xl: 40px;

    /* ── Spacing Scale ── */
    --bc-space-1: 4px;
    --bc-space-2: 8px;
    --bc-space-3: 12px;
    --bc-space-4: 16px;
    --bc-space-5: 20px;
    --bc-space-6: 24px;
    --bc-space-8: 32px;
    --bc-space-10: 40px;
    --bc-space-12: 48px;

    /* ── Animation ── */
    --bc-transition-fast: 150ms cubic-bezier(0.25, 0.1, 0.25, 1);
    --bc-transition-normal: 250ms cubic-bezier(0.25, 0.1, 0.25, 1);
    --bc-transition-slow: 350ms cubic-bezier(0.25, 0.1, 0.25, 1);
    --bc-transition-spring: 500ms cubic-bezier(0.34, 1.56, 0.64, 1);

    /* ── Font Family ── */
    --bc-font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji';

    /* ── Legacy Aliases (for gradual migration) ── */
    --accent-color: var(--bc-primary);
    --accent-light1: #4DA3FF;
    --accent-dark1: #0062CC;
    --text-primary: var(--bc-text-primary);
    --text-secondary: var(--bc-text-secondary);
    --text-tertiary: var(--bc-text-tertiary);
    --text-disabled: var(--bc-text-disabled);
    --text-on-accent: var(--bc-text-on-accent);
    --bg-canvas: var(--bc-bg-canvas);
    --bg-card: var(--bc-bg-card-solid);
    --bg-card-hover: var(--bc-bg-hover);
    --bg-card-selected: var(--bc-bg-selected);
    --neutral-quaternary: var(--bc-border);
    --neutral-quinary: var(--bc-border-hover);
    --neutral-secondary: #F3F2F1;
    --radius-large: var(--bc-radius-sm);
    --radius-medium: var(--bc-radius-md);
    --radius-small: var(--bc-radius-xs);
    --radius-xlarge: var(--bc-radius-md);
    --spacing-xs: var(--bc-space-1);
    --spacing-sm: var(--bc-space-2);
    --spacing-md: var(--bc-space-3);
    --spacing-lg: var(--bc-space-4);
    --spacing-xl: var(--bc-space-5);
    --spacing-xxl: var(--bc-space-6);
    --spacing-xxxl: var(--bc-space-8);
    --font-size-caption: var(--bc-font-sm);
    --font-size-body: var(--bc-font-base);
    --font-size-subtitle: var(--bc-font-lg);
    --font-size-title: var(--bc-font-xl);
    --font-size-large-title: var(--bc-font-2xl);
    --font-size-display: var(--bc-font-3xl);
    --animation-fast: 150ms;
    --animation-normal: 250ms;
    --animation-slow: 350ms;
    --animation-curve: cubic-bezier(0.25, 0.1, 0.25, 1);
    --font-family: var(--bc-font-family);
}

/* ═══════════════════════════════════════════════════════════════════
   Base Element Styles
   ═══════════════════════════════════════════════════════════════════ */

* { box-sizing: border-box; }

html, body {
    margin: 0;
    padding: 0;
    font-family: var(--bc-font-family);
    font-size: var(--bc-font-base);
    color: var(--bc-text-primary);
    background-color: var(--bc-bg-canvas);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    overscroll-behavior: none;
    -ms-scroll-chaining: none;
}

/* ═══════════════════════════════════════════════════════════════════
   Acrylic / Glass Effects
   ═══════════════════════════════════════════════════════════════════ */

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

/* ═══════════════════════════════════════════════════════════════════
   Card Styles
   ═══════════════════════════════════════════════════════════════════ */

.card {
    background: var(--bc-bg-card-solid);
    border-radius: var(--bc-radius-sm);
    box-shadow: var(--bc-shadow-sm);
    border: 1px solid var(--bc-border);
    transition: all var(--bc-transition-normal);
}

.card:hover {
    background: var(--bc-bg-card-solid);
    box-shadow: var(--bc-shadow-md);
}

.card-interactive { cursor: pointer; }
.card-interactive:hover { transform: translateY(-1px); }
.card-interactive:active { transform: translateY(0); }

/* BC Card Variants */
.bc-card-solid {
    background: var(--bc-bg-card-solid);
    border: 1px solid var(--bc-border);
    border-radius: var(--bc-radius-md);
    box-shadow: var(--bc-shadow-sm);
}

.bc-card-glass {
    background: var(--bc-bg-card);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.3);
    border-radius: var(--bc-radius-2xl);
    box-shadow: var(--bc-shadow-sm);
}

.bc-card-outlined {
    background: transparent;
    border: 1px solid var(--bc-border);
    border-radius: var(--bc-radius-md);
}

/* ═══════════════════════════════════════════════════════════════════
   Button Styles
   ═══════════════════════════════════════════════════════════════════ */

.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--bc-space-2);
    padding: var(--bc-space-2) var(--bc-space-4);
    border: 1px solid transparent;
    border-radius: var(--bc-radius-md);
    font-family: var(--bc-font-family);
    font-size: var(--bc-font-base);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--bc-transition-fast);
    text-decoration: none;
    white-space: nowrap;
    min-height: 32px;
}

.btn:focus-visible {
    outline: 2px solid var(--bc-primary);
    outline-offset: 2px;
}

.btn-primary {
    background: var(--bc-primary);
    color: var(--bc-text-on-accent);
}
.btn-primary:hover { background: var(--bc-primary-hover); }
.btn-primary:active { background: var(--bc-primary-active); }

.btn-secondary {
    background: transparent;
    color: var(--bc-text-primary);
    border-color: var(--bc-border);
}
.btn-secondary:hover { background: var(--bc-bg-hover); border-color: var(--bc-border-hover); }
.btn-secondary:active { background: var(--bc-bg-selected); }

.btn-danger {
    background: var(--bc-danger);
    color: var(--bc-text-on-accent);
}
.btn-danger:hover { background: #E6332A; }

.btn-subtle {
    background: transparent;
    color: var(--bc-text-secondary);
}
.btn-subtle:hover { background: var(--bc-bg-hover); color: var(--bc-text-primary); }
.btn-subtle:active { background: var(--bc-bg-selected); }

.btn-ghost {
    background: transparent;
    color: var(--bc-text-secondary);
    border-color: transparent;
}
.btn-ghost:hover { background: var(--bc-bg-hover); color: var(--bc-text-primary); }

/* BC Button Gradient Variant */
.bc-btn-gradient {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: var(--bc-space-4) var(--bc-space-6);
    font-size: var(--bc-font-lg);
    font-weight: 600;
    color: var(--bc-text-on-accent);
    background: linear-gradient(to right, var(--bc-primary), var(--bc-primary-dark));
    border: none;
    border-radius: var(--bc-radius-md);
    box-shadow: var(--bc-shadow-primary);
    cursor: pointer;
    transition: all 250ms cubic-bezier(0.25, 0.1, 0.25, 1);
}

.bc-btn-gradient:hover {
    background: linear-gradient(to right, var(--bc-primary-hover), var(--bc-primary-dark-hover));
    box-shadow: 0 20px 40px -5px rgba(0, 122, 255, 0.50);
    transform: translateY(-1px);
}

.bc-btn-gradient:active {
    transform: scale(0.98);
    box-shadow: var(--bc-shadow-primary);
}

.bc-btn-gradient:disabled,
.bc-btn-gradient.disabled {
    opacity: 0.75;
    cursor: not-allowed;
    transform: none;
}

/* ═══════════════════════════════════════════════════════════════════
   Input Styles
   ═══════════════════════════════════════════════════════════════════ */

.input {
    display: block;
    width: 100%;
    padding: var(--bc-space-2) var(--bc-space-3);
    border: 1px solid var(--bc-border);
    border-radius: var(--bc-radius-md);
    font-family: var(--bc-font-family);
    font-size: var(--bc-font-base);
    color: var(--bc-text-primary);
    background: var(--bc-bg-card-solid);
    transition: all var(--bc-transition-fast);
}

.input:hover { border-color: var(--bc-border-hover); }

.input:focus {
    outline: none;
    border-color: var(--bc-primary);
    box-shadow: 0 0 0 1px var(--bc-primary);
}

.input:disabled {
    color: var(--bc-text-disabled);
    background: var(--bc-bg-hover);
    cursor: not-allowed;
}

/* BCInput - Modern Input with Physics */
.bc-input-group { position: relative; }

.bc-input {
    display: block;
    width: 100%;
    font-family: var(--bc-font-family);
    background: var(--bc-bg-input);
    border: 1px solid transparent;
    border-radius: var(--bc-radius-md);
    transition: all 200ms cubic-bezier(0.33, 0, 0.67, 1);
    transform-origin: center;
}

.bc-input:hover { background: rgba(255, 255, 255, 0.95); }

.bc-input:focus {
    outline: none;
    background: var(--bc-bg-card-solid);
    border-color: var(--bc-primary);
    box-shadow: 0 0 0 2px var(--bc-border-focus);
    transform: scale(1.02);
}

.bc-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
}

/* BC Badge Variants */
.bc-badge-success { background: var(--bc-success-light); color: var(--bc-success); }
.bc-badge-warning { background: var(--bc-warning-light); color: var(--bc-warning); }
.bc-badge-danger { background: var(--bc-danger-light); color: var(--bc-danger); }
.bc-badge-info { background: var(--bc-info-light); color: var(--bc-info); }
.bc-badge-neutral { background: var(--bc-bg-hover); color: var(--bc-text-secondary); }

/* ═══════════════════════════════════════════════════════════════════
   Progress Bar
   ═══════════════════════════════════════════════════════════════════ */

.progress {
    width: 100%;
    height: 4px;
    background: var(--bc-border);
    border-radius: var(--bc-radius-xs);
    overflow: hidden;
}

.progress-fill {
    height: 100%;
    background: var(--bc-primary);
    border-radius: var(--bc-radius-xs);
    transition: width var(--bc-transition-normal);
}

/* ═══════════════════════════════════════════════════════════════════
   Status Indicators
   ═══════════════════════════════════════════════════════════════════ */

.status-indicator {
    display: inline-flex;
    align-items: center;
    gap: var(--bc-space-1);
    font-size: var(--bc-font-sm);
    font-weight: 600;
}

.status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
}

.status-running .status-dot {
    background: var(--bc-success);
    animation: pulse 2s infinite;
}

.status-stopped .status-dot { background: var(--bc-danger); }
.status-pending .status-dot { background: var(--bc-warning); }

/* ═══════════════════════════════════════════════════════════════════
   Animations
   ═══════════════════════════════════════════════════════════════════ */

@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}

@keyframes shake {
    0%, 100% { transform: translateX(0); }
    10%, 30%, 50%, 70%, 90% { transform: translateX(-10px); }
    20%, 40%, 60%, 80% { transform: translateX(10px); }
}

@keyframes aurora {
    0%, 100% { transform: translate(0, 0) rotate(0deg) scale(1); }
    25% { transform: translate(30px, -50px) rotate(90deg) scale(1.1); }
    50% { transform: translate(-20px, 20px) rotate(180deg) scale(0.9); }
    75% { transform: translate(20px, 40px) rotate(270deg) scale(1.05); }
}

@keyframes float {
    0%, 100% { transform: translateY(0px); }
    50% { transform: translateY(-20px); }
}

@keyframes glow-pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
}

@keyframes shimmer {
    0% { background-position: -200% 0; }
    100% { background-position: 200% 0; }
}

@keyframes morph {
    0%, 100% { border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%; }
    50% { border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%; }
}

@keyframes slide-up-fade {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
}

@keyframes scale-in {
    from { opacity: 0; transform: scale(0.95); }
    to { opacity: 1; transform: scale(1); }
}

@keyframes animate-in {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
}

@keyframes orbit {
    from { transform: rotate(0deg) translateX(100px) rotate(0deg); }
    to { transform: rotate(360deg) translateX(100px) rotate(-360deg); }
}

@keyframes ripple {
    to { transform: scale(4); opacity: 0; }
}

@keyframes gradient-flow {
    0% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
    100% { background-position: 0% 50%; }
}

@keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
}

@keyframes pulse-soft {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 1; }
}

.animate-shake { animation: shake 0.5s cubic-bezier(0.36, 0.07, 0.19, 0.97) both; }
.animate-aurora { animation: aurora 30s ease-in-out infinite; }
.animate-float { animation: float 6s ease-in-out infinite; }
.animate-glow-pulse { animation: glow-pulse 3s ease-in-out infinite; }
.animate-shimmer { animation: shimmer 2s linear infinite; }
.animate-morph { animation: morph 8s ease-in-out infinite; }
.animate-slide-up { animation: slide-up-fade 0.5s ease-out forwards; }
.animate-scale-in { animation: scale-in 0.3s ease-out forwards; }
.animate-in { animation: animate-in 0.6s ease-out forwards; }
.animate-orbit { animation: orbit 20s linear infinite; }
.animate-ripple { animation: ripple 0.6s ease-out; }
.animate-gradient-flow { animation: gradient-flow 3s ease infinite; }
.animate-blink { animation: blink 1s step-end infinite; }
.animate-pulse-soft { animation: pulse-soft 2s ease-in-out infinite; }

/* ═══════════════════════════════════════════════════════════════════
   Layout Helpers
   ═══════════════════════════════════════════════════════════════════ */

.flex { display: flex; }
.flex-col { flex-direction: column; }
.flex-1 { flex: 1; }
.w-full { width: 100%; }
.h-full { height: 100%; }
.grid { display: grid; }
.overflow-hidden { overflow: hidden; }
.overflow-x-auto { overflow-x: auto; }
.overflow-y-auto { overflow-y: auto; }

.border-t { border-top: 1px solid var(--bc-border); }
.border-b { border-bottom: 1px solid var(--bc-border); }

.text-left { text-align: left; }
.text-center { text-align: center; }
.text-right { text-align: right; }
.cursor-pointer { cursor: pointer; }
.min-height-200 { min-height: 200px; }

.items-center { align-items: center; }
.items-start { align-items: flex-start; }
.justify-between { justify-content: space-between; }
.justify-center { justify-content: center; }

/* Spacing Utilities */
.gap-xs { gap: var(--bc-space-1); }
.gap-sm { gap: var(--bc-space-2); }
.gap-md { gap: var(--bc-space-3); }
.gap-lg { gap: var(--bc-space-4); }
.gap-xl { gap: var(--bc-space-5); }

.p-xs { padding: var(--bc-space-1); }
.p-sm { padding: var(--bc-space-2); }
.p-md { padding: var(--bc-space-3); }
.p-lg { padding: var(--bc-space-4); }
.p-xl { padding: var(--bc-space-5); }
.p-xxl { padding: var(--bc-space-6); }
.p-xxxl { padding: var(--bc-space-8); }

.m-xs { margin: var(--bc-space-1); }
.m-sm { margin: var(--bc-space-2); }
.m-md { margin: var(--bc-space-3); }
.m-lg { margin: var(--bc-space-4); }
.m-xl { margin: var(--bc-space-5); }
.m-0 { margin: 0; }

.mb-xs { margin-bottom: var(--bc-space-1); }
.mb-sm { margin-bottom: var(--bc-space-2); }
.mb-md { margin-bottom: var(--bc-space-3); }
.mb-lg { margin-bottom: var(--bc-space-4); }
.mb-xl { margin-bottom: var(--bc-space-5); }
.mb-xxl { margin-bottom: var(--bc-space-6); }
.mb-xxxl { margin-bottom: var(--bc-space-8); }

.mt-xs { margin-top: var(--bc-space-1); }
.mt-sm { margin-top: var(--bc-space-2); }
.mt-md { margin-top: var(--bc-space-3); }
.mt-lg { margin-top: var(--bc-space-4); }
.mt-xl { margin-top: var(--bc-space-5); }
.mt-xxl { margin-top: var(--bc-space-6); }
.mt-xxxl { margin-top: var(--bc-space-8); }
.mt-auto { margin-top: auto; }

/* ═══════════════════════════════════════════════════════════════════
   Typography
   ═══════════════════════════════════════════════════════════════════ */

.text-caption { font-size: var(--bc-font-sm); }
.text-body { font-size: var(--bc-font-base); }
.text-subtitle { font-size: var(--bc-font-lg); }
.text-title { font-size: var(--bc-font-xl); }
.text-large-title { font-size: var(--bc-font-2xl); }
.text-display { font-size: var(--bc-font-3xl); }

.text-xxl { font-size: 32px; }
.text-xxxl { font-size: 48px; }
.text-xxs { font-size: 10px; }
.text-xxxs { font-size: 8px; }

.text-primary { color: var(--bc-text-primary); }
.text-secondary { color: var(--bc-text-secondary); }
.text-tertiary { color: var(--bc-text-tertiary); }
.text-disabled { color: var(--bc-text-disabled); }

.font-normal { font-weight: 400; }
.font-medium { font-weight: 500; }
.font-semibold { font-weight: 600; }
.font-bold { font-weight: 700; }

/* ═══════════════════════════════════════════════════════════════════
   App Layout
   ═══════════════════════════════════════════════════════════════════ */

.app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bc-bg-canvas);
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
    background: var(--bc-bg-card-solid);
    border-right: 1px solid var(--bc-border);
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
    background: var(--bc-bg-card-solid);
    border-bottom: 1px solid var(--bc-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--bc-space-4);
    flex-shrink: 0;
    -webkit-app-region: drag;
}

.title-bar button { -webkit-app-region: no-drag; }

.page-header {
    padding: var(--bc-space-5) var(--bc-space-6);
    background: var(--bc-bg-card-solid);
    border-bottom: 1px solid var(--bc-border);
    flex-shrink: 0;
}

.page-content {
    flex: 1;
    padding: var(--bc-space-6);
    overflow-y: auto;
}

/* ═══════════════════════════════════════════════════════════════════
   Navigation
   ═══════════════════════════════════════════════════════════════════ */

.nav-item {
    display: flex;
    align-items: center;
    gap: var(--bc-space-3);
    padding: var(--bc-space-3) var(--bc-space-4);
    color: var(--bc-text-secondary);
    text-decoration: none;
    border-radius: var(--bc-radius-md);
    margin: 0 var(--bc-space-2);
    transition: all var(--bc-transition-fast);
    user-select: none;
}

.nav-item:hover {
    background: var(--bc-bg-hover);
    color: var(--bc-text-primary);
    transform: translateX(4px);
}

.nav-item:active {
    background: var(--bc-bg-selected);
    transform: translateX(2px) scale(0.98);
}

.nav-item.active {
    background: var(--bc-primary);
    color: var(--bc-text-on-accent);
    box-shadow: var(--bc-shadow-sm);
}

.nav-item .icon {
    font-size: 16px;
    width: 16px;
    text-align: center;
}

/* ═══════════════════════════════════════════════════════════════════
   Model Cards & Metrics
   ═══════════════════════════════════════════════════════════════════ */

.model-card { padding: var(--bc-space-4); }

.model-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--bc-space-3);
}

.model-title {
    display: flex;
    align-items: center;
    gap: var(--bc-space-3);
    font-size: var(--bc-font-lg);
    font-weight: 600;
}

.model-actions {
    display: flex;
    gap: var(--bc-space-2);
}

.model-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: var(--bc-space-3);
    color: var(--bc-text-secondary);
    font-size: var(--bc-font-sm);
}

.metric-card {
    padding: var(--bc-space-4);
    background: var(--bc-bg-card-solid);
    border-radius: var(--bc-radius-sm);
    border: 1px solid var(--bc-border);
}

.metric-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--bc-space-3);
}

.metric-value {
    font-size: var(--bc-font-xl);
    font-weight: 600;
    color: var(--bc-text-primary);
}

.metric-label {
    font-size: var(--bc-font-sm);
    color: var(--bc-text-secondary);
}

/* ═══════════════════════════════════════════════════════════════════
   Log Viewer
   ═══════════════════════════════════════════════════════════════════ */

.log-viewer {
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: 'Cascadia Code', 'Fira Code', 'Monaco', 'Consolas', monospace;
    padding: var(--bc-space-4);
    border-radius: var(--bc-radius-md);
    height: 300px;
    overflow-y: auto;
    font-size: 13px;
    line-height: 1.4;
}

.log-entry { margin-bottom: 2px; }
.log-timestamp { color: #808080; }
.log-level-info { color: #4fc1ff; }
.log-level-warn { color: #ffcc02; }
.log-level-error { color: #f85149; }
.log-level-debug { color: #a5a5a5; }

/* ═══════════════════════════════════════════════════════════════════
   macOS-style Scrollbars (hidden by default, show on hover)
   ═══════════════════════════════════════════════════════════════════ */

*::-webkit-scrollbar {
    width: 0px !important;
    height: 0px !important;
    background: transparent !important;
    display: none !important;
}

*::-webkit-scrollbar-track {
    background: transparent !important;
    border: none !important;
    display: none !important;
}

*::-webkit-scrollbar-thumb {
    background: transparent !important;
    border-radius: 3px !important;
    border: none !important;
    display: none !important;
}

*::-webkit-scrollbar-corner {
    background: transparent !important;
    display: none !important;
}

/* Show scrollbars on hover for scrollable containers */
html .overflow-y-auto:hover::-webkit-scrollbar,
html .overflow-x-auto:hover::-webkit-scrollbar,
html .overflow-y-scroll:hover::-webkit-scrollbar,
html .overflow-x-scroll:hover::-webkit-scrollbar {
    width: 6px !important;
    height: 6px !important;
    display: block !important;
}

html .overflow-y-auto:hover::-webkit-scrollbar-track,
html .overflow-x-auto:hover::-webkit-scrollbar-track,
html .overflow-y-scroll:hover::-webkit-scrollbar-track,
html .overflow-x-scroll:hover::-webkit-scrollbar-track {
    background: transparent !important;
    display: block !important;
}

html .overflow-y-auto:hover::-webkit-scrollbar-thumb,
html .overflow-x-auto:hover::-webkit-scrollbar-thumb,
html .overflow-y-scroll:hover::-webkit-scrollbar-thumb,
html .overflow-x-scroll:hover::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.2) !important;
    display: block !important;
    transition: background 0.2s ease !important;
}

html .overflow-y-auto:hover::-webkit-scrollbar-thumb:hover,
html .overflow-x-auto:hover::-webkit-scrollbar-thumb:hover,
html .overflow-y-scroll:hover::-webkit-scrollbar-thumb:hover,
html .overflow-x-scroll:hover::-webkit-scrollbar-thumb:hover {
    background: rgba(0, 0, 0, 0.4) !important;
}

/* Show scrollbar on any element hover */
*:hover::-webkit-scrollbar {
    width: 6px !important;
    height: 6px !important;
    display: block !important;
}

*:hover::-webkit-scrollbar-track {
    background: transparent !important;
    display: block !important;
}

*:hover::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.15) !important;
    display: block !important;
    transition: background 0.2s ease !important;
}

*:hover::-webkit-scrollbar-thumb:hover {
    background: rgba(0, 0, 0, 0.3) !important;
}

/* ═══════════════════════════════════════════════════════════════════
   macOS Window Control Colors
   ═══════════════════════════════════════════════════════════════════ */

.bg-macos-red { background-color: #FF5F56; }
.border-macos-red { border-color: #E0443E; }
.text-macos-red-dark { color: #4E0002; }

.bg-macos-yellow { background-color: #FFBD2E; }
.border-macos-yellow { border-color: #E1A325; }
.text-macos-yellow-dark { color: #594119; }

.bg-macos-green { background-color: #27C93F; }
.border-macos-green { border-color: #1FA22E; }
.text-macos-green-dark { color: #0A5016; }

.shadow-glow-green { box-shadow: 0 0 8px rgba(34,197,94,0.6); }

/* Desktop window regions */
.app-drag-region { -webkit-app-region: drag; }
.app-no-drag { -webkit-app-region: no-drag; }

/* ═══════════════════════════════════════════════════════════════════
   JIT Shims (Tailwind v2 static compatibility)
   ═══════════════════════════════════════════════════════════════════ */

:root {
    --scrollbar-width: 0px !important;
}

/* Dark mode support */
@media (prefers-color-scheme: dark) {
    :root {
        --bc-bg-canvas: #1D1D1F;
        --bc-bg-card: rgba(44, 44, 46, 0.85);
        --bc-bg-card-solid: #2C2C2E;
        --bc-bg-elevated: #3A3A3C;
        --bc-bg-hover: rgba(255, 255, 255, 0.06);
        --bc-bg-selected: rgba(255, 255, 255, 0.10);
        --bc-bg-input: rgba(44, 44, 46, 0.90);

        --bc-text-primary: #FFFFFF;
        --bc-text-secondary: #AEAEB2;
        --bc-text-tertiary: #8E8E93;
        --bc-text-disabled: #636366;

        --bc-border: rgba(255, 255, 255, 0.10);
        --bc-border-hover: rgba(255, 255, 255, 0.18);
        --bc-border-focus: rgba(0, 122, 255, 0.60);

        --bc-shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.20);
        --bc-shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.25);
        --bc-shadow-md: 0 8px 24px rgba(0, 0, 0, 0.30);
        --bc-shadow-lg: 0 16px 48px rgba(0, 0, 0, 0.35);
        --bc-shadow-xl: 0 24px 64px rgba(0, 0, 0, 0.40);
    }

    .bc-input { color: #ffffff; }
    .bc-input:hover { background: rgba(255, 255, 255, 0.08); }
    .bc-input:focus {
        background: rgba(255, 255, 255, 0.12);
        box-shadow: 0 0 0 4px rgba(0, 122, 255, 0.30);
    }
    .bc-input::placeholder { color: rgba(255, 255, 255, 0.4); }
}
"#;
