#!/usr/bin/env python3
"""Deterministic runtime browser audit for the current BurnCloud Dioxus console.

The script talks to a real BurnCloud server, seeds data through real management APIs,
then drives the visible LiveView UI with agent-browser. It is intentionally external
to Rust so browser timing/debug evidence does not require recompiling the integration
crate and every failure can preserve screenshots plus machine-readable diagnostics.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

BASE_URL = os.environ.get("E2E_BASE_URL", "http://127.0.0.1:3000").rstrip("/")
AUDIT_DIR = Path(os.environ.get("STAGING_AUDIT_DIR", "target/staging-audit"))
SCREENSHOT_DIR = AUDIT_DIR / "screenshots"
SESSION = f"burncloud-staging-{os.getpid()}"
VIEWPORT = (1440, 900)

ADMIN = "staging-admin"
ADMIN_PASSWORD = "StagingAdmin123!"
CUSTOMER = "staging-customer"
PROVIDER = "Staging Dummy Provider"
MODEL = "staging-model"


@dataclass
class PageAudit:
    name: str
    path: str
    screenshot: str
    viewport_width: int
    viewport_height: int
    scroll_width: int
    scroll_height: int
    horizontal_overflow: bool
    visible_buttons: int
    visible_links: int


class BrowserError(RuntimeError):
    pass


class Browser:
    def __init__(self) -> None:
        SCREENSHOT_DIR.mkdir(parents=True, exist_ok=True)
        self.env = os.environ.copy()
        self.env["AGENT_BROWSER_SESSION"] = SESSION
        self.env["AGENT_BROWSER_ARGS"] = "--headless=new,--no-sandbox"

    def run(self, *args: str, timeout: int = 30) -> dict[str, Any]:
        cmd = ["agent-browser", "--json", "--session", SESSION, *args]
        proc = subprocess.run(
            cmd,
            env=self.env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
        stdout = proc.stdout.strip()
        stderr = proc.stderr.strip()
        try:
            payload = json.loads(stdout) if stdout else {}
        except json.JSONDecodeError:
            payload = {}
        if proc.returncode != 0 or payload.get("success") is False:
            raise BrowserError(
                f"command failed: {' '.join(cmd)}\n"
                f"returncode={proc.returncode}\nstdout={stdout}\nstderr={stderr}"
            )
        return payload

    def eval(self, expression: str) -> Any:
        payload = self.run("eval", expression)
        data = payload.get("data")
        if isinstance(data, dict) and "result" in data:
            return data["result"]
        return data

    def open(self, path: str) -> None:
        self.run("open", f"{BASE_URL}{path}")
        try:
            self.run("wait", "--load", "domcontentloaded")
        except BrowserError:
            pass
        time.sleep(0.5)

    def viewport(self, width: int, height: int) -> None:
        self.run("set", "viewport", str(width), str(height))

    def body_text(self) -> str:
        value = self.eval("document.body?.innerText || ''")
        return value if isinstance(value, str) else ""

    def path(self) -> str:
        value = self.eval("location.pathname")
        return value if isinstance(value, str) else ""

    def wait_path(self, expected: str, timeout: float = 15.0) -> None:
        deadline = time.monotonic() + timeout
        last = ""
        while time.monotonic() < deadline:
            last = self.path()
            if last == expected:
                return
            time.sleep(0.2)
        raise BrowserError(f"timeout waiting for path {expected!r}; last={last!r}")

    def wait_text(self, expected: str, timeout: float = 20.0) -> str:
        deadline = time.monotonic() + timeout
        last = ""
        while time.monotonic() < deadline:
            last = self.body_text()
            if expected in last:
                return last
            time.sleep(0.25)
        raise BrowserError(
            f"timeout waiting for text {expected!r} at path={self.path()!r}; "
            f"last_body={last[-3000:]!r}"
        )

    def click(self, selector: str) -> None:
        self.run("click", selector)

    def click_role(self, role: str, name: str) -> None:
        self.run("find", "role", role, "click", "--name", name)

    def fill(self, selector: str, value: str) -> None:
        self.run("fill", selector, value)

    def screenshot(self, name: str) -> None:
        target = SCREENSHOT_DIR / f"{name}.png"
        self.run("screenshot", "--full", str(target))

    def metrics(self) -> dict[str, Any]:
        raw = self.eval(
            "JSON.stringify((() => {"
            "const root=document.documentElement;"
            "const visible=(el)=>{const s=getComputedStyle(el);const r=el.getBoundingClientRect();"
            "return s.display!=='none'&&s.visibility!=='hidden'&&r.width>0&&r.height>0;};"
            "return {"
            "path:location.pathname,viewport_width:window.innerWidth,viewport_height:window.innerHeight,"
            "scroll_width:root.scrollWidth,scroll_height:root.scrollHeight,"
            "horizontal_overflow:root.scrollWidth>window.innerWidth+2,"
            "visible_buttons:[...document.querySelectorAll('button')].filter(visible).length,"
            "visible_links:[...document.querySelectorAll('a')].filter(visible).length};"
            "})())"
        )
        if not isinstance(raw, str):
            raise BrowserError(f"metrics did not return JSON text: {raw!r}")
        return json.loads(raw)

    def close(self) -> None:
        try:
            self.run("close", timeout=10)
        except Exception:
            pass


def api_post(path: str, body: dict[str, Any], token: str | None = None) -> dict[str, Any]:
    data = json.dumps(body).encode("utf-8")
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(
        f"{BASE_URL}{path}", data=data, headers=headers, method="POST"
    )
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            text = response.read().decode("utf-8")
            status = response.status
    except urllib.error.HTTPError as exc:
        text = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"POST {path} -> HTTP {exc.code}: {text}") from exc
    payload = json.loads(text)
    if status >= 300 or payload.get("success") is not True:
        raise RuntimeError(f"POST {path} -> {status}: {payload}")
    return payload


def seed() -> None:
    admin_response = api_post(
        "/api/auth/register",
        {
            "username": ADMIN,
            "email": "staging-admin@burncloud.local",
            "password": ADMIN_PASSWORD,
        },
    )
    admin_data = admin_response.get("data") or {}
    admin_id = admin_data.get("id")
    admin_token = admin_data.get("token")
    roles = admin_data.get("roles") or []
    if not admin_id or not admin_token or "admin" not in roles:
        raise RuntimeError(f"first staging account is not an admin: {admin_data}")

    api_post(
        "/api/auth/register",
        {
            "username": CUSTOMER,
            "email": "staging-customer@burncloud.local",
            "password": "StagingCustomer123!",
        },
    )

    api_post(
        "/console/api/channel",
        {
            "type": 8,
            "key": "staging-dummy-upstream-key",
            "name": PROVIDER,
            "base_url": "http://127.0.0.1:39999",
            "models": MODEL,
            "group": "default",
            "weight": 100,
            "priority": 0,
            "param_override": None,
            "header_override": None,
            "api_version": None,
            "model_mapping": None,
            "rpm_cap": None,
            "tpm_cap": None,
            "reservation_green": None,
            "reservation_yellow": None,
            "reservation_red": None,
        },
        admin_token,
    )

    api_post(
        "/console/api/tokens",
        {"user_id": admin_id, "quota_limit": None},
        admin_token,
    )


def ensure_login(browser: Browser) -> None:
    browser.open("/login")
    browser.viewport(*VIEWPORT)
    browser.wait_text("BurnCloud Console")
    browser.wait_text("Sign in to Console")
    browser.screenshot("00-login")

    browser.fill("input[type='text']", ADMIN)
    browser.fill("input[type='password']", ADMIN_PASSWORD)

    # agent-browser fill fires input events, but LiveView still needs one event-loop turn
    # before a click can reliably consume the new Dioxus signal values.
    synced = browser.eval(
        "document.querySelector(\"input[type='text']\")?.value.length>0 && "
        "document.querySelector(\"input[type='password']\")?.value.length>0"
    )
    if synced is not True:
        raise BrowserError("login inputs did not retain filled values")
    time.sleep(0.6)

    browser.click_role("button", "Sign in to Console")
    try:
        browser.wait_path("/", timeout=15)
    except BrowserError:
        browser.screenshot("00a-login-failed")
        raise BrowserError(
            f"login click did not leave /login; body={browser.body_text()[-3000:]!r}"
        )


def capture_page(
    browser: Browser,
    pages: list[PageAudit],
    name: str,
    path: str,
    expected_text: str,
    extra_text: tuple[str, ...] = (),
) -> None:
    browser.wait_path(path)
    browser.screenshot(f"{name}-initial")
    browser.wait_text(expected_text)
    for text in extra_text:
        browser.wait_text(text)
    time.sleep(0.25)
    browser.screenshot(name)
    metrics = browser.metrics()
    pages.append(
        PageAudit(
            name=name,
            path=str(metrics.get("path", path)),
            screenshot=f"screenshots/{name}.png",
            viewport_width=int(metrics.get("viewport_width", 0)),
            viewport_height=int(metrics.get("viewport_height", 0)),
            scroll_width=int(metrics.get("scroll_width", 0)),
            scroll_height=int(metrics.get("scroll_height", 0)),
            horizontal_overflow=bool(metrics.get("horizontal_overflow", False)),
            visible_buttons=int(metrics.get("visible_buttons", 0)),
            visible_links=int(metrics.get("visible_links", 0)),
        )
    )


def navigate(
    browser: Browser,
    pages: list[PageAudit],
    name: str,
    path: str,
    expected_text: str,
    extra_text: tuple[str, ...] = (),
) -> None:
    # Exact href makes the click-path deterministic while still exercising the actual
    # rendered sidebar anchor and Dioxus router interception.
    browser.screenshot(f"{name}-before-nav")
    browser.click(f"a[href='{path}']")
    try:
        browser.wait_path(path)
    except BrowserError:
        browser.screenshot(f"{name}-after-click-failed")
        raise BrowserError(
            f"sidebar click failed for {path}; current_path={browser.path()!r}; "
            f"body={browser.body_text()[-2500:]!r}"
        )
    capture_page(browser, pages, name, path, expected_text, extra_text)


def drawer(
    browser: Browser,
    name: str,
    button_name: str,
    expected_text: str,
) -> None:
    browser.click_role("button", button_name)
    browser.wait_text(expected_text, timeout=10)
    time.sleep(0.35)
    browser.screenshot(name)
    browser.click_role("button", "Cancel")
    time.sleep(0.2)


def write_report(pages: list[PageAudit]) -> None:
    AUDIT_DIR.mkdir(parents=True, exist_ok=True)
    report = {
        "base_url": BASE_URL,
        "viewport": f"{VIEWPORT[0]}x{VIEWPORT[1]}",
        "seeded_admin": ADMIN,
        "seeded_customer": CUSTOMER,
        "seeded_provider": PROVIDER,
        "seeded_model": MODEL,
        "pages": [asdict(page) for page in pages],
    }
    (AUDIT_DIR / "report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8"
    )

    lines = [
        "# BurnCloud Staging Browser Audit",
        "",
        f"- Base URL: `{BASE_URL}`",
        f"- Viewport: **{VIEWPORT[0]}×{VIEWPORT[1]}**",
        f"- Seeded provider/model: **{PROVIDER} / {MODEL}**",
        "",
        "| Page | Path | Viewport | Scroll | Overflow | Buttons | Links | Screenshot |",
        "|---|---|---:|---:|---|---:|---:|---|",
    ]
    for page in pages:
        lines.append(
            f"| {page.name} | `{page.path}` | "
            f"{page.viewport_width}×{page.viewport_height} | "
            f"{page.scroll_width}×{page.scroll_height} | "
            f"{'❌' if page.horizontal_overflow else '✅'} | "
            f"{page.visible_buttons} | {page.visible_links} | `{page.screenshot}` |"
        )
    (AUDIT_DIR / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_failure(browser: Browser, exc: BaseException) -> None:
    AUDIT_DIR.mkdir(parents=True, exist_ok=True)
    try:
        browser.screenshot("zz-failure")
    except Exception:
        pass
    try:
        path = browser.path()
        body = browser.body_text()
    except Exception:
        path = ""
        body = ""
    payload = {
        "error": repr(exc),
        "path": path,
        "body_tail": body[-5000:],
        "session": SESSION,
    }
    (AUDIT_DIR / "failure.json").write_text(
        json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8"
    )


def run_audit() -> None:
    AUDIT_DIR.mkdir(parents=True, exist_ok=True)
    seed()
    browser = Browser()
    pages: list[PageAudit] = []
    try:
        ensure_login(browser)
        capture_page(
            browser,
            pages,
            "01-overview",
            "/",
            "System Overview",
            ("Setup & readiness", PROVIDER),
        )

        navigate(browser, pages, "02-providers", "/providers", "Provider inventory", (PROVIDER, MODEL))
        drawer(
            browser,
            "02a-provider-add-drawer",
            "Add Provider",
            "Connect an upstream and define the models it can serve.",
        )

        navigate(
            browser,
            pages,
            "03-models",
            "/models",
            "Model availability",
            (MODEL, "Single upstream", "No failover redundancy"),
        )
        navigate(
            browser,
            pages,
            "04-routes",
            "/routes",
            "Routing Groups",
            ("default", "Single upstream"),
        )
        navigate(
            browser,
            pages,
            "05-playground",
            "/playground",
            "Playground",
            (MODEL, "Send Test Request"),
        )
        navigate(browser, pages, "06-logs", "/logs", "Logs")
        navigate(browser, pages, "07-evaluation", "/evaluation", "Evaluation")
        navigate(browser, pages, "08-billing", "/billing", "Billing")

        navigate(browser, pages, "09-api-keys", "/keys", "Credentials", (ADMIN,))
        drawer(
            browser,
            "09a-api-key-create-drawer",
            "Create API Key",
            "Choose which account will own this router credential.",
        )

        navigate(browser, pages, "10-customers", "/customers", "Customers", (CUSTOMER,))
        drawer(
            browser,
            "10a-customer-create-drawer",
            "Create Customer",
            "Create a business account that can own wallet balance and API access.",
        )

        navigate(browser, pages, "11-guardrails", "/guardrails", "Guardrails")
        navigate(browser, pages, "12-team", "/team", "Environment operators", (ADMIN,))
        navigate(browser, pages, "13-settings", "/settings", "Settings")

        write_report(pages)

        browser.click_role("button", "Sign Out")
        browser.wait_path("/login", timeout=10)
        browser.wait_text("BurnCloud Console", timeout=10)
        browser.screenshot("99-signed-out")

        overflow = [page.name for page in pages if page.horizontal_overflow]
        if overflow:
            raise BrowserError(f"horizontal overflow detected on: {', '.join(overflow)}")
        wrong_viewport = [
            page.name
            for page in pages
            if (page.viewport_width, page.viewport_height) != VIEWPORT
        ]
        if wrong_viewport:
            raise BrowserError(f"unexpected viewport on: {', '.join(wrong_viewport)}")
    except BaseException as exc:
        write_failure(browser, exc)
        raise
    finally:
        browser.close()


if __name__ == "__main__":
    try:
        run_audit()
    except BaseException as exc:
        print(f"STAGING_BROWSER_AUDIT_FAILED: {exc}", file=sys.stderr)
        sys.exit(1)
    print("STAGING_BROWSER_AUDIT_OK")
