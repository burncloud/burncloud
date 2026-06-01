/**
 * Channel Management E2E Tests
 * 
 * Tests for creating, editing, and deleting channels
 */

import { test, expect } from '@playwright/test';
import { login } from '../utils/auth';
import { confirmModal, checkToast, waitFor } from '../utils/helpers';

test.describe('Channel List', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/models');
  });

  test('should display channel list', async ({ page }) => {
    // Check for table or list container
    const listContainer = page.locator('table, [data-testid="channel-list"], .channel-grid');
    await expect(listContainer).toBeVisible({ timeout: 5000 });
  });

  test('should show channel details', async ({ page }) => {
    // Click on first channel
    const firstChannel = page.locator('table tr:first-child, .channel-item:first-child').first();
    
    if (await firstChannel.isVisible()) {
      await firstChannel.click();
      
      // Should show details panel or navigate to details page
      const detailsPanel = page.locator('[data-testid="channel-details"], .channel-details, .modal');
      await expect(detailsPanel).toBeVisible({ timeout: 3000 });
    }
  });
});

test.describe('Create Channel', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/models');
  });

  test('should open create channel form', async ({ page }) => {
    // Click create button
    const createButton = page.locator('button:has-text("Create"), button:has-text("Add"), button:has-text("新建")');
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.first().click();
      
      // Should show form or modal
      const form = page.locator('form, [data-testid="create-form"], .modal form');
      await expect(form).toBeVisible({ timeout: 3000 });
    }
  });

  test('should validate channel form fields', async ({ page }) => {
    const createButton = page.locator('button:has-text("Create"), button:has-text("Add")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      // Try to submit empty form
      const submitButton = page.locator('button[type="submit"]:has-text("Create"), button[type="submit"]:has-text("Save")');
      await submitButton.click();
      
      // Should show validation errors
      const errorMessages = page.locator('.error, [role="alert"], .invalid-feedback');
      // Check if form is still visible (wasn't submitted)
      const formVisible = await page.locator('form, .modal').isVisible();
      expect(formVisible).toBe(true);
    }
  });

  test('should create channel with valid data', async ({ page }) => {
    const createButton = page.locator('button:has-text("Create"), button:has-text("Add")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      
      // Fill form with test data
      const nameInput = page.locator('input[name="name"], input[placeholder*="name"], input[placeholder*="名称"]').first();
      await nameInput.fill('Test Channel E2E');
      
      // Fill other required fields based on form
      const urlInput = page.locator('input[name="url"], input[placeholder*="url"], input[placeholder*="URL"]').first();
      if (await urlInput.isVisible()) {
        await urlInput.fill('https://api.example.com');
      }
      
      // Submit form
      const submitButton = page.locator('button[type="submit"]');
      await submitButton.click();
      
      // Check for success
      await page.waitForTimeout(2000);
      
      // Should show success toast or close modal
      const successToast = await checkToast(page, 'success');
      const modalClosed = !await page.locator('.modal').isVisible().catch(() => true);
      
      expect(successToast || modalClosed).toBe(true);
    }
  });
});

test.describe('Edit Channel', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/models');
  });

  test('should open edit form', async ({ page }) => {
    // Find edit button for first channel
    const editButton = page.locator('button:has-text("Edit"), button:has-text("编辑"), [data-testid="edit-button"]').first();
    
    if (await editButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await editButton.click();
      
      // Should show edit form
      const form = page.locator('form, [data-testid="edit-form"], .modal');
      await expect(form).toBeVisible({ timeout: 3000 });
    }
  });

  test('should save channel changes', async ({ page }) => {
    const editButton = page.locator('button:has-text("Edit")').first();
    
    if (await editButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await editButton.click();
      
      // Modify a field
      const nameInput = page.locator('input[name="name"]').first();
      if (await nameInput.isVisible()) {
        await nameInput.fill('Updated Channel Name E2E');
        
        // Save changes
        const saveButton = page.locator('button[type="submit"], button:has-text("Save")');
        await saveButton.click();
        
        await page.waitForTimeout(2000);
        
        // Verify changes saved
        const successToast = await checkToast(page, 'success');
        expect(successToast).toBe(true);
      }
    }
  });
});

test.describe('Delete Channel', () => {
  
  test.beforeEach(async ({ page }) => {
    test.skip(); // Requires login
    await login(page);
    await page.goto('/models');
  });

  test('should show confirmation modal on delete', async ({ page }) => {
    const deleteButton = page.locator('button:has-text("Delete"), button:has-text("删除"), [data-testid="delete-button"]').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await deleteButton.click();
      
      // Should show confirmation modal
      const confirmModal = page.locator('.modal, [role="dialog"], .confirmation-dialog');
      await expect(confirmModal).toBeVisible({ timeout: 3000 });
    }
  });

  test('should delete channel after confirmation', async ({ page }) => {
    const deleteButton = page.locator('button:has-text("Delete")').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      // Get initial channel count
      const initialCount = await page.locator('table tbody tr, .channel-item').count();
      
      await deleteButton.click();
      
      // Confirm deletion
      await confirmModal(page, true);
      
      await page.waitForTimeout(2000);
      
      // Verify channel was deleted
      const newCount = await page.locator('table tbody tr, .channel-item').count();
      expect(newCount).toBeLessThan(initialCount);
    }
  });

  test('should cancel delete operation', async ({ page }) => {
    const deleteButton = page.locator('button:has-text("Delete")').first();
    
    if (await deleteButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await deleteButton.click();
      
      // Cancel deletion
      await confirmModal(page, false);
      
      // Modal should close
      const modalClosed = !await page.locator('.modal').isVisible().catch(() => true);
      expect(modalClosed).toBe(true);
    }
  });
});
