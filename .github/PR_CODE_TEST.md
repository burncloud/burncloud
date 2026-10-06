# Self-hosted pull-request code test

This is the pull-request gate for machines where GitHub Actions must not be used.
It polls GitHub from a BurnCloud-owned Windows machine and executes the repository's
own `code test` command locally.

## Contract

For every open pull request head SHA that has not already reached a terminal test
result, the runner:

1. fetches `origin/main`;
2. fetches `refs/pull/<number>/head`;
3. creates a detached temporary Git worktree for the exact PR head SHA;
4. publishes the `burncloud/code-test` commit status as `pending`;
5. runs `cargo run -- code test --base origin/main` inside that worktree;
6. publishes `success` or `failure` back to the same PR head SHA;
7. removes the temporary worktree;
8. saves the local log and result under `.git/burncloud/pr-code-test/`.

A successful or failed SHA is tested only once. Pushing another commit to the PR
changes its head SHA and therefore schedules a fresh test. A runner/infrastructure
error is not terminal and will be retried on the next poll.

The runner deliberately uses `--base origin/main`. A clean checkout of a PR has no
working-tree changes, so plain `cargo run -- code test` would correctly report
nothing to test. `--base` makes `code test` evaluate the PR's changes relative to
`main` while retaining the normal affected-package selection, formatting, tests,
Clippy and `cargo deny` behavior.

No `.github/workflows` file is involved and GitHub Actions is not used.

## Windows installation

Prerequisites:

- Git
- Rust/Cargo and rustup
- GitHub CLI (`gh`) authenticated to an account that can read the repository and
  publish commit statuses

From the repository root, run:

```powershell
powershell -ExecutionPolicy Bypass -File .github/scripts/install-pr-code-test-task.ps1
```

The installer verifies GitHub authentication, installs/verifies rustfmt and Clippy,
installs `cargo-deny` when missing, and registers the Windows scheduled task
`BurnCloud PR Code Test`. The default polling interval is one minute. Windows Task
Scheduler is configured with `IgnoreNew`, while the runner also owns an exclusive
lock, so a long `code test` cannot overlap another invocation.

To use a different interval:

```powershell
powershell -ExecutionPolicy Bypass -File .github/scripts/install-pr-code-test-task.ps1 -IntervalMinutes 5
```

To test the runner once without waiting for the scheduler:

```powershell
powershell -ExecutionPolicy Bypass -File .github/scripts/pr-code-test.ps1
```

To remove the scheduled task:

```powershell
Unregister-ScheduledTask -TaskName "BurnCloud PR Code Test" -Confirm:$false
```

## Local state

The runner keeps machine-local state only:

```text
.git/burncloud/pr-code-test/
├── logs/
│   └── pr-<number>-<sha>.log
├── results/
│   └── <full-sha>.json
├── worktrees/
└── runner.lock
```

Deleting one result JSON intentionally makes that exact SHA eligible to run again.
Deleting the entire directory causes all currently open PR heads to be tested again.
