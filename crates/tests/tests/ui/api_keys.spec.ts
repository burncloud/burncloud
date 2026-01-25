import { test, expect } from '@playwright/test';

test.describe('API Keys Page (Mocked)', () => {
  let mockKeys: any[] = [];

  test.beforeEach(async ({ page }) => {
    mockKeys = []; // Reset keys

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

    // Go to API Keys page
    await page.goto('/settings/api-keys');
  });

  test('should create new api key', async ({ page }) => {
    // Mock Keys API (GET and POST)
    await page.route('**/api/v1/keys', async route => {
      if (route.request().method() === 'GET') {
          await route.fulfill({
             status: 200,
             contentType: 'application/json',
             body: JSON.stringify(mockKeys)
          });
      } else if (route.request().method() === 'POST') {
          const newKey = { id: 'key_1', name: 'Test Key', secret: 'sk-1234567890abcdef', created: Date.now() };
          mockKeys.push(newKey);
          await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify(newKey)
          });
      }
    });

    // Verify Generate Button
    const generateBtn = page.getByRole('button', { name: /Generate New Key|Create/i });
    await expect(generateBtn).toBeVisible();
    await generateBtn.click();

    // Input Name
    const nameInput = page.getByPlaceholder(/Name|Key/i).or(page.locator('input[name="name"]')).first();
    await nameInput.fill('Test Key');
    
    // Submit
    const submitBtn = page.getByRole('button', { name: /Submit|Create|Save|立即创建/i });
    await submitBtn.click();

    // Verify New Row
    await expect(page.getByText('Test Key')).toBeVisible();

    // Verify Masked Secret (sk-...)
    // Expecting something like "sk-123..." or "sk-***"
    await expect(page.getByText(/sk-.*(\.\.\.|\*\*\*)/)).toBeVisible();
  });
});