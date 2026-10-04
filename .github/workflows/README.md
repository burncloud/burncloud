Workflow documentation moved to ../README.md.

The overview, trigger coverage, known-failing checks, conventions and the list of planned changes now live
in `.github/README.md`; a second copy here would be a second source of truth. The matrix of which crate is
verified by which job is in `../test-plan/coverage-matrix.md`.

What follows is only what that document does **not** carry: the naming scheme for the files in this
directory, and two decisions that were measured here.

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

## Why formatting has its own workflow

Formatting lives in `ci-quality.yml` rather than in `ci-tests.yml` on purpose. It used to be the first
job of the test workflow with every other job declaring `needs: formatting`, so a formatting failure
stopped the Billing, Security, Node and contract suites from reporting at all -- one early failure
suppressing five verdicts. The two workflows now report separately, and "all of them must pass" belongs
in the `CI Required` aggregate rather than in a `needs` chain.

## What `actionlint` cannot check here

`actionlint` checks syntax, expressions and deprecated action versions, and the `paths` filters can be
evaluated against `git ls-files` to catch an entry that matches nothing. Neither can check the trigger
decision itself, required-check aggregation, branch protection, `merge_group` semantics, events caused
by `GITHUB_TOKEN`, the runner image, caching, or secrets. For a workflow change the authoritative check
is a real run on GitHub.

One caveat on the path check: it reads every quoted entry under a `paths:` key, so a tag filter such as
`v*` in `cd-release.yml` is reported as matching no tracked file. That is a limitation of the check, not
a defect in the workflow.
