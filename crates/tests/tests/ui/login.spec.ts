import { test, expect } from '@playwright/test';

test.describe('Login Flow', () => {
  const username = `ui-test-${Date.now()}`;
  const password = 'password123';

  test.beforeAll(async ({ request }) => {
    // Seed user
    const response = await request.post('/console/api/user/register', {
      data: {
        username: username,
        password: password
      }
    });
    expect(response.ok()).toBeTruthy();
  });

  test('should show login page elements', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('.auth-container')).toBeVisible();
    
    // Check for "用户名" label or input placeholder
    // Depending on actual UI implementation
    await expect(page.locator('.auth-card')).toContainText('用户名');
    await expect(page.getByRole('button', { name: '登录' })).toBeVisible();
  });

  test('should login and redirect to dashboard', async ({ page }) => {
    await page.goto('/login');
    
    // Fill form
    await page.fill('input[type="text"]', username);
    await page.fill('input[type="password"]', password);
    
    // Click login
    await page.click('button');

    // Wait for redirect
    await expect(page).toHaveURL(/\/$/);
    await expect(page.locator('body')).toContainText('仪表盘'); // Or 'Dashboard'
  });
});
