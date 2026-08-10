---
doc_id: ui.naming
doc_type: engineering-standard
truth: source-derived
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Console CSS Naming Rules

This page summarizes patterns currently rejected by `crates/loops/src/gates/css_naming.rs` for Console UI code.

## Spacing

Rejected patterns include:

- legacy short spacing names such as `gap-md`, `p-sm`, `mb-lg`;
- legacy `bc-gap-*`, `bc-pl-*`, etc.;
- Tailwind numeric spacing such as `gap-3`, `p-4`, `mb-2` (numeric zero exceptions are handled by the gate).

The current naming family uses `*-bc-*` spacing classes such as `gap-bc-*`.

## Color / border naming

The gate rejects:

- `border-[var(--bc-border)]` in favor of `border-bc-border`;
- shadcn-style classes such as `text-muted-foreground`, `bg-muted`, `text-foreground`, `bg-background`, `bg-card`, `border-border`;
- default Tailwind gray/slate/zinc/neutral/stone palette classes.

Use BurnCloud-prefixed semantic classes already defined by the current style system.

## Radius / shadow

The gate rejects default/arbitrary radius and shadow forms such as:

- `rounded-sm|md|lg|xl`,
- `shadow-sm`,
- arbitrary `rounded-[...]` / `shadow-[...]`.

Use the existing `rounded-bc-*` / `shadow-bc-*` families or component-specific CSS.

## Typography

The gate rejects several default/legacy/arbitrary text-size forms, including `text-2xl` through larger defaults, `text-base`, `text-lg`, `text-xxs`, `text-display`, and `text-[Npx]` patterns in the scanned Console code.

Use the typography classes already defined by the BurnCloud style system, such as the accepted `text-title`, `text-body`, `text-caption`, `text-large-title`, and `text-bc-*` families where appropriate.

## Scope

The executable gate excludes a small guest-page list and scans specific client source directories. Read `crates/loops/src/gates/css_naming.rs` for the exact current scope and regexes.

**Executable gate wins over this prose.**
