#!/usr/bin/env python3
"""POSIX black-box tests of the actual Rust installer and Git commit hooks.

Only Cargo commands are stubbed: these tests verify gating, not workspace health.
Run with Python 3, rustc and Git on PATH; no third-party Python dependencies.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CHECKS = [
    "fmt --all -- --check",
    "test --workspace --no-default-features",
    "clippy --workspace --all-targets --no-default-features",
    "deny check",
]


class CodeInitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build = tempfile.TemporaryDirectory(prefix="burncloud-init-build-")
        build = Path(cls.build.name)
        source = ROOT / "crates/interfaces/cli/src/cli/code.rs"
        harness = build / "main.rs"
        harness.write_text(
            f"#[path = {json.dumps(str(source))}] mod code;\n"
            "fn main() -> std::io::Result<()> { code::init() }\n"
        )
        cls.binary = build / "code-init"
        subprocess.run(
            ["rustc", "--edition=2021", "-Dwarnings", str(harness), "-o", str(cls.binary)],
            check=True,
        )

    @classmethod
    def tearDownClass(cls):
        cls.build.cleanup()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="burncloud hook test ")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.repo = self.base / "repo with spaces"
        self.repo.mkdir()
        self.env = os.environ.copy()
        # Isolate test Git configuration from the developer's global hooks.
        self.env.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")
        self.log = self.base / "checks.log"
        self.env["CHECK_LOG"] = str(self.log)
        mock_bin = self.base / "bin"
        mock_bin.mkdir()
        cargo = mock_bin / "cargo"
        cargo.write_text(
            '#!/bin/sh\n'
            'if [ "${2-}" = --version ]; then\n'
            '  [ "$1" != "${MISSING_TOOL-}" ]; exit $?\n'
            'fi\n'
            'printf "%s\\n" "$*" >> "$CHECK_LOG"\n'
            '[ "$1" != "${FAIL_CHECK-}" ]\n'
        )
        cargo.chmod(0o755)
        self.env["PATH"] = str(mock_bin) + os.pathsep + self.env["PATH"]
        self.run_cmd("git", "init")
        self.run_cmd("git", "config", "user.name", "Hook Test")
        self.run_cmd("git", "config", "user.email", "hook-test@example.invalid")
        for relative in (".github/hooks/pre-commit", ".github/scripts/pre-commit-checks.sh", "deny.toml", "Cargo.toml"):
            target = self.repo / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)
        self.run_cmd("git", "add", ".")
        self.hook = self.repo / ".git/hooks/pre-commit"

    def run_cmd(self, *args, ok=True, cwd=None):
        result = subprocess.run(args, cwd=cwd or self.repo, env=self.env, text=True, capture_output=True)
        if ok:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def init(self, **kwargs):
        return self.run_cmd(str(self.binary), **kwargs)

    def commit(self, **kwargs):
        return self.run_cmd("git", "commit", "-m", "test commit", **kwargs)

    def test_install_repeat_and_commit(self):
        self.init()
        self.init()
        self.assertTrue(os.access(self.hook, os.X_OK))
        self.assertFalse(Path(str(self.hook) + ".burncloud-original").exists())
        self.commit()
        self.assertEqual(self.log.read_text().splitlines(), CHECKS)
        self.assertFalse((self.repo / ".env").exists())

    def test_each_failed_check_blocks_commit_and_stops(self):
        self.init()
        for index, check in enumerate(CHECKS):
            with self.subTest(check=check):
                self.log.write_text("")
                self.env["FAIL_CHECK"] = check.split()[0]
                self.commit(ok=False)
                self.assertEqual(self.log.read_text().splitlines(), CHECKS[:index + 1])
                self.run_cmd("git", "rev-parse", "--verify", "HEAD", ok=False)

    def test_missing_tools_block_before_checks(self):
        self.init()
        for tool in ("fmt", "clippy", "deny"):
            with self.subTest(tool=tool):
                self.env["MISSING_TOOL"] = tool
                result = self.commit(ok=False)
                self.assertIn("Missing", result.stderr)
                self.assertFalse(self.log.exists())

    def test_preserves_original_hook_and_its_failure(self):
        original = '#!/bin/sh\necho original >> "$CHECK_LOG"\nexit 7\n'
        self.hook.write_text(original)
        self.hook.chmod(0o755)
        self.init()
        self.init()
        self.commit(ok=False)
        self.assertEqual(self.log.read_text().splitlines(), CHECKS + ["original"])
        self.assertEqual(Path(str(self.hook) + ".burncloud-original").read_text(), original)

    def test_failed_gate_does_not_run_original(self):
        self.hook.write_text('#!/bin/sh\necho original >> "$CHECK_LOG"\n')
        self.hook.chmod(0o755)
        self.init()
        self.env["FAIL_CHECK"] = "test"
        self.commit(ok=False)
        self.assertEqual(self.log.read_text().splitlines(), CHECKS[:2])

    def test_partial_staging_is_rejected(self):
        self.init()
        with (self.repo / "Cargo.toml").open("a") as stream:
            stream.write("\n# unstaged\n")
        self.assertIn("stage or stash", self.commit(ok=False).stderr)
        self.assertFalse(self.log.exists())

    def test_untracked_files_are_rejected(self):
        self.init()
        (self.repo / "new.rs").write_text("// not staged\n")
        self.assertIn("untracked", self.commit(ok=False).stderr)
        self.assertFalse(self.log.exists())

    def test_custom_hooks_path_is_untouched(self):
        self.run_cmd("git", "config", "core.hooksPath", str(self.base / "shared hooks"))
        self.assertIn("core.hooksPath", self.init(ok=False).stderr)
        self.assertFalse(self.hook.exists())
        self.assertFalse((self.base / "shared hooks").exists())

    def test_conflicting_backup_is_untouched(self):
        self.hook.write_text("original")
        backup = Path(str(self.hook) + ".burncloud-original")
        backup.write_text("saved")
        self.init(ok=False)
        self.assertEqual(self.hook.read_text(), "original")
        self.assertEqual(backup.read_text(), "saved")

    def test_symlink_is_untouched(self):
        target = self.base / "shared-hook"
        target.write_text("original")
        self.hook.symlink_to(target)
        self.init(ok=False)
        self.assertTrue(self.hook.is_symlink())
        self.assertEqual(target.read_text(), "original")

    def test_pending_install_is_not_overwritten(self):
        self.hook.write_text("original")
        pending = Path(str(self.hook) + ".burncloud-pending")
        pending.write_text("in progress")
        self.init(ok=False)
        self.assertEqual(pending.read_text(), "in progress")
        self.assertEqual(self.hook.read_text(), "original")

    def test_non_repository_fails(self):
        self.init(cwd=self.base, ok=False)

    def test_other_repository_fails(self):
        (self.repo / ".github/scripts/pre-commit-checks.sh").unlink()
        self.init(ok=False)
        self.assertFalse(self.hook.exists())

    def test_linked_worktree_and_subdirectory(self):
        self.commit()  # Seed HEAD before creating the linked worktree.
        worktree = self.base / "linked worktree"
        self.run_cmd("git", "worktree", "add", "-b", "linked", str(worktree))
        self.init(cwd=worktree / ".github")
        self.assertTrue(self.hook.is_file())
        self.run_cmd("git", "commit", "--allow-empty", "-m", "linked commit", cwd=worktree)
        self.assertEqual(self.log.read_text().splitlines(), CHECKS)


if __name__ == "__main__":
    unittest.main(verbosity=2)
