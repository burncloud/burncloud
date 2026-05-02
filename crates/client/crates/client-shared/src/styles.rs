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

    /* ── Component Specific ── */
    --bc-btn-black-bg: #000000;
    --bc-btn-black-text: #FFFFFF;
    --bc-btn-black-hover: #1A1A1A;
    --bc-btn-black-active: #333333;

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

    /* ── Overlay Colors ── */
    --bc-overlay-bg: rgba(0, 0, 0, 0.30);
    --bc-overlay-bg-heavy: rgba(0, 0, 0, 0.60);

    /* ── Code / Log Colors ── */
    --bc-bg-code: #1e1e1e;
    --bc-text-code: #d4d4d4;
    --bc-log-time: #808080;
    --bc-log-info: #4fc1ff;
    --bc-log-warn: #dcdcaa;
    --bc-log-err: #f48771;

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

.btn-black {
    background: var(--bc-btn-black-bg);
    color: var(--bc-btn-black-text);
}
.btn-black:hover { background: var(--bc-btn-black-hover); }
.btn-black:active { background: var(--bc-btn-black-active); }

.btn-social {
    background: var(--bc-bg-canvas);
    color: var(--bc-text-primary);
    border-color: transparent;
}
.btn-social:hover { background: var(--bc-bg-hover); }
.btn-social:active { background: var(--bc-bg-selected); }

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
    border: none;
    padding: 6px 10px;
}
.btn-ghost:hover { background: var(--bc-bg-hover); color: var(--bc-text-primary); }

.btn-icon { width: 32px; height: 32px; padding: 0; background: transparent; border: 1px solid var(--bc-border); border-radius: 8px; color: var(--bc-text-secondary); display: inline-flex; align-items: center; justify-content: center; cursor: pointer; transition: all 150ms; }
.btn-icon:hover { background: var(--bc-bg-hover); color: var(--bc-text-primary); }

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

/* When .input is used as a flex wrapper (div.input > input), match design-kit style */
.input.sm { height: 36px; padding: 0 12px; border-radius: var(--bc-radius-sm); }
.input.sm input { font-size: 13px; }
.input input { flex:1; border:none; outline:none; background: transparent; font-size: 15px; font-family: inherit; color: var(--bc-text-primary); }
.input input::placeholder { color: var(--bc-text-tertiary); }
.input:focus-within { border-color: #000; box-shadow: 0 0 0 3px rgba(0,0,0,0.06); }

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

.bc-input-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: #6E6E73;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 8px;
}

.bc-input {
    display: flex;
    align-items: center;
    width: 100%;
    height: 48px;
    font-family: var(--bc-font-family);
    background: var(--bc-bg-card-solid);
    border: 1px solid #E5E5E5;
    border-radius: var(--bc-radius-md);
    transition: border-color 200ms cubic-bezier(0.33, 0, 0.67, 1),
                box-shadow 200ms cubic-bezier(0.33, 0, 0.67, 1);
}

.bc-input:hover { border-color: #D4D4D4; }

.bc-input-group:focus-within .bc-input {
    outline: none;
    background: var(--bc-bg-card-solid);
    border-color: #000000;
    box-shadow: 0 0 0 3px rgba(0, 0, 0, 0.06);
}

.bc-input.bc-input-error {
    border-color: var(--bc-danger);
    box-shadow: 0 0 0 3px var(--bc-danger-light);
}

.bc-input-native {
    width: 100%;
    height: 100%;
    padding: 0 16px;
    background: transparent;
    border: none;
    outline: none;
    font-family: inherit;
    font-size: 15px;
    color: #1D1D1F;
    letter-spacing: -0.01em;
}
.bc-input-native::placeholder {
    color: #B0B0B5;
    font-weight: 400;
}
.bc-input-native:focus { outline: none; }

/* Hide browser's built-in password reveal/clear buttons */
.bc-input-native::-ms-reveal,
.bc-input-native::-ms-clear {
    display: none;
}

.bc-input-error-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
}
.bc-input-error-dot {
    width: 6px;
    height: 6px;
    border-radius: 9999px;
    background: var(--bc-danger);
}
.bc-input-error-text {
    font-size: 12px;
    font-weight: 500;
    color: var(--bc-danger);
}

.bc-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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

/* Responsive overrides — these must come after the non-responsive rules above
   to fix cascade order: DESIGN_SYSTEM_CSS loads after Tailwind, so Tailwind's
   @media responsive variants would otherwise be overridden by the flat rules above. */
@media (min-width: 640px) {
    .sm\:flex-row { flex-direction: row; }
}
@media (min-width: 768px) {
    .md\:flex-row { flex-direction: row; }
    .md\:items-start { align-items: flex-start; }
    .md\:text-left { text-align: left; }
    .md\:justify-start { justify-content: flex-start; }
}

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

/* Manual dark mode toggle via [data-theme="dark"] */
[data-theme="dark"] {
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

    --bc-btn-black-bg: #FFFFFF;
    --bc-btn-black-text: #000000;
    --bc-btn-black-hover: #F5F5F7;
    --bc-btn-black-active: #E5E5EA;

    --bc-success-light: rgba(52, 199, 89, 0.15);
    --bc-warning-light: rgba(255, 149, 0, 0.15);
    --bc-danger-light: rgba(255, 59, 48, 0.15);
    --bc-info-light: rgba(90, 200, 250, 0.15);
    --bc-primary-light: rgba(0, 122, 255, 0.15);
}

/* Dark-mode overrides for hardcoded white backgrounds */
[data-theme="dark"] .page-header,
[data-theme="dark"] .stat-card,
[data-theme="dark"] .card,
[data-theme="dark"] .row-card,
[data-theme="dark"] .pick-card,
[data-theme="dark"] .chip,
[data-theme="dark"] .table,
[data-theme="dark"] .table th,
[data-theme="dark"] .bc-modal {
    background: var(--bc-bg-card-solid);
}
[data-theme="dark"] .pill.neutral {
    background: rgba(255, 255, 255, 0.08);
}
[data-theme="dark"] .input:focus-within {
    border-color: #fff;
    box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.06);
}
[data-theme="dark"] .sev-low {
    background: rgba(255, 255, 255, 0.08);
}
[data-theme="dark"] .empty-icon {
    background: rgba(255, 255, 255, 0.08);
}
[data-theme="dark"] .skeleton {
    background: linear-gradient(90deg, rgba(255,255,255,0.06) 25%, rgba(255,255,255,0.10) 50%, rgba(255,255,255,0.06) 75%);
    background-size: 200% 100%;
}

