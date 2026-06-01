# BurnCloud E2E Tests (Playwright)

This directory contains end-to-end (E2E) tests for the BurnCloud web frontend using Playwright.

## Directory Structure

All frontend E2E tests are now unified under `crates/tests/tests/`:

- **Playwright tests**: `crates/tests/tests/e2e-playwright/` (this directory)
- **Rust E2E tests**: `crates/tests/tests/e2e/` (Dioxus LiveView frontend tests using agent-browser)

## Prerequisites

1. Node.js 18+ installed
2. Backend server running on http://localhost:3334
3. Test user account created in the database

## Quick Start

```bash
cd crates/tests/tests/e2e-playwright
npm install
npx playwright install
npm test
```

## Test Structure

- `specs/` - Test files for different features
- `fixtures/` - Test fixtures and helpers
- `utils/` - Utility functions

## Running Tests

```bash
npm test                    # All tests
npx playwright test --project=chromium  # Specific browser
npm run test:ui             # With UI
npm run test:headed         # Headed mode
```

## CI/CD

Tests run automatically in GitHub Actions on push to main and PRs.

## Related

- Rust E2E tests: `../e2e/` (agent-browser based tests for Dioxus LiveView)
