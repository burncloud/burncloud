# CI and repository maintenance

Every workflow in this directory, what triggers it, and what it actually runs. Kept short on purpose:
the authoritative list is the workflow files themselves, and this file exists so a reader does not have
to open seven YAML files to find out which check ran on their PR.

## Workflows

| File | Display name | Triggers | What it runs |
| --- | --- | --- | --- |
| `ci-quality.yml` | `CI / Quality` | PR and push to `main`, on any Rust file, `Cargo.toml` or `clippy.toml` | `rustfmt --check` on the files the change touched |
| `ci-architecture.yml` | `CI / Architecture` | PR and push to `main`, on Rust manifests, `clippy.toml`, `deny.toml` | `clippy` across the workspace, `cargo-deny` policy, and the router dependency whitelist |
| `ci-client.yml` | `CI / Client` | PR, on the client crates and the root manifest | UI convention checks, LiveView check, Windows and macOS desktop checks |
| `ci-tests.yml` | `CI / Tests` | PR and push to `main`, on the crates whose behaviour the suites cover | Node invariants, billing invariants, security invariants, migration contracts |
| `cd-release.yml` | `CD / Release` | Push to `main` and `workflow_dispatch`; also `v*` tags | Builds the release artifacts and creates the GitHub release |
| `maintenance-version-tag.yml` | `Maintenance / Version Tag` | Push to `main` on manifests | Compares the root package version with the newest tag and creates the tag when the version moved forward |
| `maintenance-sync-gitee.yml` | `Maintenance / Sync to Gitee` | Push to `main` and `workflow_dispatch` | Mirrors the repository to Gitee |

## Naming

`ci-*` are checks, `cd-*` is the release, `maintenance-*` is repository upkeep. Display names use the
stable `CI / ...` form so a check can be identified on a PR page without knowing the file name. The
aggregating required check, when one exists, is named `CI Required` and carries no branch name,
timestamp or matrix value.

Job ids use domain names (`contracts`, `identity`, `supply`, `commerce`, `traffic`, `platform`,
`server`, `client`, `quality`) rather than file names.

## Dependency reproducibility: `Cargo.lock` is not tracked

Stated here because a CI document that omits it invites the claim that builds are reproducible, and they
are not.

Measured:

```
$ git ls-files Cargo.lock                       -> (no output: not tracked)
$ grep -n Cargo.lock .gitignore                 -> 29:Cargo.lock
$ grep -n -- --locked .github/workflows/*.yml   -> (no matches)
```

Consequences, all of which follow from that one fact:

- **Dependency versions drift.** Cargo resolves newest-compatible versions at build time, so two runs of
  the same commit can build against different dependency versions. A build that worked can break with no
  change to this repository.
- **`--locked` cannot be used**, which is why no workflow passes it: the flag requires the file.
- **`cargo-deny` and `clippy` results are not pinned to a dependency set** either.

**The decision, recorded rather than left implicit.** Committing `Cargo.lock` is the conventional choice
for a workspace that ships binaries, and it was considered. It is **not done here** for two reasons:

1. It requires editing `.gitignore`, which the project's instructions list as a file not to modify
   without an explicit decision. Removing one line is a small change for whoever owns that file.
2. It changes what every future build resolves -- repository-wide behaviour that deserves its own change
   and its own verification rather than being folded into a CI document.

**What would be verified before committing it:**

- `cargo build --workspace --locked` succeeds from a clean checkout on the committed file.
- `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked` behave as they
  do without the flag.
- The file is regenerated whenever a manifest changes. Otherwise the next CI run fails with a lock-file
  mismatch rather than a code error -- worth naming, because that is the cost of the choice.

Until then, no part of this project should claim reproducible builds or pinned dependency versions.
Formatting lives in `ci-quality.yml` rather than in `ci-tests.yml` on purpose. It used to be the first
job of the test workflow with every other job declaring `needs: formatting`, so a formatting failure
stopped the Billing, Security, Node and contract suites from reporting at all -- one early failure
suppressing five verdicts. The two workflows now report separately, and "all of them must pass" belongs
in the `CI Required` aggregate rather than in a `needs` chain.

## Two rules that are not optional

Both were learned by getting them wrong.

**Check the real diff before opening a PR.** Run `git diff --name-only main...HEAD` and verify every
path is inside the issue's Allowed Paths. A clean description is not evidence of a clean diff: two
branches in this repository carried 7 and 9 files where the issue allowed 2, because they were created
while `HEAD` pointed at another feature branch.

**A test must never freeze a defect as the contract.** When a test reveals wrong behaviour, the expected
value is not the current output. Writing the observed (wrong) value into the assertion declares it
correct and makes the eventual fix look like a regression. Instead: file a bug issue that states the
decision to be made, leave a failing test that records the defect and references the issue, then fix and
remove the `ignore`.

## Known limits of local verification

`actionlint` checks syntax, expressions and deprecated action versions, and the `paths` filters can be
evaluated against `git ls-files` to catch an entry that matches nothing. Neither can check the trigger
decision itself, required-check aggregation, branch protection, `merge_group` semantics, events caused
by `GITHUB_TOKEN`, the runner image, caching, or secrets. For a workflow change the authoritative check
is a real run on GitHub.

One caveat on the path check: it reads every quoted entry under a `paths:` key, so a tag filter such as
`v*` in `cd-release.yml` is reported as matching no tracked file. That is a limitation of the check, not
a defect in the workflow.

## Adding a workflow

- Prefer a step in an existing workflow over a new file; each file is another trigger to reason about.
- Give every job `timeout-minutes` and a domain `name`.
- If a workflow is triggered by `pull_request`, remember that the version used is the one on the base
  branch, so a new job cannot be verified before it is merged.
- Add the workflow's own path to its `paths` filter so a change to the workflow re-runs it.
