import { test, expect } from '@playwright/test';

test.describe('Playground Page', () => {
  test.beforeEach(async ({ page, request }) => {
      const username = `play-${Date.now()}`;
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

  test('should verify playground interaction', async ({ page }) => {
    // 1. Visit /console/playground
    await page.goto('/console/playground');
    
    // 2. Verify model selector
    // Assuming there is a select/combobox for model
    const modelSelect = page.locator('select, [role="combobox"]');
    await expect(modelSelect).toBeVisible();

    // 3. Input test text
    const input = page.getByPlaceholder('Type a message...');
    const sendBtn = page.getByRole('button', { name: 'Send' });

    // Ensure initial state
    await expect(input).toBeVisible();
    
    await input.fill('Hello World');
    
    // 4. Verify Send button enabled
    await expect(sendBtn).toBeEnabled();

    // 5. Click Send
    await sendBtn.click();
    
    // 6. Verify input cleared and user message appears
    await expect(input).toBeEmpty();
    await expect(page.getByText('Hello World')).toBeVisible();

    // 7. Wait for AI reply
    // The mock returns "This is a mocked AI response."
    await expect(page.getByText('This is a mocked AI response.')).toBeVisible();
    
    // 8. Verify history (implicit if messages are visible)
    // We can check if there are at least 2 messages
    // Adjust selector based on actual UI implementation
    // const messages = page.locator('.message-bubble');
    // await expect(messages).toHaveCount(2); 
  });
});