/* ═══════════════════════════════════════════════════════════════════
   Login / Register Page — 50/50 split-screen
   ═══════════════════════════════════════════════════════════════════ */

.login {
    display: grid;
    grid-template-columns: 1fr 1fr;
    min-height: 100vh;
    background: var(--bc-bg-card-solid);
    overflow: hidden;
    font-family: var(--bc-font-family);
    -webkit-font-smoothing: antialiased;
}
@media (max-width: 1023px) {
    .login { grid-template-columns: 1fr; }
    .login-brand { display: none; }
}

.login-brand {
    position: relative;
    background: #0A0A0A;
    color: #FFFFFF;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 56px;
    overflow: hidden;
}
.login-brand::before {
    content: "";
    position: absolute;
    inset: 0;
    background-image:
        radial-gradient(circle at 20% 30%, rgba(255,255,255,0.06) 0%, transparent 40%),
        radial-gradient(circle at 80% 70%, rgba(255,255,255,0.05) 0%, transparent 40%);
    pointer-events: none;
}
.login-brand::after {
    content: "";
    position: absolute;
    inset: 0;
    background-image:
        linear-gradient(rgba(255,255,255,0.04) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255,255,255,0.04) 1px, transparent 1px);
    background-size: 48px 48px;
    mask-image: radial-gradient(circle at 50% 50%, #000 30%, transparent 80%);
    -webkit-mask-image: radial-gradient(circle at 50% 50%, #000 30%, transparent 80%);
    pointer-events: none;
}
.login-brand > * { position: relative; z-index: 1; }

.login-form {
    background: #FFFFFF;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 56px 64px;
}

.login-brand-logo {
    width: 40px;
    height: 40px;
    color: #FFFFFF;
}

.login-brand-headline {
    font-size: 56px;
    line-height: 1.05;
    letter-spacing: -0.025em;
    font-weight: 700;
    color: #FFFFFF;
    margin: 0 0 24px 0;
}

.login-brand-subhead {
    font-size: 17px;
    line-height: 1.6;
    color: rgba(255,255,255,0.6);
    max-width: 420px;
    margin: 0;
    font-weight: 400;
}

.login-brand-eyebrow {
    font-size: 11px;
    font-weight: 600;
    color: rgba(255,255,255,0.4);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    margin-bottom: 16px;
}

.login-brand-version {
    font-size: 12px;
    color: rgba(255,255,255,0.4);
    font-family: var(--bc-font-mono);
}

/* Benefit rows (register mode) */
.login-benefit {
    display: flex;
    align-items: flex-start;
    gap: 12px;
}
.login-benefit-check {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(52,199,89,0.15);
    color: #34C759;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    flex-shrink: 0;
    margin-top: 2px;
}
.login-benefit-key {
    font-size: 13px;
    font-weight: 600;
    color: rgba(255,255,255,0.9);
}
.login-benefit-val {
    font-size: 12px;
    color: rgba(255,255,255,0.5);
}

/* Form side */
.login-form-title {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: #1D1D1F;
    margin: 0;
}
.login-form-subtitle {
    font-size: 14px;
    color: var(--bc-text-secondary);
    margin-top: 8px;
}

.login-input-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--bc-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 8px;
}
.login-input {
    display: flex;
    align-items: center;
    height: 48px;
    padding: 0 16px;
    background: #FFFFFF;
    border: 1px solid var(--bc-border);
    border-radius: 12px;
    transition: all 150ms cubic-bezier(0.25,0.1,0.25,1);
}
.login-input:focus-within {
    border-color: #000;
    box-shadow: 0 0 0 3px rgba(0,0,0,0.06);
}
.login-input input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    font-size: 15px;
    font-family: inherit;
    color: var(--bc-text-primary);
}
.login-input input::placeholder {
    color: var(--bc-text-tertiary);
}

/* Password strength meter */
.pw-meter {
    display: flex;
    gap: 4px;
    margin-top: 8px;
}
.pw-meter-bar {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--bc-border);
}

.login-divider {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 8px 0;
}
.login-divider-line {
    flex: 1;
    height: 1px;
    background: var(--bc-border);
}
.login-divider-text {
    font-size: 12px;
    color: var(--bc-text-tertiary);
}

.login-social-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
}

.login-footer {
    font-size: 13px;
    color: var(--bc-text-secondary);
    text-align: center;
    margin-top: 12px;
}
.login-footer a {
    color: var(--bc-primary);
    text-decoration: none;
    font-weight: 500;
    cursor: pointer;
}
.login-footer a:hover {
    text-decoration: underline;
}

/* ═══════════════════════════════════════════════════════════════════
   Landing / Marketing Page
   ═══════════════════════════════════════════════════════════════════ */

