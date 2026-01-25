import { test, expect } from '@playwright/test';

test.describe('Models Management Page (Mocked)', () => {
  test.beforeEach(async ({ page }) => {
    // 1. Mock Login/Auth-related endpoints to bypass real backend
    // Assuming the app checks some endpoint or just relies on local state after login.
    // We'll simulate a successful login flow or pre-set state if possible.
    // For now, let's mock the login request and any user info request.
    
    // Mock Login Endpoint
    await page.route('**/console/api/user/login', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ token: 'mock-token', user: { username: 'admin' } })
      });
    });

    // Mock User Info / Me endpoint (if it exists, often used for session check)
    // Based on previous file analysis, we saw /console/api/user/...
    await page.route('**/console/api/user/info', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ username: 'admin', role: 'admin' })
      });
    });

    // 2. Visit the page. 
    // If the app redirects to login when unauthenticated, we might need to manually trigger "login".
    // Or we can inject a token into localStorage if we knew the key.
    // Strategy: Visit login, mock the click, then go to models.
    await page.goto('/login');
    
    // Fill dummy credentials
    await page.getByPlaceholder('请输入用户名').fill('admin');
    await page.getByPlaceholder('请输入密码').fill('password');
    
    // Click Login - this triggers the mocked POST /login
    await page.getByRole('button', { name: '登录' }).click();
    
    // Wait for navigation or success state
    await expect(page).toHaveURL(/\/console\/dashboard/);

    // Now go to Models page
    await page.goto('/console/models');
  });

  test('should verify models list and delete action', async ({ page }) => {
    // 3. Mock GET /api/v1/models
    await page.route('**/api/v1/models', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: [
            {
              id: 'gpt2',
              object: 'model',
              created: 1678888888,
              owned_by: 'system',
              // Add extra fields that the UI might use
              status: 'Running', 
              replicas: 1
            }
          ]
        })
      });
    });

    // 4. Mock DELETE /api/v1/models/gpt2
    await page.route('**/api/v1/models/gpt2', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, id: 'gpt2', deleted: true })
      });
    });

    // Reload to trigger the GET request if needed, or if goto() above already did it.
    // If /console/models was visited in beforeEach, the GET might have fired before the route was set up in this test block?
    // Actually, beforeEach runs before the test block. 
    // BUT, the route handler for '**/api/v1/models' is defined INSIDE the test(), 
    // while the page.goto('/console/models') is in beforeEach(). 
    // This is a RACE CONDITION or LOGIC ERROR. 
    // The GET request happens on page load. If the route isn't set up yet, it hits the real network or fails.
    
    // CORRECTION: Move the specific mocks to beforeEach OR reload page inside the test.
    // I will simply reload the page here to ensure the Mock is hit.
    await page.reload();

    // 5. Verify Table Headers
    const table = page.locator('table');
    await expect(table).toBeVisible();
    await expect(table.getByText('Status', { exact: true })).toBeVisible();
    await expect(table.getByText('Name', { exact: true })).toBeVisible();
    await expect(table.getByText('Replicas', { exact: true })).toBeVisible();
    await expect(table.getByText('Actions', { exact: true })).toBeVisible();

    // 6. Verify Row Content
    const row = table.locator('tr', { hasText: 'gpt2' });
    await expect(row).toBeVisible();
    // Verify Status 'Running'
    await expect(row.getByText('Running')).toBeVisible();

    // 7. Click Delete Button
    const deleteBtn = row.getByRole('button', { name: 'Delete' });
    await expect(deleteBtn).toBeVisible();
    await deleteBtn.click();

    // 8. Verify Confirmation Modal
    // Assuming the modal has a heading "确认删除" or similar, and a confirm button.
    // Based on previous file, it was '确认删除'.
    await expect(page.getByRole('heading', { name: '确认删除' })).toBeVisible();

    // 9. Confirm Delete
    const confirmBtn = page.getByRole('button', { name: '确认删除' });
    await confirmBtn.click();

    // 10. Verify Row Removal
    // Wait for the DELETE request to complete (already mocked) and UI update
    await expect(row).not.toBeVisible();
  });
});