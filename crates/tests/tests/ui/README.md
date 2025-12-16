# Registration System E2E Tests

This directory contains comprehensive end-to-end tests for the BurnCloud registration system using Playwright.

## Prerequisites

1. **Node.js** (v18 or higher)
2. **npm** or **yarn**
3. **BurnCloud Server** running locally

## Setup

```bash
cd crates/tests/tests/ui
npm install
npx playwright install chromium
```

## Running Tests

### All Tests
```bash
npm test
```

### Specific Test File
```bash
npx playwright test registration.spec.ts
```

### In UI Mode (Interactive)
```bash
npm run test:ui
```

### With Debugging
```bash
npx playwright test --debug
```

### Headless Mode (CI/CD)
```bash
npx playwright test --headed=false
```

## Test Coverage

The `registration.spec.ts` file includes comprehensive tests for:

### 1. Input Validation & Feedback
- Real-time email validation
- Username availability checking
- Password strength meter
- Confirm password matching
- Field validation errors

### 2. Security
- XSS input sanitization
- Secure password handling

### 3. User Experience
- Keyboard navigation (Tab order)
- Enter key form submission
- Password visibility toggle
- Form shake animation on errors
- Loading state management

### 4. Error Handling
- Network error messages
- Password mismatch errors
- Form validation feedback

### 5. Post-Registration Flow
- Auto-login after registration
- Redirect to dashboard
- Complete user journey testing

### 6. Accessibility
- Keyboard-only navigation
- Complete registration using only keyboard

### 7. Network Conditions
- Slow 3G simulation
- Double submission prevention
- Timeout handling

## Test Configuration

Tests are configured to:
- Run in parallel on CI
- Retry failed tests (2 retries on CI)
- Generate HTML reports
- Capture traces on first retry
- Use headless Chrome by default

## Environment Variables

Set the `BASE_URL` environment variable to test against a different server:

```bash
BASE_URL=http://localhost:4000 npm test
```

## CI/CD Integration

The tests are designed to run in CI/CD environments:

```yaml
# Example GitHub Actions configuration
- name: Install dependencies
  run: |
    cd crates/tests/tests/ui
    npm ci
    npx playwright install --with-deps chromium

- name: Run E2E tests
  run: |
    cd crates/tests/tests/ui
    npm test
```

## Debugging Tips

1. **Use UI Mode**: `npm run test:ui` for interactive debugging
2. **Slow Motion**: Add `--slow-mo=1000` to slow down test execution
3. **Screenshots**: Tests automatically capture screenshots on failure
4. **Traces**: View traces at `playwright-report/` after test run
5. **Console Logs**: Check browser console output in the report

## Writing New Tests

When adding new registration features:

1. Add test cases to `registration.spec.ts`
2. Follow existing test structure
3. Use descriptive test names
4. Include both positive and negative test cases
5. Test accessibility and keyboard navigation
6. Verify error states

## Test Maintenance

- Update tests when UI changes
- Keep test data isolated (use timestamps)
- Clean up test users periodically
- Review and update timeouts as needed
- Monitor test flakiness and fix root causes

## Known Issues

- Username availability check requires backend API endpoint: `/console/api/user/check_username`
- Auto-login requires backend to return full user data including token
- Some tests may be skipped if backend features are not implemented

## Support

For issues or questions:
1. Check the test output and error messages
2. Review the HTML report at `playwright-report/index.html`
3. Consult Playwright documentation: https://playwright.dev
4. Check burncloud repository issues
