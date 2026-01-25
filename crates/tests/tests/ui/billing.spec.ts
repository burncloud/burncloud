import { test, expect } from '@playwright/test';

test.describe('Billing Page', () => {
  test.beforeEach(async ({ page, request }) => {
    const username = `billing-${Date.now()}`;
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

  test('should verify billing page elements and recharge flow', async ({ page }) => {
    // 1. Visit /console/finance
    await page.goto('/console/finance');

    // 2. Verify Title "财务中心"
    await expect(page.getByRole('heading', { name: '财务中心' })).toBeVisible();

    // 3. Verify Balance "账户余额" and currency "¥"
    await expect(page.getByText('账户余额', { exact: true })).toBeVisible();
    await expect(page.getByText('¥').first()).toBeVisible();

    // 4. Verify "充值余额" button
    const rechargeBtn = page.getByRole('button', { name: '充值余额' });
    await expect(rechargeBtn).toBeVisible();

    // 5. Click Recharge Button
    await rechargeBtn.click();

    // 6. Verify History Table
    await expect(page.getByRole('heading', { name: '充值记录' })).toBeVisible();
    const table = page.locator('table');
    await expect(table).toBeVisible();
    await expect(table.getByText('交易 ID')).toBeVisible();
    await expect(table.getByText('金额')).toBeVisible();
  });
});
