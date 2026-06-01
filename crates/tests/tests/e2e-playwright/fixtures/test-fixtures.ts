/**
 * Playwright test fixtures for BurnCloud E2E tests
 */

import { test as base, Page, BrowserContext } from '@playwright/test';
import { login, TEST_USER } from '../utils/auth';

/**
 * Authenticated page fixture
 * Use this for tests that require a logged-in user
 */
export const test = base.extend<{
  authenticatedPage: Page;
}>({
  authenticatedPage: async ({ browser }, use) => {
    const context = await browser.newContext();
    const page = await context.newPage();
    
    // Login before test
    await page.goto('/login');
    
    try {
      await login(page, TEST_USER);
      await page.waitForURL(/\/(dashboard|home|models)/, { timeout: 15000 });
    } catch (e) {
      // Login may fail if test user doesn't exist
      console.log('Login failed, test may need to be skipped');
    }
    
    await use(page);
    
    await context.close();
  },
});

export { expect } from '@playwright/test';

/**
 * Test data factory
 */
export const testData = {
  generateTestEmail: () => `test-${Date.now()}@example.com`,
  generateTestName: () => `Test Item ${Date.now()}`,
  generateTestUrl: () => `https://api.test-${Date.now()}.example.com`,
};

/**
 * API mock helpers
 */
export const mockApi = {
  /**
   * Mock successful API response
   */
  success: (page: Page, path: string, data: object) => {
    return page.route(`**/api/${path}`, route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(data),
      });
    });
  },

  /**
   * Mock API error response
   */
  error: (page: Page, path: string, message: string, status: number = 400) => {
    return page.route(`**/api/${path}`, route => {
      route.fulfill({
        status,
        contentType: 'application/json',
        body: JSON.stringify({ error: message }),
      });
    });
  },

  /**
   * Mock unauthorized response
   */
  unauthorized: (page: Page, path: string) => {
    return page.route(`**/api/${path}`, route => {
      route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Unauthorized' }),
      });
    });
  },
};
