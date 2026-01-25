import { test, expect } from '@playwright/test';

test.describe('Deploy Page (Mocked)', () => {
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

    // Go to Deploy page
    await page.goto('/console/deploy');
  });

  test('should verify deploy form and successful deployment', async ({ page }) => {
    // Mock Deploy POST request
    await page.route('**/api/v1/models/deploy', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, modelId: 'gpt2' })
      });
    });

    // Verify Title
    await expect(page.getByRole('heading', { name: 'Model Deployment' })).toBeVisible();

    // Verify Initial State
    const deployBtn = page.getByRole('button', { name: 'Deploy' });
    const modelInput = page.getByPlaceholder('e.g. gpt2 or organization/model');
    const sourceSelect = page.locator('select');

    await expect(deployBtn).toBeDisabled();
    
    // Check Source default value (HuggingFace)
    await expect(sourceSelect).toHaveValue('HuggingFace');

    // Fill Form
    await modelInput.fill('gpt2');
    
    // Select Source
    await sourceSelect.selectOption('HuggingFace');

    // Verify Button Enabled
    await expect(deployBtn).toBeEnabled();

    // Click Deploy
    await deployBtn.click();

    // Verify Toast
    await expect(page.getByText('Deployment Successful')).toBeVisible();

    // Verify Navigation to /models (likely /console/models)
    await expect(page).toHaveURL(/\/console\/models/);
  });
});