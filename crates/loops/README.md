# burncloud-loops

Rust orchestration for agent-driven UI optimization loops.

## Layout

| Layer | Location |
|-------|----------|
| Loop runner + gates | `crates/loops/src/` |
| Acceptance criteria | `crates/loops/acceptance/*.md` |
| E2E browser tests | `crates/tests/tests/e2e/` |
| Runtime artifacts | `data/loops/` |

## Commands

```powershell
cargo run -p burncloud-loops -- list-gates

# Single gate (debug)
cargo run -p burncloud-loops -- gate css-naming --verbose
cargo run -p burncloud-loops -- gate aesthetic-metrics --verbose

# Gate suites
cargo run -p burncloud-loops -- gates jobs-fast --verbose
cargo run -p burncloud-loops -- gates aesthetic-full --verbose

# Loops
cargo run -p burncloud-loops -- jobs-aesthetic --check-only
cargo run -p burncloud-loops -- jobs-aesthetic --full-css-gate
cargo run -p burncloud-loops -- css-optimize
cargo run -p burncloud-loops -- css-optimize --skip-visual
```

## Gate categories

| Gate | Implementation |
|------|----------------|
| `css-naming` | Rust static scan (naming.md + BCButton rules) |
| `css-all` | naming + `css_visual_acceptance` test |
| `css-visual` | `cargo test css_visual_acceptance` |
| `aesthetic-metrics` | `cargo test aesthetic_acceptance` (preview routes) |
| `aesthetic-review` | Rust `review.json` validation (J3/J4) |

## Logs

Each iteration writes timestamped gate output to `data/loops/<loop>/loop-check-{N}.log`:

```
[2026-07-08T01:49:00Z] [iter 1] [gate:css-naming] INFO START
  | PASS: Console CSS naming OK (all acceptance rules)
[2026-07-08T01:49:04Z] [iter 1] [gate:css-naming] INFO PASS (4.1s)
```

Agent prompts: `data/loops/jobs-aesthetic/agent-prompt.md`
