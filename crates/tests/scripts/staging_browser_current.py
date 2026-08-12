#!/usr/bin/env python3
"""Current BurnCloud Dioxus product journey, built on staging_browser_audit.py."""

from __future__ import annotations

import sys

import staging_browser_audit as base


def run() -> None:
    base.AUDIT_DIR.mkdir(parents=True, exist_ok=True)
    base.seed()
    browser = base.Browser()
    pages: list[base.PageAudit] = []
    try:
        base.ensure_login(browser)
        base.capture_page(browser, pages, "01-overview", "/", "System Overview", ("Setup & readiness", base.PROVIDER))

        base.navigate(browser, pages, "02-providers", "/providers", "Provider inventory", (base.PROVIDER, base.MODEL))
        base.drawer(browser, "02a-provider-add-drawer", "Add Provider", "Connect an upstream and define the models it can serve.")

        base.navigate(browser, pages, "03-models", "/models", "Model availability", (base.MODEL, "Single upstream", "No failover redundancy"))
        # Use the page title plus actual route risk state. Metric labels are deliberately
        # not treated as copy contracts because their casing is purely presentational.
        base.navigate(browser, pages, "04-routes", "/routes", "Routes", ("default", "Single upstream", base.PROVIDER))
        base.navigate(browser, pages, "05-playground", "/playground", "Playground", (base.MODEL, "Send Test Request"))

        base.navigate(browser, pages, "06-logs", "/logs", "Logs", ("Request activity",))
        base.navigate(browser, pages, "07-evaluation", "/evaluation", "Evaluation")
        base.navigate(browser, pages, "08-billing", "/billing", "Billing")

        base.navigate(browser, pages, "09-api-keys", "/keys", "Credentials", (base.ADMIN,))
        base.drawer(browser, "09a-api-key-create-drawer", "Create API Key", "Choose which account will own this router credential.")

        base.navigate(browser, pages, "10-customers", "/customers", "Customers", (base.CUSTOMER,))
        base.drawer(browser, "10a-customer-create-drawer", "Create Customer", "Create a business account that can own wallet balance and API access.")

        base.navigate(browser, pages, "11-guardrails", "/guardrails", "Guardrails")
        base.navigate(browser, pages, "12-team", "/team", "Environment operators", (base.ADMIN,))
        base.navigate(browser, pages, "13-settings", "/settings", "Settings")

        base.write_report(pages)

        browser.click_role("button", "Sign Out")
        browser.wait_path("/login", timeout=10)
        browser.wait_text("BurnCloud Console", timeout=10)
        browser.screenshot("99-signed-out")

        overflow = [page.name for page in pages if page.horizontal_overflow]
        if overflow:
            raise base.BrowserError(f"horizontal overflow detected on: {', '.join(overflow)}")
        wrong_viewport = [page.name for page in pages if (page.viewport_width, page.viewport_height) != base.VIEWPORT]
        if wrong_viewport:
            raise base.BrowserError(f"unexpected viewport on: {', '.join(wrong_viewport)}")
    except BaseException as exc:
        base.write_failure(browser, exc)
        raise
    finally:
        browser.close()


if __name__ == "__main__":
    try:
        run()
    except BaseException as exc:
        print(f"STAGING_BROWSER_AUDIT_FAILED: {exc}", file=sys.stderr)
        sys.exit(1)
    print("STAGING_BROWSER_AUDIT_OK")
