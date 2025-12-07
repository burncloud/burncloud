import { test, expect } from '@playwright/test';

test.describe('Channel Management UI', () => {
  test.beforeEach(async ({ page, request }) => {
      const username = `chan-ui-${Date.now()}`;
      const password = 'password123';
      
      await request.post('/console/api/user/register', { data: { username, password } });
      
      await page.goto('/login');
      await page.fill('input[type="text"]', username);
      await page.fill('input[type="password"]', password);
      // Use robust role-based locator
      await page.getByRole('button', { name: '登录' }).click();
      await expect(page).toHaveURL(/\/$/);
  });

  test('should create and delete a channel via UI', async ({ page }) => {
    const channelName = `UI-Auto-${Date.now()}`;
    const channelKey = `sk-test-key-${Date.now()}`;

    // 1. Navigate to Channels
    const channelLink = page.locator('a[href*="channels"]');
    if (await channelLink.isVisible()) {
        await channelLink.click();
    } else {
        await page.goto('/channels');
    }
    
    console.log('Current URL:', page.url());
    await expect(page).toHaveURL(/.*channels/);

    // 2. Open Create Modal
    // Wait for hydration - give Dioxus a moment to attach event listeners
    await page.waitForTimeout(3000); // Increased to 3s
    await page.getByRole('button', { name: '新建渠道' }).click();
    
    try {
        // Assert modal title text is visible, class name changed
        await expect(page.locator('.modal-title-text')).toBeVisible({ timeout: 5000 });
    } catch (e) {
        console.log('DEBUG: Modal not found. Dumping page content:');
        console.log(await page.content());
        throw e;
    }

    // 3. Fill Form
    // Since we didn't add classes to inputs, we use placeholders or order.
    // Placeholder matching is fragile with encoding issues, let's use type/order if possible
    // But BCInput structure is div > input.
    // Let's try matching by label text proximity if possible, or just use the updated code structure knowledge.
    // The inputs are in a vstack.
    // 1. Name, 2. Type (select), 3. Key, 4. URL, 5. Models
    
    // Safer approach: Get all text inputs inside modal
    const inputs = page.locator('.modal-body input[type="text"]');
    await inputs.nth(0).fill(channelName); // Name
    await inputs.nth(1).fill(channelKey);  // Key
    // Base URL (nth 2) and Models (nth 3) have defaults

    // 4. Save
    await page.click('.btn-save-channel');

    // 5. Verify Success Toast
    // Wait for toast to appear
    await expect(page.locator('.toast-success')).toBeVisible();
    
    // 6. Verify List
    // Wait for the new row to appear
    await expect(page.locator('.channel-row', { hasText: channelName })).toBeVisible();

    // 7. Delete
    const row = page.locator('.channel-row', { hasText: channelName });
    await row.locator('.btn-delete-channel').click();
    
    // 8. Verify Delete
    await expect(page.locator('.toast-success').last()).toBeVisible(); // Might match the previous one if not careful, but usually new toast appends
    await expect(page.locator('.channel-row', { hasText: channelName })).not.toBeVisible();
  });
});
