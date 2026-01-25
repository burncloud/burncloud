import { test, expect } from '@playwright/test';

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page, request }) => {
      const username = `dash-${Date.now()}`;
      const password = 'password123';
      
      // Seed user
      await request.post('/console/api/user/register', { data: { username, password } });
      
      // Login
      await page.goto('/login');
      await page.fill('input[type="text"]', username);
      await page.fill('input[type="password"]', password);
      await page.click('button');
      await expect(page).toHaveURL(/\/console\/dashboard/);
  });

  test('should display system metrics', async ({ page }) => {
    // Wait for the metric card
    // Use a more specific selector if possible, or wait for text
    await expect(page.locator('.metric-card').first()).toBeVisible({ timeout: 10000 });
    
    const body = page.locator('body');
    await expect(body).toContainText('CPU使用率');
    await expect(body).toContainText('内存');
    
    // Ensure we see some data (basic check)
    // We expect "GB" for memory and "%" for CPU
    await expect(body).toContainText('%');
    await expect(body).toContainText('GB');
  });
});
