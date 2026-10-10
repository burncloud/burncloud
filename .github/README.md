# GitHub CI required-check policy

## Required checks for main

The intended sole required status check is **`CI Required`**, produced by
`.github/workflows/ci-required.yml` on pull requests. It is an aggregate of
`Fmt`, `Test`, `Clippy`, and `Deny`, which all must succeed. A failure,
cancellation, timeout, unexpected skip or missing result blocks a successful
aggregate. Deny can explicitly skip its scan step when no dependency-policy
files changed; the overall Deny job must still succeed.

**Repository enforcement is NOT established by this file.** Before enabling
the rule, verify all tests are stable, and check the relevant prerequisites in
#637 (#633, #634, #635). Enable a GitHub repository/organization ruleset:
- Target `main` and set enforcement to **Active**.
- Require pull requests and status check `CI Required` from GitHub Actions.
- No bypass actors (including admins, organization owners, bots and apps).
- Block direct pushes and force pushes to `main` as applicable.
- If using branch protection instead: enable enforcement for administrators
  and disallow bypass.
- Verify an intentionally failed PR cannot be merged by a normal contributor,
  a repository Admin, or an organization Owner without first changing the rule.
- Keep the ruleset identifier, test PR links and administrative verification
  results here after setup; do not claim #637 is complete until verified.

A user who can administer repository or organization rules can still edit or
delete the rules themselves; this is distinct from bypassing an active rule.
