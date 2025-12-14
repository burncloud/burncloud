import { test, expect } from '@playwright/test';

test.describe('Channel Management UI', () => {
  test.beforeEach(async ({ page, request }) => {
      // Login first
      const username = `chan-ui-${Date.now()}`;
      const password = 'password123';
      await request.post('/console/api/user/register', { data: { username, password } });
      await page.goto('/login');
      await page.fill('input[type="text"]', username);
      await page.fill('input[type="password"]', password);
      await page.click('button');
      await expect(page).toHaveURL(/\/$/);
      
      // Navigate to Channels page
      await page.goto('/channels'); // Adjust URL if necessary based on router
  });

  test('should open create modal with modern design', async ({ page }) => {
    // Click "新建渠道" button
    await page.getByRole('button', { name: '新建渠道' }).click();

    // Check for modal overlay with backdrop blur
    const overlay = page.locator('.fixed.inset-0.bg-black\/40.backdrop-blur-sm');
    await expect(overlay).toBeVisible();

    // Check for modal box with rounded corners and shadow
    const modalBox = page.locator('.bg-base-100.rounded-xl.shadow-2xl');
    await expect(modalBox).toBeVisible();

    // Check for inputs with DaisyUI classes
    const nameInput = page.getByPlaceholder('e.g. OpenAI Main');
    await expect(nameInput).toBeVisible();
    await expect(nameInput).toHaveClass(/input input-bordered/);

    // Check for Cancel button
    await page.getByRole('button', { name: '取消' }).click();
    await expect(overlay).toBeHidden();
  });
});