.landing { background: #FFFFFF; color: var(--bc-text-primary); font-family: var(--bc-font-family); -webkit-font-smoothing: antialiased; }
.landing ::selection { background: rgba(255,255,255,0.18); }
.landing a { color: inherit; text-decoration: none; }

.landing-wrap { max-width: 1200px; margin: 0 auto; padding: 0 32px; }

/* Nav */
.landing-nav { position: absolute; top: 0; left: 0; right: 0; z-index: 10; padding: 24px 0; }
.landing-nav-inner { display: flex; align-items: center; justify-content: space-between; }
.landing-brand { display: flex; align-items: center; gap: 10px; color: #FFFFFF; font-weight: 700; font-size: 16px; letter-spacing: -0.01em; }
.landing-brand-mark { width: 28px; height: 28px; border-radius: 8px; background: linear-gradient(135deg, #FF6B3D, #FF3B30); display: flex; align-items: center; justify-content: center; color: #FFFFFF; font-weight: 800; font-size: 14px; }
.landing-nav-links { display: flex; gap: 32px; font-size: 14px; color: rgba(255,255,255,0.72); }
.landing-nav-links a:hover { color: #FFFFFF; }
.landing-nav-cta { display: flex; gap: 12px; align-items: center; }

/* Buttons */
.landing-btn { display: inline-flex; align-items: center; justify-content: center; gap: 8px; padding: 10px 18px; border-radius: 9999px; font-size: 14px; font-weight: 600; transition: all 150ms; cursor: pointer; border: 1px solid transparent; }
.landing-btn-light { background: #FFFFFF; color: #000; }
.landing-btn-light:hover { background: rgba(255,255,255,0.92); transform: translateY(-1px); }
.landing-btn-ghost { background: transparent; color: #FFFFFF; border-color: rgba(255,255,255,0.2); }
.landing-btn-ghost:hover { border-color: rgba(255,255,255,0.45); }
.landing-btn-dark { background: #000; color: #fff; }
.landing-btn-dark:hover { background: #1a1a1a; }

/* Hero */
.landing-hero { position: relative; background: #0A0A0A; color: #FFFFFF; overflow: hidden; padding: 140px 0 100px; }
.landing-hero::before { content: ""; position: absolute; inset: 0; background-image: radial-gradient(circle at 18% 28%, rgba(255,107,61,0.18) 0%, transparent 38%), radial-gradient(circle at 82% 72%, rgba(0,122,255,0.16) 0%, transparent 42%); pointer-events: none; }
.landing-hero::after { content: ""; position: absolute; inset: 0; background-image: linear-gradient(rgba(255,255,255,0.04) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.04) 1px, transparent 1px); background-size: 56px 56px; mask-image: radial-gradient(ellipse at 50% 40%, #000 30%, transparent 80%); -webkit-mask-image: radial-gradient(ellipse at 50% 40%, #000 30%, transparent 80%); pointer-events: none; }
.landing-hero-grid { position: relative; z-index: 1; display: grid; grid-template-columns: 1fr 1fr; gap: 64px; align-items: center; }

.landing-eyebrow { display: inline-flex; align-items: center; gap: 8px; padding: 6px 12px; border-radius: 9999px; background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.1); font-size: 12px; font-weight: 500; color: rgba(255,255,255,0.78); margin-bottom: 24px; }
.landing-eyebrow .pulse-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--bc-success); box-shadow: 0 0 8px var(--bc-success); }

.landing-hero-title { font-size: 72px; font-weight: 700; letter-spacing: -0.035em; line-height: 1.02; margin: 0 0 24px; color: #FFFFFF; }
.landing-hero-title .grad { background-image: linear-gradient(135deg, #FF6B3D 0%, #FF3B30 50%, #FF9500 100%); background-clip: text; -webkit-background-clip: text; color: transparent; -webkit-text-fill-color: transparent; }
.landing-hero-sub { font-size: 19px; line-height: 1.55; color: rgba(255,255,255,0.7); margin: 0 0 36px; max-width: 520px; }
.landing-hero-ctas { display: flex; gap: 12px; }
.landing-hero-meta { display: flex; gap: 24px; margin-top: 48px; font-size: 13px; color: rgba(255,255,255,0.5); }
.landing-hero-meta .item { display: flex; align-items: center; gap: 8px; }

/* Terminal */
.landing-terminal { background: #0F0F10; border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); box-shadow: 0 24px 64px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.04) inset; overflow: hidden; }
.landing-term-bar { padding: 14px 16px; display: flex; align-items: center; gap: 8px; border-bottom: 1px solid rgba(255,255,255,0.06); }
.landing-term-dot { width: 12px; height: 12px; border-radius: 50%; }
.landing-term-title { margin-left: 12px; font-size: 12px; color: rgba(255,255,255,0.55); font-family: var(--bc-font-mono); }
.landing-term-body { padding: 18px 20px; font-family: var(--bc-font-mono); font-size: 13px; line-height: 1.7; color: #d4d4d4; white-space: pre; overflow-x: auto; }
.tk-prompt { color: #50fa7b; }
.tk-flag { color: #ff79c6; }
.tk-string { color: #f1fa8c; }
.tk-comment { color: #6272a4; }
.tk-key { color: #8be9fd; }
.tk-punct { color: #f8f8f2; }

/* Section primitives */
.landing-section { padding: 120px 0; position: relative; }
.landing-section-eyebrow { font-size: 12px; font-weight: 700; color: var(--bc-text-tertiary); text-transform: uppercase; letter-spacing: 0.18em; margin-bottom: 16px; }
.landing-section-title { font-size: 56px; font-weight: 700; letter-spacing: -0.025em; line-height: 1.05; margin: 0 0 16px; max-width: 800px; }
.landing-section-sub { font-size: 18px; color: var(--bc-text-secondary); line-height: 1.55; max-width: 640px; margin: 0 0 64px; }

/* Trust strip */
.landing-strip { background: #FFFFFF; border-bottom: 1px solid var(--bc-border); }
.landing-strip-inner { display: grid; grid-template-columns: repeat(4, 1fr); }
.landing-strip-item { padding: 32px 24px; border-right: 1px solid var(--bc-border); display: flex; flex-direction: column; gap: 6px; }
.landing-strip-item:last-child { border-right: none; }
.landing-strip-num { font-size: 32px; font-weight: 700; letter-spacing: -0.02em; line-height: 1; }
.landing-strip-num .unit { font-size: 16px; color: var(--bc-text-secondary); font-weight: 500; margin-left: 6px; }
.landing-strip-label { font-size: 13px; color: var(--bc-text-secondary); }

/* Values grid */
.landing-values { display: grid; grid-template-columns: repeat(12, 1fr); gap: 16px; }
.landing-value-card { background: #FFFFFF; border: 1px solid var(--bc-border); border-radius: 16px; padding: 32px; transition: all 200ms; position: relative; overflow: hidden; }
.landing-value-card:hover { transform: translateY(-2px); box-shadow: 0 16px 48px rgba(0,0,0,0.08); border-color: var(--bc-border-hover); }
.landing-value-card.span-7 { grid-column: span 7; }
.landing-value-card.span-5 { grid-column: span 5; }
.landing-value-card.span-4 { grid-column: span 4; }
.landing-value-card.span-12 { grid-column: span 12; }
.landing-value-card.dark { background: #0A0A0A; color: #FFFFFF; border-color: rgba(255,255,255,0.08); }
.landing-value-card.dark .v-eyebrow { color: rgba(255,255,255,0.5); }
.landing-value-card.dark .v-desc { color: rgba(255,255,255,0.65); }
.v-icon { width: 44px; height: 44px; border-radius: 12px; background: var(--bc-bg-hover); display: flex; align-items: center; justify-content: center; margin-bottom: 24px; color: var(--bc-text-primary); }
.landing-value-card.dark .v-icon { background: rgba(255,255,255,0.08); color: #FFFFFF; }
.v-eyebrow { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.16em; color: var(--bc-text-tertiary); margin-bottom: 8px; }
.v-title { font-size: 22px; font-weight: 700; letter-spacing: -0.015em; margin: 0 0 12px; line-height: 1.2; }
.v-desc { font-size: 14px; line-height: 1.6; color: var(--bc-text-secondary); }

/* Aggregation diagram */
.landing-agg { background: #0A0A0A; color: #FFFFFF; }
.landing-agg .landing-section-title { color: #FFFFFF; }
.landing-agg .landing-section-sub { color: rgba(255,255,255,0.65); }
.landing-agg .landing-section-eyebrow { color: rgba(255,255,255,0.5); }
.landing-agg-stage { display: grid; grid-template-columns: 1fr auto 1fr auto 1fr; gap: 24px; align-items: center; margin-top: 56px; }
.landing-agg-col { display: flex; flex-direction: column; gap: 12px; }
.landing-agg-col.center { align-items: stretch; }
.landing-agg-node { padding: 16px 20px; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.landing-agg-node strong { font-size: 14px; font-weight: 600; }
.landing-agg-node .meta { font-family: var(--bc-font-mono); font-size: 11px; color: rgba(255,255,255,0.45); }
.landing-agg-arrow { color: rgba(255,255,255,0.3); font-size: 20px; text-align: center; }
.landing-agg-core { background: linear-gradient(135deg, #FF6B3D, #FF3B30); border: none; padding: 32px 24px; border-radius: 20px; text-align: center; box-shadow: 0 0 64px rgba(255,107,61,0.3); }
.landing-agg-core .label { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.16em; opacity: 0.85; }
.landing-agg-core .name { font-size: 22px; font-weight: 700; margin-top: 6px; line-height: 1.1; }
.landing-agg-core .ver { font-family: var(--bc-font-mono); font-size: 11px; opacity: 0.75; margin-top: 6px; }
.landing-agg-out { background: rgba(0,122,255,0.12); border: 1px solid rgba(0,122,255,0.3); }
.landing-agg-out strong { color: #5AC8FA; }

/* Architecture */
.landing-arch { background: var(--bc-bg-canvas); }
.landing-arch-diagram { background: #FFFFFF; border: 1px solid var(--bc-border); border-radius: 20px; padding: 48px; position: relative; box-shadow: 0 8px 32px rgba(0,0,0,0.04); }
.landing-arch-layer { display: grid; grid-template-columns: 220px 1fr; gap: 32px; align-items: center; padding: 24px 0; border-bottom: 1px dashed var(--bc-border); }
.landing-arch-layer:last-child { border-bottom: none; }
.landing-arch-tag { font-family: var(--bc-font-mono); font-size: 12px; padding: 6px 12px; background: var(--bc-bg-hover); border-radius: 6px; display: inline-block; color: var(--bc-text-secondary); font-weight: 500; margin-bottom: 8px; }
.landing-arch-name { font-size: 20px; font-weight: 700; letter-spacing: -0.01em; }
.landing-arch-desc { font-size: 14px; color: var(--bc-text-secondary); margin-top: 4px; line-height: 1.5; }
.landing-arch-flow { display: flex; gap: 12px; flex-wrap: wrap; }
.landing-arch-chip { padding: 6px 12px; border-radius: 8px; font-size: 12px; font-weight: 500; background: #FFFFFF; border: 1px solid var(--bc-border); color: var(--bc-text-secondary); }
.landing-arch-chip.accent { background: #FFF5F0; border-color: #FFD9C2; color: #C2410C; }

/* Code section */
.landing-code-section { background: var(--bc-bg-canvas); }
.landing-code-grid { display: grid; grid-template-columns: 1fr 1.4fr; gap: 64px; align-items: center; }
.landing-code-feat { font-size: 14px; color: var(--bc-text-secondary); display: flex; flex-direction: column; gap: 16px; margin-top: 32px; }
.landing-code-feat .pt { display: flex; gap: 12px; align-items: flex-start; }
.landing-code-feat .pt-mark { flex-shrink: 0; width: 24px; height: 24px; border-radius: 50%; background: var(--bc-text-primary); color: #FFFFFF; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 700; }
.landing-code-feat strong { color: var(--bc-text-primary); display: block; margin-bottom: 2px; font-size: 14px; }

/* Roadmap */
.landing-roadmap-track { display: grid; grid-template-columns: repeat(6, 1fr); gap: 0; margin-top: 56px; position: relative; }
.landing-roadmap-track::before { content: ""; position: absolute; top: 18px; left: 32px; right: 32px; height: 2px; background: var(--bc-border); }
.landing-rm-step { padding: 0 16px; position: relative; }
.landing-rm-dot { width: 36px; height: 36px; border-radius: 50%; background: #FFFFFF; border: 2px solid var(--bc-border); display: flex; align-items: center; justify-content: center; position: relative; z-index: 1; font-weight: 700; font-size: 13px; color: var(--bc-text-tertiary); }
.landing-rm-step.done .landing-rm-dot { background: var(--bc-success); border-color: var(--bc-success); color: #FFFFFF; }
.landing-rm-step.active .landing-rm-dot { background: #FFFFFF; border-color: var(--bc-text-primary); color: var(--bc-text-primary); box-shadow: 0 0 0 6px rgba(0,0,0,0.06); }
.landing-rm-ver { margin-top: 16px; font-size: 13px; font-weight: 700; font-family: var(--bc-font-mono); }
.landing-rm-title { margin-top: 4px; font-size: 13px; font-weight: 600; line-height: 1.3; }
.landing-rm-desc { margin-top: 6px; font-size: 12px; color: var(--bc-text-secondary); line-height: 1.4; }
.landing-rm-pill { display: inline-block; margin-top: 8px; font-size: 10px; font-weight: 700; padding: 2px 8px; border-radius: 9999px; text-transform: uppercase; letter-spacing: 0.08em; }
.landing-rm-pill.done { background: var(--bc-success-light); color: var(--bc-success); }
.landing-rm-pill.in-prog { background: var(--bc-warning-light); color: var(--bc-warning); }
.landing-rm-pill.next { background: var(--bc-bg-hover); color: var(--bc-text-secondary); }

/* Final CTA */
.landing-final-cta { background: #0A0A0A; color: #FFFFFF; overflow: hidden; position: relative; padding: 96px 0; }
.landing-final-cta::before { content: ""; position: absolute; inset: 0; background-image: radial-gradient(circle at 50% 100%, rgba(255,107,61,0.25) 0%, transparent 60%), radial-gradient(circle at 90% 30%, rgba(0,122,255,0.15) 0%, transparent 50%); pointer-events: none; }
.landing-final-cta-inner { position: relative; z-index: 1; text-align: center; }

/* Footer */
.landing-footer { background: #050505; color: rgba(255,255,255,0.6); padding: 56px 0 40px; font-size: 13px; }
.landing-foot-grid { display: grid; grid-template-columns: 2fr 1fr 1fr 1fr; gap: 48px; margin-bottom: 48px; }
.landing-foot-h { color: #FFFFFF; font-weight: 700; font-size: 13px; margin-bottom: 16px; }
.landing-foot-grid ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 10px; }
.landing-foot-grid a:hover { color: #FFFFFF; }
.landing-foot-bottom { display: flex; justify-content: space-between; padding-top: 24px; border-top: 1px solid rgba(255,255,255,0.06); font-size: 12px; color: rgba(255,255,255,0.4); }

/* BCButton sizes — concrete utility (no arbitrary values) */
.bc-btn-sm {
    min-height: 28px;
    padding: 4px 10px;
    font-size: 12px;
}
.bc-btn-lg {
    min-height: 48px;
    padding: 12px 20px;
    font-size: 15px;
    border-radius: var(--bc-radius-md);
}
.bc-btn-block { width: 100%; }
.bc-btn-press {
    transition: transform 200ms cubic-bezier(0.33, 0, 0.67, 1),
                background 200ms cubic-bezier(0.33, 0, 0.67, 1);
}
.bc-btn-press:hover { transform: scale(1.02); }
.bc-btn-press:active { transform: scale(0.98); }

.bc-btn-icon {
    width: 20px;
    height: 20px;
    margin-right: 8px;
    flex-shrink: 0;
}

/* ═══════════════════════════════════════════════════════════════════
   DaisyUI Replacements
   Minimal re-implementations of the DaisyUI classes still referenced
   across the codebase, so daisyui.css can be removed entirely.
   All values come from --bc-* design tokens.
   ═══════════════════════════════════════════════════════════════════ */

/* Loading spinner — replaces `loading loading-spinner loading-{xs,sm,md,lg}` */
.loading {
    display: inline-block;
    vertical-align: middle;
}
.loading-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--bc-border);
    border-top-color: var(--bc-primary);
    border-radius: 9999px;
    animation: bc-spin 0.7s linear infinite;
}
.loading-xs { width: 12px; height: 12px; border-width: 2px; }
.loading-sm { width: 16px; height: 16px; border-width: 2px; }
.loading-md { width: 24px; height: 24px; border-width: 3px; }
.loading-lg { width: 40px; height: 40px; border-width: 4px; }
@keyframes bc-spin { to { transform: rotate(360deg); } }

/* Toggle switch — replaces `toggle toggle-success toggle-sm` */
.toggle {
    appearance: none;
    -webkit-appearance: none;
    width: 40px;
    height: 22px;
    background: var(--bc-border);
    border-radius: 9999px;
    position: relative;
    cursor: pointer;
    transition: background var(--bc-transition-fast);
    flex-shrink: 0;
    margin: 0;
}
.toggle::before {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    background: var(--bc-bg-card-solid);
    border-radius: 9999px;
    box-shadow: var(--bc-shadow-xs);
    transition: transform var(--bc-transition-fast);
}
.toggle:checked { background: var(--bc-primary); }
.toggle:checked::before { transform: translateX(18px); }
.toggle-success:checked { background: var(--bc-success); }
.toggle-sm { width: 32px; height: 18px; }
.toggle-sm::before { width: 14px; height: 14px; }
.toggle-sm:checked::before { transform: translateX(14px); }

/* Select bordered — replaces `select select-bordered select-sm select-primary` */
.select {
    display: block;
    width: 100%;
    padding: 0 36px 0 12px;
    height: 38px;
    font-size: var(--bc-font-base);
    font-family: inherit;
    color: var(--bc-text-primary);
    background: var(--bc-bg-card-solid);
    border: 1px solid var(--bc-border);
    border-radius: var(--bc-radius-sm);
    outline: none;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    background-image: url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2386868B' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 12px center;
    transition: border-color var(--bc-transition-fast), box-shadow var(--bc-transition-fast);
}
.select:hover { border-color: var(--bc-border-hover); }
.select:focus, .select.select-primary:focus {
    border-color: var(--bc-primary);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.15);
}
.select-bordered { /* alias */ }
.select-primary {}
.select-sm { height: 32px; font-size: var(--bc-font-sm); padding-right: 32px; }

/* Textarea bordered — replaces `textarea textarea-bordered` */
.textarea {
    display: block;
    width: 100%;
    padding: 10px 12px;
    font-size: var(--bc-font-base);
    font-family: inherit;
    color: var(--bc-text-primary);
    background: var(--bc-bg-card-solid);
    border: 1px solid var(--bc-border);
    border-radius: var(--bc-radius-sm);
    outline: none;
    resize: vertical;
    transition: border-color var(--bc-transition-fast), box-shadow var(--bc-transition-fast);
}
.textarea:hover { border-color: var(--bc-border-hover); }
.textarea:focus {
    border-color: var(--bc-primary);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.15);
}
.textarea-bordered {}

/* Input modifiers — `.input` is already defined above; add DaisyUI aliases */
.input-bordered {}
.input-primary:focus {
    border-color: var(--bc-primary);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.15);
}

/* Form control / label — replaces `form-control`, `label`, `label-text` */
.form-control {
    display: flex;
    flex-direction: column;
    gap: var(--bc-space-1);
}
.label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--bc-space-1) 0;
}
.label-text {
    font-size: var(--bc-font-sm);
    color: var(--bc-text-primary);
}

/* Alert — replaces `alert alert-info alert-warning alert-success alert-error` */
.alert {
    display: flex;
    align-items: center;
    gap: var(--bc-space-2);
    padding: var(--bc-space-3) var(--bc-space-4);
    border-radius: var(--bc-radius-md);
    border: 1px solid transparent;
    font-size: var(--bc-font-base);
}
.alert-info    { background: var(--bc-info-light);    color: var(--bc-info);    border-color: var(--bc-info); }
.alert-warning { background: var(--bc-warning-light); color: var(--bc-warning); border-color: var(--bc-warning); }
.alert-success { background: var(--bc-success-light); color: var(--bc-success); border-color: var(--bc-success); }
.alert-error   { background: var(--bc-danger-light);  color: var(--bc-danger);  border-color: var(--bc-danger); }

/* Join — replaces `join` + `join-item` (segmented button groups) */
.join { display: inline-flex; align-items: stretch; }
.join > .join-item { border-radius: 0; }
.join > .join-item:first-child {
    border-top-left-radius: var(--bc-radius-sm);
    border-bottom-left-radius: var(--bc-radius-sm);
}
.join > .join-item:last-child {
    border-top-right-radius: var(--bc-radius-sm);
    border-bottom-right-radius: var(--bc-radius-sm);
}
.join > .join-item + .join-item { margin-left: -1px; }

/* Tooltip — replaces `tooltip tooltip-{top,bottom,left,right}` */
.tooltip { position: relative; }
.tooltip::after {
    content: attr(data-tip);
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    padding: var(--bc-space-1) var(--bc-space-2);
    background: #1D1D1F;
    color: #FFFFFF;
    font-size: var(--bc-font-xs);
    border-radius: var(--bc-radius-xs);
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--bc-transition-fast);
    z-index: 1000;
}
.tooltip:hover::after { opacity: 1; }
.tooltip-bottom::after { top: calc(100% + 4px); }
.tooltip-top::after    { bottom: calc(100% + 4px); }

/* Button modifiers — extends `.btn` already defined above */
.btn-circle {
    width: 36px;
    height: 36px;
    padding: 0;
    border-radius: 9999px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
}
.btn-xs { height: 26px; padding: 0 10px; font-size: var(--bc-font-xs); border-radius: var(--bc-radius-xs); }
.btn-sm { height: 32px; padding: 0 14px; font-size: var(--bc-font-sm); }

/* Badge — replaces `badge badge-ghost badge-xs` */
.badge {
    display: inline-flex;
    align-items: center;
    padding: 2px var(--bc-space-2);
    border-radius: 9999px;
    font-size: var(--bc-font-xs);
    font-weight: 500;
    background: var(--bc-bg-hover);
    color: var(--bc-text-secondary);
}
.badge-ghost  { background: var(--bc-bg-hover); color: var(--bc-text-secondary); }
.badge-xs     { padding: 1px 6px; font-size: 10px; }

/* ═══════════════════════════════════════════════════════════════════
   Page-level helpers — from design kit styles.css
   ═══════════════════════════════════════════════════════════════════ */

/* Stats grid */
.stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; }
.stats-grid.cols-4 { grid-template-columns: repeat(4, 1fr); }
.stat-card { background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); border-radius: 12px; padding: 20px; display: flex; flex-direction: column; gap: 6px; }
.stat-eyebrow { font-size: 10px; font-weight: 600; color: var(--bc-text-tertiary); text-transform: uppercase; letter-spacing: 0.16em; }
.stat-value { font-size: 28px; font-weight: 700; letter-spacing: -0.02em; line-height: 1; color: var(--bc-text-primary); display:flex; align-items: baseline; gap: 8px; }
.stat-value.lg { font-size: 36px; }
.stat-value.success { color: var(--bc-success); }
.stat-pill { font-size: 11px; font-weight: 500; padding: 2px 6px; border-radius: 4px; }
.stat-pill.success { color: var(--bc-success); background: var(--bc-success-light); }
.stat-pill.danger { color: var(--bc-danger); background: var(--bc-danger-light); }
.stat-pill.muted { color: var(--bc-text-tertiary); }

/* Page layout rhythm */
.page-header {
  background: var(--bc-bg-card-solid);
  border-bottom: 1px solid var(--bc-border);
  padding: 20px 24px;
  display: flex; align-items: center; gap: 16px;
}
.page-title { font-size: 20px; font-weight: 600; letter-spacing: -0.01em; }
.page-sub { font-size: 13px; color: var(--bc-text-secondary); margin-top: 2px; }
.page-content { padding: 24px; overflow-y: auto; }

/* Table */
.table { width: 100%; border-collapse: collapse; background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); border-radius: 8px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
.table th { text-align: left; padding: 12px 16px; font-size: 11px; font-weight: 600; color: var(--bc-text-secondary); text-transform: uppercase; letter-spacing: 0.16em; border-bottom: 1px solid var(--bc-border); background: var(--bc-bg-card-solid); }
.table td { padding: 14px 16px; font-size: 14px; border-bottom: 1px solid var(--bc-border); }
.table tr:last-child td { border-bottom: none; }
.table tr:hover td { background: var(--bc-bg-hover); }
.table .mono { font-family: var(--bc-font-mono); font-size: 13px; color: var(--bc-text-secondary); }

/* Modal */
.bc-modal-overlay { position: absolute; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 100; }
.bc-modal { width: 480px; background: var(--bc-bg-card-solid); border-radius: 12px; box-shadow: 0 24px 64px rgba(0,0,0,0.20); overflow: hidden; }
.bc-modal-header { padding: 20px 24px; border-bottom: 1px solid var(--bc-border); display:flex; align-items:center; justify-content:space-between; }
.bc-modal-body { padding: 24px; display:flex; flex-direction:column; gap: 16px; }
.bc-modal-footer { padding: 16px 24px; border-top: 1px solid var(--bc-border); display:flex; gap: 8px; justify-content: flex-end; }
.bc-modal-title { font-size: 17px; font-weight: 600; }

/* Input label (design-kit style) */
.input-label { display:block; font-size: 11px; font-weight: 600; color: var(--bc-text-secondary); text-transform: uppercase; letter-spacing: 0.08em; margin-bottom: 8px; }

/* Section titles — single rhythm across the app */
.section-h { font-size: 13px; font-weight: 500; color: var(--bc-text-secondary); border-bottom: 1px solid var(--bc-border); padding-bottom: 8px; margin: 0 0 16px; display:flex; align-items: flex-end; justify-content: space-between; gap: 12px; }
.section-h.row { /* legacy alias */ }
.section-h.lg { font-size: 15px; color: var(--bc-text-primary); font-weight: 600; padding-bottom: 10px; }
.section-h .lead { display:flex; flex-direction: column; gap: 2px; }
.section-h .lead-title { font-size: inherit; font-weight: inherit; color: inherit; }
.section-h .lead-sub { font-size: 11px; color: var(--bc-text-tertiary); font-weight: 400; }
.section-sub { font-size: 11px; color: var(--bc-text-tertiary); font-weight: 400; }

/* Chip / filter tags */
.chip-row { display:flex; gap: 6px; flex-wrap: wrap; }
.chip { display:inline-flex; align-items:center; gap: 6px; padding: 5px 11px; border-radius: 9999px; font-size: 12px; font-weight: 500; background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); color: var(--bc-text-secondary); cursor: pointer; transition: all 150ms; font-family: inherit; }
.chip:hover { border-color: var(--bc-border-hover); color: var(--bc-text-primary); }
.chip.active { background: var(--bc-text-primary); color: var(--bc-text-on-accent); border-color: var(--bc-text-primary); }
.chip .chip-count { font-variant-numeric: tabular-nums; opacity: 0.6; font-weight: 600; }
.chip.active .chip-count { opacity: 0.7; }

/* Sparkline */
.spark { display:flex; align-items: flex-end; gap: 3px; height: 40px; }
.spark .bar { flex:1; background: var(--bc-primary); border-radius: 2px; opacity: 0.85; }
.spark.sm { height: 28px; }
.spark .bar.success { background: var(--bc-success); }
.spark .bar.warning { background: var(--bc-warning); }
.spark .bar.danger  { background: var(--bc-danger);  }

/* Tabs (underline style) */
.tabs { display: flex; gap: 24px; border-bottom: 1px solid var(--bc-border); padding-bottom: 0; }
.tabs.compact { gap: 16px; }
.tab { font-size: 13px; font-weight: 500; color: var(--bc-text-tertiary); padding: 0 0 12px; border-bottom: 2px solid transparent; margin-bottom: -1px; cursor: pointer; background: none; border-left: none; border-right: none; border-top: none; transition: color 150ms; }
.tab:hover { color: var(--bc-text-secondary); }
.tab.active { color: var(--bc-text-primary); border-bottom-color: var(--bc-text-primary); font-weight: 600; }

/* Bordered list rows */
.row-card { background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); border-radius: 12px; padding: 16px 20px; display:flex; align-items:center; justify-content: space-between; gap: 16px; transition: all 150ms; }
.row-card:hover { box-shadow: var(--bc-shadow-sm); }
.row-card.outlined { box-shadow: none; }
.row-card.outlined:hover { box-shadow: var(--bc-shadow-sm); }

/* Marketplace-style picker cards */
.pick-card { background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); border-radius: 12px; padding: 14px; transition: all 150ms; cursor: pointer; }
.pick-card:hover { border-color: var(--bc-border-hover); box-shadow: var(--bc-shadow-sm); transform: translateY(-1px); }

/* Sidebar / config rail labels */
.config-label { font-size: 10px; font-weight: 600; color: var(--bc-text-tertiary); text-transform: uppercase; letter-spacing: 0.16em; margin-bottom: 8px; display:block; }
.config-row + .config-row { margin-top: 20px; }

/* Pills / status badges */
.pill { display:inline-flex; align-items:center; gap: 6px; padding: 4px 10px; border-radius: 9999px; font-size: 12px; font-weight: 600; }
.pill .dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
.pill.success { background: var(--bc-success-light); color: var(--bc-success); }
.pill.warning { background: var(--bc-warning-light); color: var(--bc-warning); }
.pill.danger { background: var(--bc-danger-light); color: var(--bc-danger); }
.pill.info { background: var(--bc-info-light); color: var(--bc-info); }
.pill.neutral { background: rgba(0,0,0,0.04); color: var(--bc-text-secondary); }

/* Log / code block */
.log-block { font-family: var(--bc-font-mono); font-size: 12.5px; background: var(--bc-bg-code); color: var(--bc-text-code); padding: 16px; border-radius: 8px; line-height: 1.7; white-space: pre; overflow:auto; }
.log-time { color: var(--bc-log-time); }
.log-info { color: var(--bc-log-info); }
.log-warn { color: var(--bc-log-warn); }
.log-err  { color: var(--bc-log-err); }

/* Black/neutral CTA button */
.btn-black { background: var(--bc-btn-black-bg); color: var(--bc-btn-black-text); }
.btn-black:hover { background: var(--bc-btn-black-hover); }
.btn-black:active { background: var(--bc-btn-black-active); }

/* Toggle switch */
.switch { position: relative; width: 36px; height: 22px; flex-shrink: 0; }
.switch input { opacity: 0; width: 0; height: 0; }
.switch-track { position: absolute; inset: 0; background: var(--bc-border-hover); border-radius: 9999px; transition: background 200ms; cursor: pointer; }
.switch-track::after { content: ""; position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; background: var(--bc-bg-card-solid); border-radius: 50%; transition: transform 200ms; box-shadow: 0 1px 3px rgba(0,0,0,0.15); }
.switch input:checked + .switch-track { background: var(--bc-success); }
.switch input:checked + .switch-track::after { transform: translateX(14px); }

/* Empty state */
.empty { display:flex; flex-direction:column; align-items: center; justify-content: center; padding: 48px 24px; gap: 12px; text-align: center; color: var(--bc-text-secondary); }
.empty-icon { width: 56px; height: 56px; border-radius: 50%; background: var(--bc-bg-hover); display:flex; align-items:center; justify-content:center; color: var(--bc-text-disabled); margin-bottom: 4px; }

/* Tabular numerics */
.mono, .tabular, .stat-value, .table .mono, td.mono { font-variant-numeric: tabular-nums; }

/* Threat severity */
.sev-high { background: var(--bc-danger-light); color: var(--bc-danger); }
.sev-medium { background: var(--bc-warning-light); color: var(--bc-warning); }
.sev-low { background: rgba(0,0,0,0.04); color: var(--bc-text-secondary); }

/* Card variants */
.card.flat { box-shadow: none; }
.card.tight { padding: 14px; }

/* Stat card extras */
.stat-card .stat-foot { font-size: 11px; color: var(--bc-text-tertiary); margin-top: 4px; display:flex; align-items:center; gap: 6px; }
.stat-card .stat-foot.up { color: var(--bc-success); }
.stat-card .stat-foot.down { color: var(--bc-danger); }
.stat-card.dark { background: #0A0A0A; color: var(--bc-text-on-accent); border-color: transparent; }
.stat-card.dark .stat-eyebrow { color: rgba(255,255,255,0.5); }
.stat-card.dark .stat-value { color: #fff; }

/* Form rows */
.form-row { display:flex; flex-direction: column; gap: 8px; }
.form-row select, .form-row textarea { font-family: inherit; font-size: 14px; }
.select-input { display:flex; align-items:center; height: 40px; padding: 0 12px; background: var(--bc-bg-card-solid); border: 1px solid var(--bc-border); border-radius: 8px; font-size: 14px; min-width: 160px; }
.select-input select { flex:1; border:none; background: transparent; outline: none; font-family: inherit; font-size: 14px; color: var(--bc-text-primary); appearance: none; -webkit-appearance: none; padding-right: 16px; }

/* Skeleton / shimmer loading */
@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}
.skeleton {
  background: linear-gradient(90deg, var(--bc-bg-hover) 25%, rgba(0,0,0,0.06) 50%, var(--bc-bg-hover) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
  border-radius: 6px;
}
.skeleton-kpi { width: 100%; height: 80px; }
.skeleton-row { width: 100%; height: 48px; margin-bottom: 8px; }
.skeleton-bar { width: 60%; height: 16px; margin-bottom: 8px; }

/* Error banner */
.error-banner {
  background: var(--bc-danger-light);
  border: 1px solid var(--bc-danger);
  border-radius: 8px;
  padding: 12px 16px;
  display: flex; align-items: center; gap: 12px;
  color: var(--bc-danger);
  font-size: 13px;
}
.error-banner .retry-btn {
  margin-left: auto;
  padding: 4px 12px;
  border-radius: 6px;
  background: var(--bc-danger);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: opacity 150ms;
}
.error-banner .retry-btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* ═══════════════════════════════════════════════════════════════════
   Token-compliant utility classes (Issue #179 migration)
   ═══════════════════════════════════════════════════════════════════ */

/* Gap scale extensions */
.bc-gap-xs { gap: 2px; }
.bc-gap-6 { gap: var(--bc-space-6); }   /* 24px */
.bc-gap-7 { gap: 28px; }
.bc-gap-8 { gap: var(--bc-space-8); }   /* 32px */
.bc-gap-9 { gap: 36px; }
.bc-gap-10 { gap: var(--bc-space-10); } /* 40px */

/* Brand / semantic text color utilities */
.bc-text-brand { color: var(--bc-primary); }
.bc-text-success { color: var(--bc-success); }
.bc-text-danger { color: var(--bc-danger); }
.bc-text-warning { color: var(--bc-warning); }
.bc-text-info { color: var(--bc-info); }

/* Font size utilities for intermediate sizes */
.bc-font-13 { font-size: 13px; }
.bc-font-11 { font-size: 11px; }
.bc-font-15 { font-size: 15px; }
.bc-font-17 { font-size: 17px; }
.bc-font-emoji { font-size: 40px; }
.bc-font-emoji-sm { font-size: 32px; }

/* Eyebrow label (uppercase micro-label) */
.bc-eyebrow { font-size: 10px; color: var(--bc-text-tertiary); text-transform: uppercase; letter-spacing: 0.16em; }

/* Icon circle (40px round icon container) */
.bc-icon-circle { width: 40px; height: 40px; border-radius: 99px; display: flex; align-items: center; justify-content: center; }
.bc-icon-circle-brand { background: var(--bc-primary-light); }

/* Grid utilities */
.bc-grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); }
.bc-grid-2-1 { display: grid; grid-template-columns: 2fr 1fr; }
.bc-col-span-2 { grid-column: span 2; }
.bc-col-span-4 { grid-column: span 4; }

/* Border utilities */
.bc-border-l { border-left: 1px solid var(--bc-border); }
.bc-border-l-2 { border-left: 2px solid var(--bc-border); }

/* Indent left (marketplace sidebar) */
.bc-indent-left { padding-left: 20px; margin-left: 8px; border-left: 2px solid var(--bc-border); }

/* Padding left extension */
.bc-pl-6 { padding-left: var(--bc-space-6); }  /* 24px */

/* Extra-small button */
.bc-btn-xs { min-height: 24px; padding: 2px 10px; font-size: 12px; }

/* Security score card (monitor) */
.bc-score-card { grid-column: span 2; flex-direction: row; align-items: center; justify-content: space-between; padding: 24px; position: relative; overflow: hidden; }
.bc-score-glow { position: absolute; right: 0; top: 0; bottom: 0; width: 160px; opacity: 0.45; pointer-events: none; }
.bc-score-body { display: flex; flex-direction: column; gap: 6px; z-index: 1; }
.bc-score-value { font-size: 56px; font-weight: 700; letter-spacing: -0.03em; line-height: 1; }
.bc-score-label { font-size: 13px; font-weight: 500; }
.bc-score-shield { width: 64px; height: 64px; border-radius: 99px; display: flex; align-items: center; justify-content: center; z-index: 1; font-size: 28px; }

/* Emergency modal warning box */
.bc-modal-warning { margin-bottom: 16px; padding: 12px; background: var(--bc-danger-light); color: var(--bc-danger); border-radius: 8px; font-size: 13px; }

/* Info tip box */
.bc-info-tip { margin-top: 16px; padding: 16px; font-size: 12px; line-height: 1.6; background: var(--bc-info-light); color: var(--bc-info); border-radius: 12px; }

/* Status dot (8px round indicator) */
.bc-status-dot { width: 8px; height: 8px; border-radius: 99px; }

/* Dynamic style slots (--bc-dynamic-* pattern) */
.bc-dynamic-color { color: var(--bc-dynamic-color); }
.bc-dynamic-bg { background: var(--bc-dynamic-bg); }
.bc-dynamic-border-color { border-color: var(--bc-dynamic-border-color); }
.bc-dynamic-opacity { opacity: var(--bc-dynamic-opacity); }
.bc-dynamic-display { display: var(--bc-dynamic-display); }

/* Margin-top micro */
.bc-mt-2 { margin-top: 2px; }
.bc-mt-6 { margin-top: 6px; }

/* Padding-top micro */
.bc-pt-10 { padding-top: 10px; }

/* Pool metric column (right-aligned) */
.bc-pool-metric { text-align: right; }

/* Pool metric value with brand color */
.bc-pool-value-brand { font-size: 17px; font-weight: 700; color: var(--bc-primary); margin-top: 2px; }

/* Marketplace card footer row */
.bc-marketplace-footer { display: flex; justify-content: space-between; align-items: center; margin-top: 12px; padding-top: 10px; border-top: 1px solid var(--bc-border); }

/* Emergency button padding */
.bc-btn-emergency { padding-left: 24px; padding-right: 24px; }

/* Modal form row */
.bc-modal-form-row { margin-bottom: 16px; }
.bc-modal-form-label { font-size: 13px; font-weight: 500; display: block; margin-bottom: 6px; }

/* Error text inline */
.bc-error-text { font-size: 12px; color: var(--bc-danger); }

/* Flex row with gap */
.bc-flex-row-end { display: flex; gap: 12px; justify-content: flex-end; }

/* h3 reset */
.bc-h3 { font-size: 15px; font-weight: 700; margin: 0; }

/* Zero margin */
.m-0 { margin: 0; }
"#;
