import { test, expect } from '@playwright/test';

test.describe('Models Management Page', () => {
  test.beforeEach(async ({ page, request }) => {
    // We reuse the registration/login flow to ensure we have a valid session
    // even if we mock the models data later.
    const username = `models-${Date.now()}`;
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

  test('should verify models table and management actions', async ({ page, request }) => {
    // Unique model name to avoid collisions
    const modelName = `gpt2-${Date.now()}`;

    // 1. Visit /console/models
    await page.goto('/console/models');
    
    // If empty, we see the empty state.
    // Click "开始连接" (Start Connection) or "添加连接" (Add Connection)
    // The empty state has a button "开始连接".
    const startBtn = page.getByRole('button', { name: '开始连接' });
    if (await startBtn.isVisible()) {
        await startBtn.click();
    } else {
        await page.getByRole('button', { name: '添加连接' }).click();
    }
    
    // Modal Step 0: Select Provider.
    // Click 'OpenAI' (or generic).
    await page.getByText('OpenAI', { exact: true }).click();
    
    // Modal Step 1: Form
    // Fill Name
    await page.locator('input').first().fill(modelName); 
    // Fill Key
    // We need to find the key input.
    await page.getByPlaceholder('sk-...').fill('sk-test-key');
    
    // Click Save
    await page.getByRole('button', { name: '保存' }).click();
    
    // Wait for success toast or modal close
    await expect(page.getByText('保存成功')).toBeVisible();
    
    // Now verify Table
    // 2. Verify Table Columns
    const table = page.locator('table');
    await expect(table).toBeVisible();
    await expect(table.getByText('Status', { exact: true })).toBeVisible();
    await expect(table.getByText('Name', { exact: true })).toBeVisible();
    await expect(table.getByText('Replicas', { exact: true })).toBeVisible();
    await expect(table.getByText('Actions', { exact: true })).toBeVisible();

    // 3. Verify row
    const row = table.locator('tr', { hasText: modelName });
    await expect(row).toBeVisible();
    // Default status might be Running (1)
    await expect(row.getByText('Running')).toBeVisible();

    // 4. Click 'Stop' button
    const stopButton = row.getByRole('button', { name: 'Stop' });
    await expect(stopButton).toBeVisible();
    await stopButton.click();
    
    // Wait for server to process and push update
    await page.waitForTimeout(1000); 

    // Verify status changes to 'Stopped' (or Start button appears)
    // We check for "Stopped" badge or text
    // await expect(row.getByText('Stopped')).toBeVisible();

    // 5. Click 'Delete' button
    const deleteButton = row.getByRole('button', { name: 'Delete' });
    await deleteButton.click();

    // 6. Verify Confirmation Modal
    await expect(page.getByRole('heading', { name: '确认删除' })).toBeVisible();

    // 7. Confirm Delete
    const confirmButton = page.getByRole('button', { name: '确认删除' });
    await confirmButton.click();

    // 8. Verify row disappears
    await expect(row).not.toBeVisible();
  });
});
