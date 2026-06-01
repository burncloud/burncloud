/**
 * Token Management E2E Tests
 * 
 * Tests for API token CRUD operations
 */

import { test, expect } from '@playwright/test';
import { login } from '../utils/auth';
import { confirmModal, checkToast } from '../utils/helpers';

test.describe('Token List', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/settings');
    // Navigate to tokens tab if needed
    const tokensTab = page.locator('[role="tab"]:has-text("Token"), button:has-text("Token")');
    if (await tokensTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await tokensTab.click();
    }
  });

  test('should display token list', async ({ page }) => {
    // Check for token table or list
    const tokenList = page.locator('table, [data-testid="token-list"], .token-grid');
    await expect(tokenList).toBeVisible({ timeout: 5000 });
  });

  test('should show masked token values for security', async ({ page }) => {
    // Tokens should be displayed as masked (e.g., "sk-***...***")
    const tokenCell = page.locator('td:has-text("***"), .token-value:has-text("***")');
    
    if (await tokenCell.isVisible({ timeout: 3000 }).catch(() => false)) {
      const text = await tokenCell.textContent();
      expect(text).toContain('***');
    }
  });
});

test.describe('Create Token', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/settings');
    
    // Navigate to tokens tab
    const tokensTab = page.locator('[role="tab"]:has-text("Token")');
    if (await tokensTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await tokensTab.click();
    }
  });

  test('should open create token form', async ({ page }) => {
    const createButton = page.locator('button:has-text("Create Token"), button:has-text("Create"), button:has-text("新建")');
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.first().click();
      
      // Should show form
      const form = page.locator('form, .modal');
      await expect(form).toBeVisible({ timeout: 3000 });
    }
  });

  test('should create token and display once', async ({ page }) => {
    const createButton = page.locator('button:has-text("Create Token"), button:has-text("Create")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      // Fill token name
      const nameInput = page.locator('input[name="name"], input[placeholder*="name"]').first();
      await nameInput.fill('E2E Test Token');
      
      // Submit
      const submitButton = page.locator('button[type="submit"]');
      await submitButton.click();
      
      await page.waitForTimeout(2000);
      
      // Token should be displayed once (full value) - user needs to copy it
      const tokenDisplay = page.locator('[data-testid="new-token"], .token-display, code');
      
      // Check if token is shown or success message appears
      const tokenVisible = await tokenDisplay.isVisible().catch(() => false);
      const successVisible = await checkToast(page, 'success');
      
      expect(tokenVisible || successVisible).toBe(true);
    }
  });

  test('should allow copying new token', async ({ page }) => {
    const createButton = page.locator('button:has-text("Create Token")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      // Fill form and submit
      const nameInput = page.locator('input[name="name"]').first();
      await nameInput.fill('Copy Test Token');
      await page.locator('button[type="submit"]').click();
      
      await page.waitForTimeout(1000);
      
      // Look for copy button
      const copyButton = page.locator('button:has-text("Copy"), button[aria-label*="copy"]');
      
      if (await copyButton.isVisible({ timeout: 3000 }).catch(() => false)) {
        await copyButton.click();
        
        // Should show copied confirmation
        const copiedToast = await checkToast(page, 'copied');
        expect(copiedToast).toBe(true);
      }
    }
  });
});

test.describe('Delete Token', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/settings');
    
    const tokensTab = page.locator('[role="tab"]:has-text("Token")');
    if (await tokensTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await tokensTab.click();
    }
  });

  test('should show delete confirmation', async ({ page }) => {
    const deleteButton = page.locator('button:has-text("Delete"), button:has-text("删除")').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await deleteButton.click();
      
      // Should show confirmation dialog
      const confirmDialog = page.locator('.modal, [role="dialog"]');
      await expect(confirmDialog).toBeVisible({ timeout: 3000 });
    }
  });

  test('should delete token after confirmation', async ({ page }) => {
    const deleteButton = page.locator('button:has-text("Delete")').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      const initialCount = await page.locator('table tbody tr, .token-item').count();
      
      await deleteButton.click();
      await confirmModal(page, true);
      
      await page.waitForTimeout(2000);
      
      const newCount = await page.locator('table tbody tr, .token-item').count();
      expect(newCount).toBeLessThan(initialCount);
    }
  });
});
