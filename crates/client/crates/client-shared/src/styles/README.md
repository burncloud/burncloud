# Design system CSS fragments

Assembled by `mod.rs` into `DESIGN_SYSTEM_CSS` → `AppStyles`.

| File | Contents |
|------|----------|
| `00_*` | `:root` tokens + legacy aliases |
| `01–08` | Base, acrylic, card, button, input, progress, status, animations |
| `09` | Legacy spacing helpers (`gap-md` — prefer Tailwind on new pages) |
| `10–16` | Typography, layout, nav, metrics, log, scrollbar, titlebar |
| `17` | Dark + **system** (`prefers-color-scheme`) theme |
| `18` | Login / register (Guest) |
| `19–25` | Console utilities, landing, schema selectors, migrations |

Rules: `docs/ui/system.md` · Tokens: `docs/ui/tokens.md`
