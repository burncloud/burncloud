import { test, expect } from '@playwright/test';

test.describe('Billing Page (Mocked)', () => {
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

    // Go to Billing page
    await page.goto('/settings/billing');
  });

  test('should verify balance and recharge flow', async ({ page }) => {
    // Mock Balance API
    await page.route('**/api/v1/billing/balance', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ balance: 50000, currency: 'USD' })
      });
    });

    // Mock Recharge POST
    await page.route('**/api/v1/billing/recharge', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, new_balance: 51000 })
      });
    });

    // Verify Balance Display
    await expect(page.getByText('$50,000')).toBeVisible();

    // Click Recharge Button
    const rechargeBtn = page.getByRole('button', { name: /Recharge/i });
    await rechargeBtn.click();

    // Input Amount
    // Use a robust selector for amount input (placeholder or generic number input)
    const amountInput = page.getByPlaceholder(/Amount|金额/i).or(page.locator('input[type="number"]')).first();
    await amountInput.fill('1000');

    // Click Submit
    const submitBtn = page.getByRole('button', { name: /Submit|Confirm|Pay|提交/i });
    await submitBtn.click();

    // Verify Result (New Balance or Success Message)
    await expect(
        page.getByText('$51,000').or(page.getByText(/充值成功|Success/i))
    ).toBeVisible();
  });
});