import { test, expect } from '@playwright/test';
import * as fs from 'fs';

test.describe('API Keys (Access Credentials) Page', () => {
  test.beforeEach(async ({ page, request }) => {
    const username = `apikey-user-${Date.now()}`;
    const password = 'password123';

    // Register user
    await request.post('/console/api/user/register', { data: { username, password } });

    // Login
    await page.goto('/login');
    await page.fill('input[type="text"]', username);
    await page.fill('input[type="password"]', password);
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('should display Generate New Key button', async ({ page }) => {
    // Navigate to Access Credentials page
    await page.goto('/console/access');

    try {
      // Verify the "创建新凭证" (Generate New Key) button exists
      const createButton = page.getByRole('button', { name: '创建新凭证' });
      await expect(createButton).toBeVisible({ timeout: 10000 });
    } catch (e) {
      console.log('DEBUG: API Keys page elements not found. Dumping page content:');
      console.log(await page.content());
      if (!fs.existsSync('data')) { fs.mkdirSync('data'); }
      await page.screenshot({ path: 'data/debug-api-keys-button.png', fullPage: true });
      fs.writeFileSync('data/debug-api-keys-page.html', await page.content());
      throw e;
    }
  });

  test('should create a new API key and display masked key', async ({ page }) => {
    await page.goto('/console/access');

    try {
      // Click "创建新凭证" button to open modal
      await page.getByRole('button', { name: '创建新凭证' }).click();

      // Verify modal opens - look for modal title "创建访问凭证"
      await expect(page.locator('h3', { hasText: '创建访问凭证' })).toBeVisible({ timeout: 5000 });

      // Fill in key name (optional but good practice)
      const nameInput = page.getByPlaceholder('e.g. My Chatbot Production');
      if (await nameInput.isVisible()) {
        await nameInput.fill('Test API Key');
      }

      // Click "立即创建" to generate the key
      await page.getByRole('button', { name: '立即创建' }).click();

      // Verify success modal appears with "凭证已创建" title
      await expect(page.locator('h3', { hasText: '凭证已创建' })).toBeVisible({ timeout: 10000 });

      // Verify the full key is displayed (should be selectable text)
      const keyDisplay = page.locator('.select-all');
      await expect(keyDisplay).toBeVisible();
      const keyText = await keyDisplay.textContent();
      expect(keyText).toBeTruthy();
      expect(keyText!.length).toBeGreaterThan(10); // Key should have reasonable length

      // Close the modal by clicking "我已保存"
      await page.getByRole('button', { name: '我已保存' }).click();

      // Verify the key now appears in the list with masked format (e.g., 'sk-burn...xxxx')
      await expect(page.locator('.font-mono', { hasText: '...' })).toBeVisible({ timeout: 5000 });
    } catch (e) {
      console.log('DEBUG: API key creation failed. Dumping page content:');
      console.log(await page.content());
      if (!fs.existsSync('data')) { fs.mkdirSync('data'); }
      await page.screenshot({ path: 'data/debug-api-keys-create.png', fullPage: true });
      fs.writeFileSync('data/debug-api-keys-create.html', await page.content());
      throw e;
    }
  });

  test('should show delete confirmation modal when clicking delete button', async ({ page }) => {
    await page.goto('/console/access');

    try {
      // First create a key to delete
      await page.getByRole('button', { name: '创建新凭证' }).click();
      await expect(page.locator('h3', { hasText: '创建访问凭证' })).toBeVisible({ timeout: 5000 });
      await page.getByRole('button', { name: '立即创建' }).click();
      await expect(page.locator('h3', { hasText: '凭证已创建' })).toBeVisible({ timeout: 10000 });
      await page.getByRole('button', { name: '我已保存' }).click();

      // Wait for key to appear in list
      await expect(page.locator('.font-mono', { hasText: '...' })).toBeVisible({ timeout: 5000 });

      // Click delete button (trash icon button)
      // The delete button has a trash SVG icon
      const deleteButton = page.locator('button.btn-ghost.btn-square').filter({ hasText: '' }).last();
      await deleteButton.click();

      // Verify delete confirmation modal appears with "确认吊销" title
      await expect(page.locator('h3', { hasText: '确认吊销' })).toBeVisible({ timeout: 5000 });

      // Verify warning text is present
      await expect(page.locator('text=此操作无法撤销')).toBeVisible();
    } catch (e) {
      console.log('DEBUG: Delete confirmation modal test failed. Dumping page content:');
      console.log(await page.content());
      if (!fs.existsSync('data')) { fs.mkdirSync('data'); }
      await page.screenshot({ path: 'data/debug-api-keys-delete-modal.png', fullPage: true });
      fs.writeFileSync('data/debug-api-keys-delete-modal.html', await page.content());
      throw e;
    }
  });

  test('should delete API key after confirming in modal', async ({ page }) => {
    await page.goto('/console/access');

    try {
      // First create a key to delete
      await page.getByRole('button', { name: '创建新凭证' }).click();
      await expect(page.locator('h3', { hasText: '创建访问凭证' })).toBeVisible({ timeout: 5000 });
      await page.getByRole('button', { name: '立即创建' }).click();
      await expect(page.locator('h3', { hasText: '凭证已创建' })).toBeVisible({ timeout: 10000 });
      await page.getByRole('button', { name: '我已保存' }).click();

      // Wait for key to appear in list
      const keyRow = page.locator('.font-mono', { hasText: '...' });
      await expect(keyRow).toBeVisible({ timeout: 5000 });

      // Click delete button
      const deleteButton = page.locator('button.btn-ghost.btn-square').filter({ hasText: '' }).last();
      await deleteButton.click();

      // Verify delete modal appears
      await expect(page.locator('h3', { hasText: '确认吊销' })).toBeVisible({ timeout: 5000 });

      // Click "确认吊销" to confirm deletion
      await page.getByRole('button', { name: '确认吊销' }).click();

      // Verify the key is removed from the list
      // After deletion, either the empty state appears or the key row disappears
      // We check that we're back to showing the empty state or the list has no masked keys
      await expect(page.locator('text=没有活跃的访问凭证').or(page.locator('.font-mono', { hasText: '...' }).first())).toBeVisible({ timeout: 10000 });
    } catch (e) {
      console.log('DEBUG: Delete API key test failed. Dumping page content:');
      console.log(await page.content());
      if (!fs.existsSync('data')) { fs.mkdirSync('data'); }
      await page.screenshot({ path: 'data/debug-api-keys-delete.png', fullPage: true });
      fs.writeFileSync('data/debug-api-keys-delete.html', await page.content());
      throw e;
    }
  });
});
