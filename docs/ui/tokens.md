---
doc_id: ui.tokens
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: 74ee1d6212f4ab796838bbd824885a3095b7bfb9
---

# UI Token Source of Truth

BurnCloud's current Dioxus console has one canonical semantic visual layer:

`crates/client/src/visual_system.css`

It is loaded last by `crates/client/src/app.rs` so semantic visual decisions win over legacy and page-local CSS. The file owns the stable token families for:

- typography;
- neutral surfaces and text hierarchy;
- borders and elevation;
- brand and interaction states;
- success, warning and danger states;
- spacing, radius and control sizing;
- console layout dimensions;
- motion and reduced-motion behavior.

Legacy/page CSS may keep structural selectors, but new color, spacing, radius, shadow or interaction decisions should consume `--bc-*` semantic tokens instead of introducing another palette or local scale.

Compatibility aliases such as `--canvas`, `--muted`, `--border`, `--panel`, `--surface-subtle` and `--shadow-card` exist only to bridge current page CSS into the canonical layer. Do not treat those aliases as a second token system.

Before adding a visual value:

1. search `crates/client/src/visual_system.css` for an existing semantic token;
2. prefer an existing component rule before adding a page-local override;
3. if a genuinely new visual decision is needed, add a semantic `--bc-*` token first;
4. keep raw palette literals centralized in `visual_system.css` rather than product/page CSS;
5. run `bash crates/client/scripts/check-visual-system.sh` plus the existing UI gates.

Do not copy token values into documentation. Source is the token catalog; this document defines ownership and usage rules.
