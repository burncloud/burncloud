import { test, expect } from '@playwright/test';

test.describe('Logs Page', () => {
  test.beforeEach(async ({ page, request }) => {
    const username = `logs-user-${Date.now()}`;
    const password = 'password123';
    
    // Register user
    const regResponse = await request.post('/console/api/user/register', { data: { username, password } });
    expect(regResponse.ok()).toBeTruthy();
    
    // Login
    await page.goto('/login');
    await page.getByPlaceholder('请输入用户名').fill(username);
    await page.getByPlaceholder('请输入密码').fill(password);
    await page.getByRole('button', { name: '登录' }).click(); 
    await expect(page).toHaveURL(/\/console\/dashboard/, { timeout: 10000 });
  });

  test('should verify logs loading and filtering', async ({ page }) => {
    // 1. Mock API response
    await page.route('**/console/api/logs*', async route => {
      const json = {
        data: [
          { id: '1', timestamp: '2023-10-27 10:00:00', level: 'INFO', message: 'System started' },
          { id: '2', timestamp: '2023-10-27 10:00:01', level: 'INFO', message: 'Service initialized' },
          { id: '3', timestamp: '2023-10-27 10:05:00', level: 'ERROR', message: 'Connection failed: timeout' },
          { id: '4', timestamp: '2023-10-27 10:05:01', level: 'ERROR', message: 'Retrying connection...' },
          { id: '5', timestamp: '2023-10-27 10:10:00', level: 'WARN', message: 'High memory usage' },
        ],
        page: 1,
        page_size: 50
      };
      await route.fulfill({ json });
    });

    // 2. Visit /console/logs
    await page.goto('/console/logs');

    // 3. Verify log container has entries
    await expect(page.getByText('Loading logs...')).not.toBeVisible();
    
    const logRows = page.locator('.log-entry');
    await expect(logRows).toHaveCount(5);
    await expect(page.getByText('System started')).toBeVisible();

    // 4. Search for 'ERROR'
    const searchInput = page.getByPlaceholder('Search logs...');
    await searchInput.fill('ERROR');

    // 5. Verify filtering
    await expect(logRows).toHaveCount(2);
    await expect(page.getByText('Connection failed: timeout')).toBeVisible();
    await expect(page.getByText('Retrying connection...')).toBeVisible();
    await expect(page.getByText('System started')).not.toBeVisible();

    // 6. Clear search
    await searchInput.fill('');

    // 7. Verify restoration
    await expect(logRows).toHaveCount(5);
    await expect(page.getByText('System started')).toBeVisible();
  });
});
