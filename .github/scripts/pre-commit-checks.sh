#!/bin/sh
# Shared, version-controlled local commit gate. No failures may be skipped.
set -eu
cd "$(git rev-parse --show-toplevel)"

# Cargo checks the working tree. Do not certify a different staged snapshot.
if ! git diff --quiet --ignore-submodules=none; then
    echo "Commit blocked: stage or stash all tracked changes before running the checks." >&2
    exit 1
fi
if [ -n "$(git ls-files --others --exclude-standard)" ]; then
    echo "Commit blocked: stage or stash untracked files before running the checks." >&2
    exit 1
fi

if ! cargo fmt --version >/dev/null 2>&1; then
    echo "Missing rustfmt. Run: rustup component add rustfmt" >&2
    exit 1
fi
if ! cargo clippy --version >/dev/null 2>&1; then
    echo "Missing Clippy. Run: rustup component add clippy" >&2
    exit 1
fi
if ! cargo deny --version >/dev/null 2>&1; then
    echo "Missing cargo-deny. Run: cargo install cargo-deny --locked" >&2
    exit 1
fi

echo "[1/4] cargo fmt --all -- --check"
cargo fmt --all -- --check
echo "[2/4] cargo test --workspace --no-default-features"
cargo test --workspace --no-default-features
echo "[3/4] cargo clippy --workspace --all-targets --no-default-features"
cargo clippy --workspace --all-targets --no-default-features
echo "[4/4] cargo deny check"
cargo deny check
