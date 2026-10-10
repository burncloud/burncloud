# GitHub Actions workflows

- Main PR gate: `ci-required.yml` runs four independent Rust checks from
  `ci-github-hosted-base.yml` and aggregates their conclusions into the
  stable `CI Required` check. All four jobs must succeed. A failed, cancelled,
  skipped, or unavailable job fails the gate.
- `Cargo Deny` may explicitly skip its *step* when no dependency-policy files
  changed, but the Deny job must still finish successfully.
- Architecture-only manual checks: `ci-architecture.yml`.
- Release tagging: `maintenance-version-tag.yml` performs full-workspace
  Fmt, Test, Clippy and Deny validation before creating a new version tag.
- Release packaging and publishing: `cd-release.yml`.

The four former independent `ci-github-hosted-{fmt,test,clippy,deny}.yml`
workflows have been replaced by the single PR entry point to avoid executing
the expensive checks twice. It preserves the reusable check implementation.
The dedicated real-PostgreSQL integration job and manual multi-platform client
checks are not supplied by this PR gate.

To change PR checks, edit `ci-github-hosted-base.yml`. Keep the GitHub status
check name `CI Required` stable once it is made mandatory.
