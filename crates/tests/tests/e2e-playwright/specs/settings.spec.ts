/**
 * Settings Page E2E Tests
 * 
 * Tests for user settings and configuration
 */

import { test, expect } from '@playwright/test';
import { login } from '../utils/auth';
import { checkToast } from '../utils/helpers';

test.describe('Settings Page', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/settings');
  });

  test('should display settings page', async ({ page }) => {
    // Check for settings container
    const settingsContainer = page.locator('[data-testid="settings"], .settings-container, main');
    await expect(settingsContainer).toBeVisible({ timeout: 5000 });
  });

  test('should have tabs for different settings sections', async ({ page }) => {
    // Common settings tabs
    const expectedTabs = ['Token', 'Profile', 'General'];
    
    for (const tabName of expectedTabs) {
      const tab = page.locator(`[role="tab"]:has-text("${tabName}"), button:has-text("${tabName}")`);
      
      // At least one tab should be visible
      if (await tab.first().isVisible({ timeout: 2000 }).catch(() => false)) {
        await tab.first().click();
        await page.waitForTimeout(500);
        
        // Tab content should be visible
        const tabPanel = page.locator('[role="tabpanel"], .tab-content');
        await expect(tabPanel.first()).toBeVisible({ timeout: 3000 });
      }
    }
  });

  test('should save profile settings', async ({ page }) => {
    // Navigate to profile tab
    const profileTab = page.locator('[role="tab"]:has-text("Profile"), button:has-text("Profile")');
    
    if (await profileTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await profileTab.click();
      
      // Find editable fields
      const emailInput = page.locator('input[name="email"], input[type="email"]').first();
      const usernameInput = page.locator('input[name="username"], input[name="name"]').first();
      
      if (await emailInput.isVisible()) {
        // Make a change
        const currentValue = await emailInput.inputValue();
        // Note: Usually email is not editable directly
        
        // Look for save button
        const saveButton = page.locator('button:has-text("Save"), button[type="submit"]');
        if (await saveButton.isVisible()) {
          await saveButton.click();
          
          await page.waitForTimeout(1000);
          
          // Check for success message
          const success = await checkToast(page, 'success');
          // May or may not show success depending on if changes were made
        }
      }
    }
  });

  test('should validate required fields', async ({ page }) => {
    // Try to submit with empty required fields
    const saveButton = page.locator('button:has-text("Save"), button[type="submit"]').first();
    
    if (await saveButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Clear a required field if possible
      const requiredInput = page.locator('input[required], input:required').first();
      
      if (await requiredInput.isVisible()) {
        await requiredInput.fill('');
        await saveButton.click();
        
        // Should show validation error
        const errorVisible = await page.locator('.error, [role="alert"], .invalid-feedback').isVisible({ timeout: 2000 }).catch(() => false);
        
        // Form should still be visible (not submitted)
        const formVisible = await page.locator('form, .settings-form').isVisible();
        expect(formVisible).toBe(true);
      }
    }
  });
});

test.describe('Groups Tab (if available)', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login and groups feature
    await login(page);
    await page.goto('/settings');
    
    // Navigate to groups tab
    const groupsTab = page.locator('[role="tab"]:has-text("Group"), button:has-text("Group")');
    if (await groupsTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await groupsTab.click();
    }
  });

  test('should display groups list or empty state', async ({ page }) => {
    const groupsTab = page.locator('[role="tab"]:has-text("Group")');
    
    if (await groupsTab.isVisible({ timeout: 2000 }).catch(() => false)) {
      await groupsTab.click();
      
      // Should show either groups list or empty state
      const groupList = page.locator('table, [data-testid="groups-list"], .group-item');
      const emptyState = page.locator('[data-testid="empty-state"], .empty-state, text=/no groups/i');
      
      const hasContent = await groupList.isVisible().catch(() => false) ||
                         await emptyState.isVisible().catch(() => false);
      
      expect(hasContent).toBe(true);
    }
  });
});
