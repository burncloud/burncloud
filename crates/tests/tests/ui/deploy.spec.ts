import { test, expect } from '@playwright/test';

test.describe('Deploy Page', () => {
  test.beforeEach(async ({ page, request }) => {
      const username = `deploy-${Date.now()}`;
      const password = 'password123';
      
      console.log(`Creating test user: ${username}`);
      
      // Register user
      const regResponse = await request.post('/console/api/user/register', { data: { username, password } });
      const regBody = await regResponse.json();
      console.log('Registration response:', regBody);
      
      expect(regResponse.ok()).toBeTruthy();
      expect(regBody.success).toBeTruthy();
      
      // Login
      console.log('Navigating to login page...');
      await page.goto('/login');
      
      // Fill form with robust selectors
      await page.getByPlaceholder('请输入用户名').fill(username);
      await page.getByPlaceholder('请输入密码').fill(password);
      
      console.log('Clicking login button...');
      await page.getByRole('button', { name: '登录' }).click(); 
      
      // Wait for login to complete (usually redirects to dashboard)
      console.log('Waiting for redirect to dashboard...');
      await expect(page).toHaveURL(/\/console\/dashboard/, { timeout: 10000 });
  });

  test('should verify deploy form and successful deployment', async ({ page }) => {
    console.log('Starting deploy form test...');
    // 1. Visit /console/deploy
    await page.goto('/console/deploy');
    
    // Verify title
    await expect(page.getByRole('heading', { name: 'Model Deployment' })).toBeVisible();

    // 2. Verify Form Initial State
    const deployBtn = page.getByRole('button', { name: 'Deploy' });
    const modelInput = page.getByPlaceholder('e.g. gpt2 or organization/model');
    const sourceSelect = page.locator('select');

    await expect(deployBtn).toBeDisabled();
    await expect(sourceSelect).toHaveValue('HuggingFace');

    // 3. Fill Form
    await modelInput.fill('gpt2');
    
    // 4. Verify Button Enabled
    await expect(deployBtn).toBeEnabled();

    // 5. Click Deploy
    console.log('Clicking deploy button...');
    await deployBtn.click();
    
    // 6. Verify Navigation to /console/models
    // Note: The app adds /console prefix in routes probably
    await expect(page).toHaveURL(/\/console\/models/);

    // 7. Verify Toast
    await expect(page.getByText('Deployment Successful')).toBeVisible();
    console.log('Deployment successful toast verified.');
  });
});
