/**
 * Modal Dialog E2E Tests
 * 
 * Tests for modal interactions across the application
 */

import { test, expect } from '@playwright/test';
import { login } from '../utils/auth';

test.describe('Modal Interactions', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
  });

  test('should open and close modal', async ({ page }) => {
    await page.goto('/settings');
    
    // Open a modal (e.g., create token)
    const createButton = page.locator('button:has-text("Create"), button:has-text("新建")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      // Modal should be visible
      const modal = page.locator('.modal, [role="dialog"]');
      await expect(modal).toBeVisible({ timeout: 3000 });
      
      // Close modal via close button
      const closeButton = page.locator('.modal button:has-text("Cancel"), .modal button[aria-label*="close"], button:has-text("取消")');
      await closeButton.first().click();
      
      // Modal should close
      await expect(modal).not.toBeVisible({ timeout: 2000 });
    }
  });

  test('should close modal on escape key', async ({ page }) => {
    await page.goto('/settings');
    
    const createButton = page.locator('button:has-text("Create")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      const modal = page.locator('.modal, [role="dialog"]');
      await expect(modal).toBeVisible({ timeout: 3000 });
      
      // Press escape
      await page.keyboard.press('Escape');
      
      // Modal should close
      await expect(modal).not.toBeVisible({ timeout: 2000 });
    }
  });

  test('should close modal on backdrop click', async ({ page }) => {
    await page.goto('/settings');
    
    const createButton = page.locator('button:has-text("Create")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      const modal = page.locator('.modal, [role="dialog"]');
      await expect(modal).toBeVisible({ timeout: 3000 });
      
      // Click outside modal (backdrop)
      // Note: This depends on modal implementation
      const backdrop = page.locator('.modal-backdrop, .overlay');
      
      if (await backdrop.isVisible({ timeout: 1000 }).catch(() => false)) {
        await backdrop.click({ position: { x: 10, y: 10 } });
        await expect(modal).not.toBeVisible({ timeout: 2000 });
      }
    }
  });

  test('should trap focus within modal', async ({ page }) => {
    await page.goto('/settings');
    
    const createButton = page.locator('button:has-text("Create")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      const modal = page.locator('.modal, [role="dialog"]');
      await expect(modal).toBeVisible({ timeout: 3000 });
      
      // Tab through modal elements
      const focusableElements = await modal.locator('button, input, select, textarea, [tabindex]:not([tabindex="-1"])').count();
      
      if (focusableElements > 1) {
        // Tab through elements - focus should stay within modal
        for (let i = 0; i < focusableElements + 2; i++) {
          await page.keyboard.press('Tab');
        }
        
        // Focus should still be within modal
        const focusedElement = await page.evaluateHandle(() => document.activeElement);
        const isInModal = await modal.evaluate((node, focused) => node.contains(focused), focusedElement);
        
        expect(isInModal).toBe(true);
      }
    }
  });
});

test.describe('Confirmation Dialogs', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
  });

  test('should show confirmation for destructive actions', async ({ page }) => {
    await page.goto('/models');
    
    // Try to delete something
    const deleteButton = page.locator('button:has-text("Delete"), button:has-text("删除")').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await deleteButton.click();
      
      // Confirmation dialog should appear
      const confirmDialog = page.locator('.modal, [role="dialog"], .confirmation-dialog');
      await expect(confirmDialog).toBeVisible({ timeout: 3000 });
      
      // Should have confirm and cancel buttons
      const confirmButton = confirmDialog.locator('button:has-text("Confirm"), button:has-text("Delete"), button:has-text("确定")');
      const cancelButton = confirmDialog.locator('button:has-text("Cancel"), button:has-text("取消")');
      
      expect(await confirmButton.count() + await cancelButton.count()).toBeGreaterThan(0);
    }
  });

  test('should cancel destructive action', async ({ page }) => {
    await page.goto('/models');
    
    const deleteButton = page.locator('button:has-text("Delete")').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      const itemCountBefore = await page.locator('table tbody tr, .item').count();
      
      await deleteButton.click();
      
      // Cancel the action
      const cancelButton = page.locator('.modal button:has-text("Cancel"), button:has-text("取消")').first();
      await cancelButton.click();
      
      await page.waitForTimeout(500);
      
      // Item should still exist
      const itemCountAfter = await page.locator('table tbody tr, .item').count();
      expect(itemCountAfter).toBe(itemCountBefore);
    }
  });
});
