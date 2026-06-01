/**
 * Authentication E2E Tests
 * 
 * Tests login/logout flow for BurnCloud web frontend
 */

import { test, expect } from '@playwright/test';
import { login, logout, isLoggedIn, getAuthToken } from '../utils/auth';

test.describe('Authentication Flow', () => {
  
  test.beforeEach(async ({ page }) => {
    // Navigate to home page
    await page.goto('/');
  });

  test('should display login page for unauthenticated users', async ({ page }) => {
    await page.goto('/login');
    
    // Check for login form elements
    await expect(page.locator('input[type="email"], input[placeholder*="email"]').first()).toBeVisible({ timeout: 10000 });
    await expect(page.locator('input[type="password"]').first()).toBeVisible();
    await expect(page.locator('button:has-text("Login"), button:has-text("登录"), button[type="submit"]').first()).toBeVisible();
  });

  test('should show error for invalid credentials', async ({ page }) => {
    await page.goto('/login');
    
    // Fill with invalid credentials
    const emailInput = page.locator('input[type="email"], input[placeholder*="email"]').first();
    await emailInput.fill('invalid@test.com');
    
    const passwordInput = page.locator('input[type="password"]').first();
    await passwordInput.fill('wrongpassword');
    
    // Click login
    const loginButton = page.locator('button:has-text("Login"), button:has-text("登录"), button[type="submit"]').first();
    await loginButton.click();
    
    // Wait for error message or stay on login page
    await page.waitForTimeout(2000);
    
    // Should still be on login page (not redirected to dashboard)
    const currentUrl = page.url();
    expect(currentUrl).toContain('/login');
  });

  test('should navigate to dashboard after successful login', async ({ page }) => {
    // This test requires a valid test user account
    // Skip if test user doesn't exist
    test.skip();
    
    await login(page);
    
    // Verify we're logged in
    const loggedIn = await isLoggedIn(page);
    expect(loggedIn).toBe(true);
    
    // Verify token is stored
    const token = await getAuthToken(page);
    expect(token).toBeTruthy();
  });

  test('should redirect protected routes to login', async ({ page }) => {
    // Try to access protected route without authentication
    const protectedRoutes = ['/settings', '/dashboard', '/models', '/monitor'];
    
    for (const route of protectedRoutes) {
      await page.goto(route);
      await page.waitForTimeout(1000);
      
      // Should either redirect to login or show login form
      const currentUrl = page.url();
      const hasLoginForm = await page.locator('input[type="password"]').isVisible().catch(() => false);
      
      expect(
        currentUrl.includes('/login') || hasLoginForm,
        `Route ${route} should redirect to login or show login form`
      ).toBe(true);
    }
  });

  test('should logout successfully', async ({ page }) => {
    test.skip(); // Requires login first
    
    await login(page);
    await logout(page);
    
    // Verify we're logged out
    const loggedIn = await isLoggedIn(page);
    expect(loggedIn).toBe(false);
    
    // Verify token is cleared
    const token = await getAuthToken(page);
    expect(token).toBeFalsy();
  });

  test('should remember user session (remember me)', async ({ page }) => {
    test.skip(); // Requires login and session persistence
    
    await login(page);
    
    // Reload page
    await page.reload();
    
    // Should still be logged in
    const loggedIn = await isLoggedIn(page);
    expect(loggedIn).toBe(true);
  });
});

test.describe('Login Form Validation', () => {
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
  });

  test('should validate email format', async ({ page }) => {
    const emailInput = page.locator('input[type="email"], input[placeholder*="email"]').first();
    
    // Enter invalid email
    await emailInput.fill('not-an-email');
    
    // Try to submit
    const loginButton = page.locator('button:has-text("Login"), button:has-text("登录"), button[type="submit"]').first();
    await loginButton.click();
    
    // Should show validation error or prevent submission
    // HTML5 validation should prevent form submission
    const isValid = await emailInput.evaluate((el: HTMLInputElement) => el.checkValidity());
    expect(isValid).toBe(false);
  });

  test('should require password field', async ({ page }) => {
    const emailInput = page.locator('input[type="email"], input[placeholder*="email"]').first();
    await emailInput.fill('test@example.com');
    
    // Leave password empty
    const loginButton = page.locator('button:has-text("Login"), button:has-text("登录"), button[type="submit"]').first();
    await loginButton.click();
    
    // Password field should be required
    const passwordInput = page.locator('input[type="password"]').first();
    const isValid = await passwordInput.evaluate((el: HTMLInputElement) => el.checkValidity());
    expect(isValid).toBe(false);
  });
});
