# BurnCloud E2E Tests

This directory contains end-to-end (E2E) tests for the BurnCloud web frontend using Playwright.

## Prerequisites

1. Node.js 18+ installed
2. Backend server running on http://localhost:3334
3. Test user account created in the database

## Quick Start

```bash
cd tests/e2e
npm install
npx playwright install
npm test
```

## Test Structure

- specs/ - Test files for different features
- fixtures/ - Test fixtures and helpers
- utils/ - Utility functions

## Running Tests

```bash
npm test                    # All tests
npx playwright test --project=chromium  # Specific browser
npm run test:ui             # With UI
npm run test:headed         # Headed mode
```

## CI/CD

Tests run automatically in GitHub Actions on push to main and PRs.
