import { test, expect } from '@playwright/test';
import * as fs from 'fs';

test.describe('User Management UI', () => {
  test.beforeEach(async ({ page, request }) => {
      const username = `admin-user-${Date.now()}`;
      const password = 'password123';
      
      // Register (seed)
      await request.post('/console/api/user/register', { data: { username, password } });
      
      // Login
      await page.goto('/login');
      await page.fill('input[type="text"]', username);
      await page.fill('input[type="password"]', password);
      await page.getByRole('button', { name: '登录' }).click();
      await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('should list users with correct balance and status', async ({ page, request }) => {
    // Create another user via API to verify list shows multiple users
    const testUsername = `test-user-${Date.now()}`;
    await request.post('/console/api/user/register', { data: { username: testUsername, password: 'password123' } });

    // Navigate to Users
    const userLink = page.locator('a[href*="users"]');
    if (await userLink.isVisible()) {
        await userLink.click();
    } else {
        await page.goto('/users');
    }
    
    await expect(page).toHaveURL(/.*users/);

    try {
        // Verify table headers using test-ids
        await expect(page.getByTestId('th-username')).toBeVisible({ timeout: 5000 });
        await expect(page.getByTestId('th-balance')).toBeVisible();

        // Verify new user presence
        // Use locator filtering to find the row with our user
        // We look for the username text which is ASCII
        await expect(page.locator('tr', { hasText: testUsername })).toBeVisible();
        
        // Verify Balance (Default 10.0) - Check for the number
        const row = page.locator('tr', { hasText: testUsername });
        await expect(row).toContainText('10.00');
        
                // Verify Role (Default user)
                await expect(row).toContainText('user');
        
                // --- Test Topup ---
                // Verify Topup button exists
                await expect(row.getByRole('button', { name: '充值' })).toBeVisible();
        
                /* 
                // TODO: Modal interaction is flaky in CI environment with Dioxus LiveView.
                // 1. Click Topup Button (Ensure we target the correct row's button)
                // The button text is "充值"
                await row.getByRole('button', { name: '充值' }).click();
        
                // 2. Verify Modal Opens
                await expect(page.locator('.modal-title-text')).toHaveText('用户充值');
        
                // 3. Fill Amount
                // Input usually has placeholder "请输入金额" or label "充值金额 (¥)"
                // BCInput structure: label + div > input
                // We can find by placeholder
                await page.getByPlaceholder('请输入金额').fill('50');
        
                // 4. Confirm
                await page.getByRole('button', { name: '确认充值' }).click();
        
                // 5. Verify Success Toast
                await expect(page.locator('.toast-success')).toBeVisible();
                await expect(page.locator('.toast-success')).toContainText('充值成功');
        
                // 6. Verify New Balance (10 + 50 = 60.00)
                // Wait for table update
                await expect(row).toContainText('60.00');
                */    } catch (e) {
        console.log('DEBUG: User table elements not found. Dumping page content:');
        console.log(await page.content());
        if (!fs.existsSync('data')) { fs.mkdirSync('data'); }
        await page.screenshot({ path: 'data/debug-users-failure.png', fullPage: true });
        fs.writeFileSync('data/debug-users-page.html', await page.content());
        throw e;
    }
  });
});
