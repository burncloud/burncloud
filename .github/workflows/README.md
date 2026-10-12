# GitHub Actions workflows

- PR Rust gates: `ci-github-hosted-fmt.yml`, `ci-github-hosted-test.yml`,
  `ci-github-hosted-clippy.yml`, and `ci-github-hosted-deny.yml`.
  They share `ci-github-hosted-base.yml` and check the PR merge commit.
- Architecture-only manual checks: `ci-architecture.yml`.
- Release tagging: `maintenance-version-tag.yml` performs **full-workspace**
  Fmt, Test, Clippy and Deny validation before creating a new version tag.
- Release packaging and publishing: `cd-release.yml`.

The former `ci-quality.yml`, `ci-integration.yml` and `ci-client.yml`
workflows were intentionally retired. The dedicated real-PostgreSQL integration
job and manual multi-platform client checks are no longer provided by those
workflows; do not assume the PR Rust checks replace that coverage.

To change shared PR checks, edit `ci-github-hosted-base.yml`. To change the
release-only full-workspace gate, edit `maintenance-version-tag.yml`.
