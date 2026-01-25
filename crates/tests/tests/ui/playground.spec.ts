import { test, expect } from '@playwright/test';

test.describe('Playground Page (Mocked)', () => {
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

    // Go to Playground
    await page.goto('/console/playground');
  });

  test('should verify playground interaction', async ({ page }) => {
    // Mock Chat Completions API
    await page.route('**/api/v1/chat/completions', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ 
          choices: [{ message: { content: 'Mock AI Response' } }] 
        })
      });
    });

    // Get Elements
    const input = page.getByPlaceholder(/Type a message/i);
    const sendBtn = page.getByRole('button', { name: /Send/i });

    // Ensure initial state
    await expect(input).toBeVisible();
    
    // Input text
    await input.fill('Hello World');
    
    // Verify Send button enabled
    await expect(sendBtn).toBeEnabled();

    // Click Send
    await sendBtn.click();
    
    // Verify input cleared
    await expect(input).toBeEmpty();
    
    // Verify user message appears
    await expect(page.getByText('Hello World')).toBeVisible();

    // Verify AI response appears (wait for mock)
    await expect(page.getByText('Mock AI Response')).toBeVisible();
  });
});