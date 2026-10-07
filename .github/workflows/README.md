Workflow documentation lives in `../README.md`.

This directory has three CI layers:

- PR gates: `ci-self-hosted-fmt.yml`, `ci-self-hosted-test.yml`,
  `ci-self-hosted-clippy.yml`, and `ci-self-hosted-deny.yml`.
- Shared PR execution: `ci-self-hosted-rust.yml` owns self-hosted authorization, runner/bootstrap
  logic, and the four fixed PR commands.
- Full release gate: `ci-quality.yml` validates the whole workspace before version tagging.
- Specialist checks: `ci-architecture.yml`, `ci-client.yml`, and `ci-integration.yml` cover
  architecture rules, platform client builds, and real PostgreSQL respectively.

Do not duplicate generic fmt/test/Clippy/deny policy into specialist workflows. Change shared PR runner
policy in `ci-self-hosted-rust.yml`; change full-workspace release policy in `ci-quality.yml`.
