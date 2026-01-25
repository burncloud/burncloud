import { test, expect } from '@playwright/test';

test.describe('Logs Page (Mocked)', () => {
  test.beforeEach(async ({ page }) => {
    // Mock Login endpoints
    await page.route('**/console/api/user/login', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ token: 'mock-token', user: { username: 'admin' } })
      });
    });

    await page.route('**/console/api/user/info', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ username: 'admin', role: 'admin' })
      });
    });

    // Perform Login
    await page.goto('/login');
    await page.getByPlaceholder('请输入用户名').fill('admin');
    await page.getByPlaceholder('请输入密码').fill('password');
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL(/\/console\/dashboard/);

    // Go to Logs page
    await page.goto('/console/logs');
  });

  test('should verify logs loading and filtering', async ({ page }) => {
    // Mock Logs API
    await page.route('**/api/v1/logs*', async route => {
      const url = new URL(route.request().url());
      const query = url.searchParams.get('q');
      
      if (query === 'ERROR') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            data: [
              { id: '2', timestamp: '2023-10-27 10:05:00', level: 'ERROR', message: 'Connection failed' }
            ]
          })
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            data: [
              { id: '1', timestamp: '2023-10-27 10:00:00', level: 'INFO', message: 'App started' },
              { id: '2', timestamp: '2023-10-27 10:05:00', level: 'ERROR', message: 'Connection failed' }
            ]
          })
        });
      }
    });

    // Verify 2 logs loaded
    await expect(page.getByText('App started')).toBeVisible();
    await expect(page.getByText('Connection failed')).toBeVisible();
    
    // Search
    const searchInput = page.getByPlaceholder(/Search/i);
    await searchInput.fill('ERROR');
    await searchInput.press('Enter');

    // Verify filtered to 1 log
    await expect(page.getByText('App started')).not.toBeVisible();
    await expect(page.getByText('Connection failed')).toBeVisible();

    // Clear Search
    // Attempt to find a clear button, fallback to clearing input manually if not found (to ensure test robustness)
    const clearBtn = page.getByRole('button', { name: /Clear|Reset/i });
    if (await clearBtn.isVisible()) {
        await clearBtn.click();
    } else {
        await searchInput.fill('');
        await searchInput.press('Enter');
    }
    
    // Verify restored
    await expect(page.getByText('App started')).toBeVisible();
    await expect(page.getByText('Connection failed')).toBeVisible();
  });
});