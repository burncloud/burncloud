#!/usr/bin/env python3
"""Run BurnCloud staging + browser audit locally, without GitHub Actions.

This is the fast iteration entrypoint for ChatGPT/Codex UI work:

    BurnCloud binary -> isolated SQLite -> /health -> agent-browser -> screenshots/report

The runner never touches the default BurnCloud database. Every run receives its own
SQLite file through BURNCLOUD_DATABASE_URL and the server is terminated afterwards.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
AUDIT_SCRIPT = Path(__file__).with_name("staging_browser_audit.py")
DEFAULT_MASTER_KEY = "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Start an isolated local BurnCloud staging server and run the real browser audit."
    )
    parser.add_argument("--port", type=int, default=3000)
    parser.add_argument(
        "--build",
        choices=("auto", "always", "never"),
        default="auto",
        help="auto builds only when the binary is missing or Rust/Cargo sources are newer",
    )
    parser.add_argument(
        "--browser-args",
        default=os.environ.get("AGENT_BROWSER_ARGS", "--headless=new,--no-sandbox"),
        help="arguments forwarded to agent-browser through AGENT_BROWSER_ARGS",
    )
    parser.add_argument(
        "--keep-runtime",
        action="store_true",
        help="keep the isolated SQLite/runtime directory after the run for debugging",
    )
    parser.add_argument(
        "--keep-server",
        action="store_true",
        help="leave the local staging server running after the audit",
    )
    return parser.parse_args()


def burncloud_binary() -> Path:
    configured = os.environ.get("BURNCLOUD_BIN")
    if configured:
        return Path(configured).expanduser().resolve()
    suffix = ".exe" if os.name == "nt" else ""
    return ROOT / "target" / "debug" / f"burncloud{suffix}"


def newest_source_mtime() -> float:
    newest = 0.0
    roots = [ROOT / "src", ROOT / "crates"]
    for base in roots:
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            try:
                newest = max(newest, path.stat().st_mtime)
            except OSError:
                pass
    for path in (ROOT / "Cargo.toml", ROOT / "Cargo.lock"):
        if path.exists():
            newest = max(newest, path.stat().st_mtime)
    for path in ROOT.rglob("Cargo.toml"):
        try:
            newest = max(newest, path.stat().st_mtime)
        except OSError:
            pass
    return newest


def should_build(binary: Path, mode: str) -> bool:
    if mode == "always":
        return True
    if mode == "never":
        if not binary.exists():
            raise RuntimeError(f"BurnCloud binary does not exist: {binary}")
        return False
    if not binary.exists():
        return True
    return newest_source_mtime() > binary.stat().st_mtime


def build(binary: Path, mode: str) -> None:
    if not should_build(binary, mode):
        print(f"[staging] reuse binary: {binary}")
        return
    print("[staging] cargo build --bin burncloud")
    subprocess.run(["cargo", "build", "--bin", "burncloud"], cwd=ROOT, check=True)
    if not binary.exists():
        raise RuntimeError(f"cargo build completed but binary is missing: {binary}")


def sqlite_url(path: Path) -> str:
    normalized = path.resolve().as_posix()
    if os.name == "nt":
        return f"sqlite:///{normalized}?mode=rwc"
    return f"sqlite://{normalized}?mode=rwc"


def wait_health(base_url: str, process: subprocess.Popen[bytes], timeout: float = 45.0) -> None:
    deadline = time.monotonic() + timeout
    last_error = ""
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"BurnCloud exited before /health became ready (code={process.returncode})")
        try:
            with urllib.request.urlopen(f"{base_url}/health", timeout=1.0) as response:
                if 200 <= response.status < 300:
                    return
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            last_error = str(exc)
        time.sleep(0.2)
    raise RuntimeError(f"timeout waiting for {base_url}/health: {last_error}")


def stop_process(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def main() -> int:
    args = parse_args()
    binary = burncloud_binary()

    if shutil.which("agent-browser") is None:
        print(
            "agent-browser is not installed. Install it once, then rerun:\n"
            "  npm install -g agent-browser\n"
            "  agent-browser install",
            file=sys.stderr,
        )
        return 2

    build(binary, args.build)

    audit_dir = ROOT / "target" / "staging-audit"
    if audit_dir.exists():
        shutil.rmtree(audit_dir)
    (audit_dir / "screenshots").mkdir(parents=True, exist_ok=True)

    runtime_dir = ROOT / "target" / "staging-runtime" / f"local-{os.getpid()}"
    runtime_dir.mkdir(parents=True, exist_ok=True)
    database_path = runtime_dir / "burncloud-staging.db"
    server_log_path = audit_dir / "server.log"
    base_url = f"http://127.0.0.1:{args.port}"

    env = os.environ.copy()
    env.update(
        {
            "HOST": "127.0.0.1",
            "PORT": str(args.port),
            "E2E_BASE_URL": base_url,
            "BURNCLOUD_DATABASE_URL": sqlite_url(database_path),
            "MASTER_KEY": env.get("MASTER_KEY", DEFAULT_MASTER_KEY),
            "PRICE_SYNC_INTERVAL_SECS": "999999",
            "SKIP_INITIAL_PRICE_SYNC": "1",
            "NO_PROXY": "*",
            "STAGING_AUDIT_DIR": str(audit_dir),
            "AGENT_BROWSER_ARGS": args.browser_args,
        }
    )
    env.pop("BURNCLOUD_FRESH_DB", None)

    print(f"[staging] database: {database_path}")
    print(f"[staging] url:      {base_url}")
    print(f"[staging] evidence: {audit_dir}")

    server_log = server_log_path.open("wb")
    process = subprocess.Popen(
        [str(binary), "server"],
        cwd=ROOT,
        env=env,
        stdout=server_log,
        stderr=subprocess.STDOUT,
    )

    audit_code = 1
    try:
        wait_health(base_url, process)
        print("[staging] /health ready")
        print("[staging] running real browser journey")
        audit = subprocess.run([sys.executable, str(AUDIT_SCRIPT)], cwd=ROOT, env=env, check=False)
        audit_code = audit.returncode
        if audit_code == 0:
            print(f"[staging] PASS -> {audit_dir / 'report.md'}")
        else:
            print(f"[staging] FAIL -> {audit_dir / 'failure.json'}", file=sys.stderr)
        if args.keep_server:
            print(f"[staging] server left running: {base_url} (pid={process.pid})")
            process = None  # type: ignore[assignment]
    except Exception as exc:
        print(f"[staging] ERROR: {exc}", file=sys.stderr)
        audit_code = 1
    finally:
        if process is not None:
            stop_process(process)
        server_log.close()
        if not args.keep_runtime:
            shutil.rmtree(runtime_dir, ignore_errors=True)
        else:
            print(f"[staging] runtime kept: {runtime_dir}")

    return audit_code


if __name__ == "__main__":
    raise SystemExit(main())
