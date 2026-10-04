#!/usr/bin/env bash
# Exercise the real Rust installer and Git commits on Linux and Git for Windows.
# Only Cargo is stubbed: these tests verify gates, not workspace health.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/burncloud hook tests.XXXXXX")
trap 'cd "$root"; rm -rf "$scratch"' EXIT
windows=false
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) windows=true ;;
esac

source_file="$root/crates/interfaces/cli/src/cli/code.rs"
binary="$scratch/code-init"
: > "$scratch/gitconfig"
export GIT_CONFIG_GLOBAL="$scratch/gitconfig" GIT_CONFIG_NOSYSTEM=1
if "$windows"; then
    source_file=$(cygpath -m "$source_file")
    GIT_CONFIG_GLOBAL=$(cygpath -m "$GIT_CONFIG_GLOBAL")
    binary="$binary.exe"
fi
cat > "$scratch/main.rs" <<EOF
#[path = r####"$source_file"####] mod code;
fn main() -> std::io::Result<()> { code::init() }
EOF
rustc --edition=2021 -Dwarnings "$scratch/main.rs" -o "$binary"

cat > "$scratch/expected" <<'EOF'
fmt --all -- --check
test --workspace --no-default-features
clippy --workspace --all-targets --no-default-features
deny check
EOF
mkdir "$scratch/bin"
cat > "$scratch/bin/cargo" <<'EOF'
#!/bin/sh
if [ "${2-}" = --version ]; then
    [ "$1" != "${MISSING_TOOL-}" ]; exit $?
fi
printf '%s\n' "$*" >> "$CHECK_LOG"
[ "$1" != "${FAIL_CHECK-}" ]
EOF
chmod +x "$scratch/bin/cargo"
export PATH="$scratch/bin:$PATH"

fail() { echo "FAIL: $*" >&2; exit 1; }
expect_failure() {
    if "$@" > "$case_dir/output" 2>&1; then
        cat "$case_dir/output" >&2
        fail "command unexpectedly succeeded: $*"
    fi
}
expect_output() { grep -F -- "$1" "$case_dir/output" >/dev/null || fail "missing diagnostic: $1"; }
expect_absent() { [[ ! -e "$1" && ! -L "$1" ]] || fail "unexpected path: $1"; }
expect_text() { [[ $(cat "$1") == "$2" ]] || fail "unexpected contents: $1"; }
expect_log() { diff -u "$1" "$CHECK_LOG"; }
commit() { git commit -qm 'hook test'; }

case_number=0
begin_case() {
    case_number=$((case_number + 1))
    printf '[%s] %s\n' "$case_number" "$1"
    case_dir="$scratch/case $case_number"
    repo="$case_dir/repo with spaces"
    mkdir -p "$repo"
    cd "$repo"
    unset FAIL_CHECK MISSING_TOOL
    export CHECK_LOG="$case_dir/checks.log"
    git init -q
    git config core.autocrlf false
    git config user.name 'Hook Test'
    git config user.email 'hook-test@example.invalid'
    for relative in .github/hooks/pre-commit .github/scripts/pre-commit-checks.sh deny.toml Cargo.toml; do
        mkdir -p "$(dirname "$relative")"
        cp "$root/$relative" "$relative"
    done
    git add .
    hook="$repo/.git/hooks/pre-commit"
    backup="$hook.burncloud-original"
    pending="$hook.burncloud-pending"
}

begin_case 'install, repeat, and commit'
"$binary"
"$binary"
[[ -x "$hook" ]] || fail 'hook is not executable'
expect_absent "$backup"
expect_absent "$pending"
commit
expect_log "$scratch/expected"
expect_absent "$repo/.env"

begin_case 'each failed gate blocks commit and stops subsequent gates'
"$binary"
count=0
for check in fmt test clippy deny; do
    count=$((count + 1))
    : > "$CHECK_LOG"
    export FAIL_CHECK="$check"
    expect_failure commit
    head -n "$count" "$scratch/expected" > "$case_dir/expected"
    expect_log "$case_dir/expected"
    expect_failure git rev-parse --verify HEAD
done

begin_case 'missing tools block before checks'
"$binary"
for tool in fmt clippy deny; do
    export MISSING_TOOL="$tool"
    expect_failure commit
    expect_output 'Missing'
    expect_absent "$CHECK_LOG"
done

begin_case 'preserve original hook and propagate its failure'
cat > "$hook" <<'EOF'
#!/bin/sh
echo original >> "$CHECK_LOG"
exit 7
EOF
chmod +x "$hook"
cp "$hook" "$case_dir/original"
"$binary"
"$binary"
expect_failure commit
cat "$scratch/expected" > "$case_dir/expected"
echo original >> "$case_dir/expected"
expect_log "$case_dir/expected"
cmp "$backup" "$case_dir/original"

begin_case 'failed gate does not run original hook'
cat > "$hook" <<'EOF'
#!/bin/sh
echo original >> "$CHECK_LOG"
EOF
chmod +x "$hook"
"$binary"
export FAIL_CHECK=test
expect_failure commit
head -n 2 "$scratch/expected" > "$case_dir/expected"
expect_log "$case_dir/expected"

begin_case 'partial staging is rejected'
"$binary"
printf '\n# unstaged\n' >> Cargo.toml
expect_failure commit
expect_output 'stage or stash'
expect_absent "$CHECK_LOG"

begin_case 'untracked files are rejected'
"$binary"
echo '// not staged' > new.rs
expect_failure commit
expect_output 'untracked'
expect_absent "$CHECK_LOG"

begin_case 'custom hooks path is untouched'
git config core.hooksPath "$case_dir/shared hooks"
expect_failure "$binary"
expect_output 'core.hooksPath'
expect_absent "$hook"
expect_absent "$case_dir/shared hooks"

begin_case 'conflicting backup is untouched'
echo original > "$hook"
echo saved > "$backup"
expect_failure "$binary"
expect_text "$hook" original
expect_text "$backup" saved
expect_absent "$pending"

begin_case 'symlink is untouched'
echo original > "$case_dir/shared-hook"
# MSYS normally copies on ln -s. Request a real symlink, failing if not permitted.
if MSYS=winsymlinks:nativestrict ln -s "$case_dir/shared-hook" "$hook" 2> "$case_dir/symlink-error"; then
    [[ -L "$hook" ]] || fail 'ln did not create a real symlink'
    expect_failure "$binary"
    [[ -L "$hook" ]] || fail 'installer replaced the symlink'
    expect_text "$case_dir/shared-hook" original
elif "$windows"; then
    echo 'SKIP: real Windows symlink creation unavailable on this runner'
    cat "$case_dir/symlink-error"
else
    cat "$case_dir/symlink-error" >&2
    fail 'symlink setup failed'
fi

begin_case 'pending install is not overwritten'
echo original > "$hook"
echo 'in progress' > "$pending"
expect_failure "$binary"
expect_text "$pending" 'in progress'
expect_text "$hook" original

begin_case 'non-repository fails'
cd "$case_dir"
expect_failure "$binary"

begin_case 'other repository fails'
rm .github/scripts/pre-commit-checks.sh
expect_failure "$binary"
expect_absent "$hook"

begin_case 'linked worktree and subdirectory'
commit
git worktree add -q -b linked "$case_dir/linked worktree"
cd "$case_dir/linked worktree/.github"
"$binary"
[[ -f "$hook" ]] || fail 'worktree did not use common hooks directory'
git commit -q --allow-empty -m 'linked commit'
expect_log "$scratch/expected"

printf 'PASS: %s hook test scenarios (see any explicit SKIP above).\n' "$case_number"
