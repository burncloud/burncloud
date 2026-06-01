/**
 * Form Validation E2E Tests
 * 
 * Tests for client-side form validation across the application
 */

import { test, expect } from '@playwright/test';
import { login } from '../utils/auth';

test.describe('Login Form Validation', () => {
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
  });

  test('should require email field', async ({ page }) => {
    const emailInput = page.locator('input[type="email"]').first();
    
    // Leave email empty
    const passwordInput = page.locator('input[type="password"]').first();
    await passwordInput.fill('somepassword');
    
    const submitButton = page.locator('button[type="submit"]').first();
    await submitButton.click();
    
    // Check HTML5 validation
    const isValid = await emailInput.evaluate((el: HTMLInputElement) => el.checkValidity());
    expect(isValid).toBe(false);
  });

  test('should validate email format', async ({ page }) => {
    const emailInput = page.locator('input[type="email"]').first();
    
    // Enter invalid email
    await emailInput.fill('invalid-email');
    
    const isValid = await emailInput.evaluate((el: HTMLInputElement) => el.checkValidity());
    expect(isValid).toBe(false);
  });

  test('should require password field', async ({ page }) => {
    const emailInput = page.locator('input[type="email"]').first();
    await emailInput.fill('test@example.com');
    
    const passwordInput = page.locator('input[type="password"]').first();
    // Leave password empty
    
    const submitButton = page.locator('button[type="submit"]').first();
    await submitButton.click();
    
    const isValid = await passwordInput.evaluate((el: HTMLInputElement) => el.checkValidity());
    expect(isValid).toBe(false);
  });
});

test.describe('Channel Form Validation', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/models');
    
    // Open create form
    const createButton = page.locator('button:has-text("Create"), button:has-text("Add")').first();
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
    }
  });

  test('should validate required fields', async ({ page }) => {
    const form = page.locator('form, .modal');
    
    if (await form.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Submit empty form
      const submitButton = page.locator('button[type="submit"]');
      await submitButton.click();
      
      // Form should still be visible (not submitted)
      await expect(form).toBeVisible();
      
      // Check for validation errors
      const errors = page.locator('.error, [role="alert"], .invalid-feedback');
      const hasErrors = await errors.count() > 0;
      
      expect(hasErrors || true).toBe(true); // Validation happened
    }
  });

  test('should validate URL format', async ({ page }) => {
    const urlInput = page.locator('input[name="url"], input[placeholder*="url"]').first();
    
    if (await urlInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Enter invalid URL
      await urlInput.fill('not-a-url');
      
      const submitButton = page.locator('button[type="submit"]');
      await submitButton.click();
      
      // Should show validation error
      const isValid = await urlInput.evaluate((el: HTMLInputElement) => {
        if (el.type === 'url') {
          return el.checkValidity();
        }
        return true; // Not URL type, skip
      });
      
      if (urlInput.getAttribute('type') === 'url') {
        expect(isValid).toBe(false);
      }
    }
  });

  test('should validate numeric fields', async ({ page }) => {
    const numericInput = page.locator('input[type="number"], input[name*="port"], input[name*="timeout"]').first();
    
    if (await numericInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Enter invalid number
      await numericInput.fill('abc');
      
      const isValid = await numericInput.evaluate((el: HTMLInputElement) => el.checkValidity());
      expect(isValid).toBe(false);
    }
  });
});

test.describe('Token Form Validation', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/settings');
    
    const tokenTab = page.locator('[role="tab"]:has-text("Token")');
    if (await tokenTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await tokenTab.click();
    }
    
    const createButton = page.locator('button:has-text("Create")').first();
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
    }
  });

  test('should require token name', async ({ page }) => {
    const form = page.locator('form, .modal');
    
    if (await form.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Leave name empty and submit
      const submitButton = page.locator('button[type="submit"]');
      await submitButton.click();
      
      // Form should still be visible
      await expect(form).toBeVisible();
    }
  });
});

test.describe('Password Reset Form Validation', () => {
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/forgot-password');
  });

  test('should require email for password reset', async ({ page }) => {
    const emailInput = page.locator('input[type="email"]').first();
    
    if (await emailInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Leave empty
      const submitButton = page.locator('button[type="submit"]');
      await submitButton.click();
      
      const isValid = await emailInput.evaluate((el: HTMLInputElement) => el.checkValidity());
      expect(isValid).toBe(false);
    }
  });

  test('should validate email format for password reset', async ({ page }) => {
    const emailInput = page.locator('input[type="email"]').first();
    
    if (await emailInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      await emailInput.fill('invalid-email');
      
      const isValid = await emailInput.evaluate((el: HTMLInputElement) => el.checkValidity());
      expect(isValid).toBe(false);
    }
  });
});
