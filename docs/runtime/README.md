---
doc_id: runtime.index
doc_type: runtime-navigation
truth: informational
status: active
audited_against: c7107382b8479deb44f992e9e5ae8dcac5efb417
---

# Runtime Flow Navigation

The human-facing progressive Runtime Flow & ICFG Atlas is currently rendered at:

**https://burncloud.github.io/**

It is organized from user action to source:

`User Action → End-to-End Flow → Drill-down ICFG → smaller ICFG → Source Evidence`.

## Agent usage

Use the Runtime Atlas to understand the execution path and discover source evidence. Before changing code:

1. open the current repository source linked by the flow;
2. confirm the relevant branch still exists;
3. inspect current tests;
4. classify dynamic dispatch/configuration honestly.

The website is a navigation/explanation layer, not authority over the current checkout.

## Long-term direction

Source-derived runtime Markdown should eventually be versioned in this repository so code and runtime explanation share a commit. The Docusaurus repository should become a renderer rather than an independent owner of behavioral truth.

That migration is intentionally not performed in this first docs-cleanup change; keeping the first PR small makes the truth model reviewable before moving the full Runtime Atlas.
