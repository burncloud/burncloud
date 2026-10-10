# GitHub CI required checks

## Required checks for main

Four independent GitHub Actions PR workflows run concurrently when runners
are available. Configure ALL FOUR actual check-run names as required in the
GitHub Ruleset for `main`:

- `check / Cargo Fmt` (workflow: `ci-github-hosted-fmt.yml`)
- `check / Code Test` (workflow: `ci-github-hosted-test.yml`)
- `check / Cargo Clippy` (workflow: `ci-github-hosted-clippy.yml`)
- `check / Cargo Deny` (workflow: `ci-github-hosted-deny.yml`)

**Exact names verified** against GitHub Actions job API on PR #856, commit
`081310397ceb624ad1b1b9939539067e7b26fb8e` (2026-10-10): all four jobs
returned `success` in their independent workflows. Examples:
- Fmt: https://github.com/burncloud/burncloud/actions/runs/38050465892
- Test: https://github.com/burncloud/burncloud/actions/runs/38050465894
- Clippy: https://github.com/burncloud/burncloud/actions/runs/38050465903
- Deny: https://github.com/burncloud/burncloud/actions/runs/38050465889

Only configure the verified exact names. Revalidate after future renames.
All four checks must be present and successful for the latest PR revision.
The Deny job may finish successfully without running the cargo-deny step when
no dependency-policy files change; do not require that optional step separately.

## Administrator-proof enforcement

This documentation does not activate repository rules. After #637's hard
prerequisites (#633, #634, #635) and CI stability verification:
1. Use an **active GitHub Ruleset** targeting only `main`.
2. Require a pull request and all four exact status checks, pinned to the
   trusted GitHub Actions app as source when possible.
3. Do not assign any bypass actors: no Owner, Admin, team, bot, or GitHub App.
4. Block unauthorized direct pushes and force pushes.
5. Verify a deliberately failing PR is blocked for contributors AND repository
   Admin / organization Owner. Record links, Ruleset ID and test evidence here
   before considering #637 complete.

Repository/org rule administrators can still **edit the rule itself**. This
is separate from bypassing an active rule. GitHub connector authorization
may not permit configuring branch protection: document rather than claim it
is active unless confirmed via GitHub.
