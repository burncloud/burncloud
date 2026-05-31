/**
 * Authentication utilities for E2E tests
 */

import { Page, expect } from '@playwright/test';

export interface TestUser {
  username: string;
  password: string;
}

// Default test user credentials
export const TEST_USER: TestUser = {
  username: 'test@example.com',
  password: 'TestPassword123!',
};

/**
 * Login to the application and store auth state
 */
export async function login(page: Page, user: TestUser = TEST_USER): Promise<void> {
  await page.goto('/login');
  
  // Wait for the login form to be visible
  await page.waitForSelector('input[type="email"], input[placeholder*="email"], input[name="email"]', { timeout: 10000 });
  
  // Fill in login credentials
  const emailInput = page.locator('input[type="email"], input[placeholder*="email"], input[name="email"]').first();
  await emailInput.fill(user.username);
  
  const passwordInput = page.locator('input[type="password"]').first();
  await passwordInput.fill(user.password);
  
  // Click login button
  const loginButton = page.locator('button:has-text("Login"), button:has-text("登录"), button[type="submit"]').first();
  await loginButton.click();
  
  // Wait for navigation after login
  await page.waitForURL(/\/(dashboard|home|models)/, { timeout: 15000 }).catch(() => {
    // May stay on login page if credentials are wrong
  });
}

/**
 * Logout from the application
 */
export async function logout(page: Page): Promise<void> {
  // Try to find and click logout button
  const logoutButton = page.locator('button:has-text("Logout"), button:has-text("登出"), a:has-text("Logout")');
  
  if (await logoutButton.isVisible({ timeout: 2000 }).catch(() => false)) {
    await logoutButton.click();
    await page.waitForURL('/login', { timeout: 5000 }).catch(() => {});
  }
}

/**
 * Check if user is logged in
 */
export async function isLoggedIn(page: Page): Promise<boolean> {
  // Check for presence of user-specific elements
  const dashboardElements = await page.locator('nav, [data-testid="sidebar"], .sidebar').count();
  const loginElements = await page.locator('input[type="password"]').count();
  
  return dashboardElements > 0 && loginElements === 0;
}

/**
 * Get JWT token from localStorage
 */
export async function getAuthToken(page: Page): Promise<string | null> {
  return await page.evaluate(() => {
    return localStorage.getItem('token') || localStorage.getItem('jwt') || sessionStorage.getItem('token');
  });
}

/**
 * Save authentication state to a file
 */
export async function saveAuthState(page: Page, path: string): Promise<void> {
  await page.context().storageState({ path });
}
