# Local development commands

Owner: Platform repository tooling. `burncloud-code` owns local hook installation,
changed-file discovery, test selection and execution. It does not import application
services, generate runtime secrets, modify source files, skip failing checks, or
replace GitHub branch protection. Interfaces/CLI delegates to this package.

## Commands

```sh
cargo run -- code init
cargo run -- code test
cargo run -- code test --plan
cargo run -- code test --base origin/main --plan
cargo run -- code test --all
cargo run -- code test --staged
cargo run -- code test --last
cargo run -- code clippy
cargo run -- code clippy --plan
cargo run -- code clippy --base origin/main
cargo run -- code clippy --all
```

`code init` checks for rustfmt, Clippy and cargo-deny. It installs missing Rust
components with `rustup component add` and missing cargo-deny with
`cargo install --locked cargo-deny`. It verifies each command before activating
the Git hooks. Repeating `code init` does not reinstall available tools.

The `code test` command owns test selection and test execution only. `code clippy`
owns Clippy package selection/execution for the self-hosted PR gate. Neither command
installs quality tools; `code init` remains responsible for installing them because
the managed pre-commit hook runs its four gates independently.

The lightweight equivalent, useful when application compilation is broken, is
`cargo run -p burncloud-code -- test` (or `-- init`). Both binaries use the same
command definition and implementation. Tests are `cargo test -p burncloud-code`.

## Selection contract

| Input | Result |
| --- | --- |
| Default | Tracked changes versus HEAD plus non-ignored untracked paths; handles an unborn HEAD |
| `--staged` | Index changes; refuses unstaged tracked changes and non-ignored untracked paths |
| `--base REF` | Changes since the merge base of REF and HEAD, plus local changes; invalid refs fail |
| `--all` | All workspace packages, even in a clean checkout |
| `--plan` | Print changed paths, selection reason, affected packages and commands; execute no checks |
| `--last` | Show the latest local status, per-check logs and parsed test counts |
| Source, assets or test data in a package | Owning package plus every transitive workspace consumer |
| Root application `src/` or `crates/interfaces/cli/` | Root application and its consumers |
| Root `Cargo.toml`/`Cargo.lock`, `.cargo/**`, Rust toolchain files | Whole workspace |
| A package's own `Cargo.toml` | Owning package plus every transitive workspace consumer |
| `.github/**`, `clippy.toml`, `deny.toml` | No test impact; ignored for test selection |
| `clippy.toml` for `code clippy` | Whole workspace |
| `.github/**`, `deny.toml` for `code clippy` | No Clippy impact |
| Unknown path | Whole workspace, with the path printed as the reason |
| Only root README/license or `.github/README.md` | No checks, explicitly reported |
| No changes | No checks, explicitly reported; use `--all` or `--base REF` |

Ownership uses the longest package-directory prefix from `cargo metadata --no-deps`.
The reverse graph uses workspace dependency paths, including renamed, dev, build,
optional and target-specific dependencies, without restricting it to the current
host. Cargo metadata failures stop execution. Deleted files retain their former
package ownership; renames are treated as deletion plus addition, so both ends
contribute. NUL-delimited Git output preserves paths containing spaces/newlines.
Non-UTF-8 paths fail explicitly rather than silently disappearing from the plan.
Other documentation (including architecture contracts and test plans) is not
excluded: unknown shared documentation forces the whole workspace.

## Execution contract

For selected code changes, `code test` runs:

1. `cargo test -p <affected> ... --no-default-features` (includes package integration/doc tests)

For selected code changes, `code clippy` runs:

1. `cargo clippy -p <affected> ... --all-targets --no-default-features -- -D warnings`

Both commands use the same reverse workspace dependency closure, so changing crate A
selects A plus every workspace package that consumes A directly or transitively.
Root Cargo/build/toolchain configuration expands to the whole workspace; `clippy.toml`
also forces a full Clippy run. The managed pre-commit hook intentionally remains
broader: workspace fmt, affected `code test --staged`, strict workspace Clippy, then
`cargo deny check`. The self-hosted PR Clippy gate uses `code clippy --base` instead
to avoid linting unrelated packages on every PR.

Existing ignores, external test prerequisites and feature policies remain in effect.
These commands do not claim to cover every feature combination, UI convention script
or operating system. Dynamic relationships not represented in Cargo remain a reason
to run `--all` and retain the existing full-workspace/release suites.

`code test` does not own Cargo target-cache housekeeping. Trusted self-hosted
GitHub Actions use the shared `~/.cache/burncloud/target` directory and perform
the 100 GiB size check as the final workflow step, after the Rust check has
finished. Local `code test` runs therefore never delete the developer's target
directory as a side effect.

Each executed check writes its full output under `.git/burncloud/checks/<run>/`,
alongside `summary.json`; `latest.json` tracks the most recent result. The terminal
shows ✅/❌, log locations, and unit/integration/doc test counts. A count is marked
unavailable when Cargo emitted no parseable summary. A documentation-only `code test` run records the test step as `SKIP`; the surrounding pre-commit fmt, Clippy and deny gates still run independently.

The installed pre-commit hook runs formatting, `cargo run --quiet -- code test --staged`,
strict workspace Clippy and cargo-deny in that order, then the preserved original hook. The commit-msg hook runs the preserved original hook
and then stamps the message with check status, test counts and the staged tree hash.
It rejects a changed index or HEAD after checks. On `--amend`, the previous receipt
is replaced. Full logs remain local; commit messages carry only the compact result.
Git hooks can be bypassed with `--no-verify`, so remote CI is still authoritative.
Initialization is idempotent, upgrades the previous
exact managed wrapper, and refuses custom `core.hooksPath`, symlinks, conflicting
backups and concurrent setup. Linux uses Git's system shell and Windows uses Git
for Windows. Selection/execution and regression tests are native Rust, with no
Python or Bash test harness. Regression tests use a native mock Cargo executable
and real Git repositories; their success is not evidence of workspace health.
