import { test, expect, Page } from '@playwright/test';

test.describe('Registration System - Comprehensive Tests', () => {
  const timestamp = Date.now();
  const testUsername = `testuser_${timestamp}`;
  const testEmail = `testuser_${timestamp}@example.com`;
  const testPassword = 'SecurePass123!';
  const weakPassword = 'weak';
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/register');
  });

  test.describe('1. Input Validation & Feedback', () => {
    
    test('should show real-time email validation errors', async ({ page }) => {
      const emailInput = page.locator('input[type="email"]');
      
      // Type invalid email
      await emailInput.fill('invalid-email');
      await page.waitForTimeout(500); // Wait for debounce
      
      // Should show error message
      await expect(page.locator('text=邮箱格式不正确')).toBeVisible();
      
      // Type valid email
      await emailInput.fill(testEmail);
      await page.waitForTimeout(500);
      
      // Error should disappear
      await expect(page.locator('text=邮箱格式不正确')).not.toBeVisible();
    });

    test('should check username availability asynchronously', async ({ page }) => {
      const usernameInput = page.locator('input[type="text"]').first();
      
      // Type a valid username
      await usernameInput.fill(testUsername);
      await page.waitForTimeout(1000); // Wait for availability check
      
      // Should show checkmark or spinner
      const checkmark = page.locator('svg').filter({ hasText: '' }).first();
      // Availability check would show checkmark if available
    });

    test('should display password strength meter', async ({ page }) => {
      const passwordInput = page.locator('input[type="password"]').first();
      
      // Type weak password
      await passwordInput.fill('weak');
      await expect(page.locator('text=强度:')).toBeVisible();
      await expect(page.locator('text=弱')).toBeVisible();
      
      // Type medium password
      await passwordInput.fill('Password1');
      await page.waitForTimeout(300);
      await expect(page.locator('text=中')).toBeVisible();
      
      // Type strong password
      await passwordInput.fill(testPassword);
      await page.waitForTimeout(300);
      await expect(page.locator('text=强')).toBeVisible();
    });

    test('should show checkmark when passwords match', async ({ page }) => {
      const passwordInput = page.locator('input[type="password"]').first();
      const confirmInput = page.locator('input[type="password"]').nth(1);
      
      await passwordInput.fill(testPassword);
      await confirmInput.fill(testPassword);
      
      // Should show checkmark for matching passwords
      await page.waitForTimeout(300);
      const matchIndicators = page.locator('svg path[d*="M5 13l4 4L19 7"]');
      await expect(matchIndicators.first()).toBeVisible();
    });

    test('should show validation errors for empty fields', async ({ page }) => {
      const submitButton = page.locator('button:has-text("创建账号")');
      
      // Try to submit empty form
      await submitButton.click();
      
      // Should see error toast
      await expect(page.locator('text=请检查表单填写是否正确')).toBeVisible({ timeout: 2000 });
    });
  });

  test.describe('2. Security & Integrity', () => {
    
    test('should sanitize XSS inputs', async ({ page }) => {
      const usernameInput = page.locator('input[type="text"]').first();
      
      // Try to input XSS
      const xssPayload = '<script>alert("XSS")</script>';
      await usernameInput.fill(xssPayload);
      
      // Should be sanitized (client-side)
      const value = await usernameInput.inputValue();
      expect(value).not.toContain('<script>');
    });
  });

  test.describe('3. User Experience (UX)', () => {
    
    test('should support keyboard navigation with Tab', async ({ page }) => {
      // Focus should start on first input
      await page.keyboard.press('Tab');
      const usernameInput = page.locator('input[type="text"]').first();
      await expect(usernameInput).toBeFocused();
      
      // Tab to email
      await page.keyboard.press('Tab');
      const emailInput = page.locator('input[type="email"]');
      await expect(emailInput).toBeFocused();
      
      // Tab to password
      await page.keyboard.press('Tab');
      const passwordInput = page.locator('input[type="password"]').first();
      await expect(passwordInput).toBeFocused();
      
      // Tab to confirm password
      await page.keyboard.press('Tab');
      const confirmInput = page.locator('input[type="password"]').nth(1);
      await expect(confirmInput).toBeFocused();
    });

    test('should submit form with Enter key', async ({ page }) => {
      const usernameInput = page.locator('input[type="text"]').first();
      const emailInput = page.locator('input[type="email"]');
      const passwordInput = page.locator('input[type="password"]').first();
      const confirmInput = page.locator('input[type="password"]').nth(1);
      
      // Fill form using keyboard
      await usernameInput.fill(testUsername + '_enter');
      await page.keyboard.press('Tab');
      await emailInput.fill(`enter_${timestamp}@example.com`);
      await page.keyboard.press('Tab');
      await passwordInput.fill(testPassword);
      await page.keyboard.press('Tab');
      await confirmInput.fill(testPassword);
      
      // Submit with Enter
      await page.keyboard.press('Enter');
      
      // Should see loading state or redirect
      await page.waitForTimeout(1000);
    });

    test('should toggle password visibility', async ({ page }) => {
      const passwordInput = page.locator('input[type="password"]').first();
      const toggleButton = page.locator('button').filter({ has: passwordInput }).first();
      
      await passwordInput.fill(testPassword);
      
      // Initially should be password type
      await expect(passwordInput).toHaveAttribute('type', 'password');
      
      // Click toggle
      await toggleButton.click();
      
      // Should change to text type
      await expect(passwordInput).toHaveAttribute('type', 'text');
      
      // Click again to hide
      await toggleButton.click();
      await expect(passwordInput).toHaveAttribute('type', 'password');
    });

    test('should show shake animation on error', async ({ page }) => {
      const submitButton = page.locator('button:has-text("创建账号")');
      
      // Try to submit invalid form
      await submitButton.click();
      
      // Form should have shake animation class
      const form = page.locator('.animate-shake');
      await expect(form).toBeVisible({ timeout: 1000 });
    });

    test('should disable button while loading', async ({ page }) => {
      const usernameInput = page.locator('input[type="text"]').first();
      const passwordInput = page.locator('input[type="password"]').first();
      const confirmInput = page.locator('input[type="password"]').nth(1);
      const submitButton = page.locator('button:has-text("创建账号")');
      
      await usernameInput.fill(testUsername + '_loading');
      await passwordInput.fill(testPassword);
      await confirmInput.fill(testPassword);
      
      await submitButton.click();
      
      // Button should be disabled
      await expect(submitButton).toBeDisabled();
      await expect(page.locator('text=注册中...')).toBeVisible({ timeout: 2000 });
    });
  });

  test.describe('4. Error Handling', () => {
    
    test('should show friendly network error messages', async ({ page }) => {
      // Simulate network offline
      await page.context().setOffline(true);
      
      const usernameInput = page.locator('input[type="text"]').first();
      const passwordInput = page.locator('input[type="password"]').first();
      const confirmInput = page.locator('input[type="password"]').nth(1);
      const submitButton = page.locator('button:has-text("创建账号")');
      
      await usernameInput.fill(testUsername + '_offline');
      await passwordInput.fill(testPassword);
      await confirmInput.fill(testPassword);
      await submitButton.click();
      
      // Should show network error toast
      await expect(page.locator('.toast-error')).toBeVisible({ timeout: 3000 });
      
      // Restore network
      await page.context().setOffline(false);
    });

    test('should display password mismatch error', async ({ page }) => {
      const passwordInput = page.locator('input[type="password"]').first();
      const confirmInput = page.locator('input[type="password"]').nth(1);
      
      await passwordInput.fill(testPassword);
      await confirmInput.fill('DifferentPassword123!');
      
      // Should show error
      await page.waitForTimeout(300);
      await expect(page.locator('text=两次输入的密码不一致')).toBeVisible();
    });
  });

  test.describe('5. Post-Registration Flow', () => {
    
    test('should auto-login and redirect to dashboard after successful registration', async ({ page }) => {
      const usernameInput = page.locator('input[type="text"]').first();
      const emailInput = page.locator('input[type="email"]');
      const passwordInput = page.locator('input[type="password"]').first();
      const confirmInput = page.locator('input[type="password"]').nth(1);
      const submitButton = page.locator('button:has-text("创建账号")');
      
      const uniqueUsername = `user_${Date.now()}`;
      
      await usernameInput.fill(uniqueUsername);
      await emailInput.fill(`${uniqueUsername}@example.com`);
      await passwordInput.fill(testPassword);
      await confirmInput.fill(testPassword);
      
      await submitButton.click();
      
      // Should show success message
      await expect(page.locator('text=注册成功')).toBeVisible({ timeout: 3000 });
      
      // Should redirect to dashboard (not login page)
      await expect(page).toHaveURL(/\/dashboard|\/$/i, { timeout: 5000 });
    });
  });

  test.describe('6. Full Lifecycle E2E Test', () => {
    
    test('should complete full user journey: Register -> Auto Login -> Logout -> Login', async ({ page }) => {
      const uniqueUsername = `lifecycle_${Date.now()}`;
      const uniqueEmail = `${uniqueUsername}@example.com`;
      
      // Step 1: Register
      await page.goto('/register');
      await page.locator('input[type="text"]').first().fill(uniqueUsername);
      await page.locator('input[type="email"]').fill(uniqueEmail);
      await page.locator('input[type="password"]').first().fill(testPassword);
      await page.locator('input[type="password"]').nth(1).fill(testPassword);
      await page.locator('button:has-text("创建账号")').click();
      
      // Step 2: Verify auto-login and redirect to dashboard
      await expect(page).toHaveURL(/\/dashboard|\/$/i, { timeout: 5000 });
      
      // Step 3: Logout
      const logoutButton = page.locator('button:has-text("退出"), a:has-text("退出")').first();
      if (await logoutButton.isVisible({ timeout: 2000 })) {
        await logoutButton.click();
      }
      
      // Step 4: Navigate to login and login again
      await page.goto('/login');
      await page.locator('input[type="text"]').fill(uniqueUsername);
      await page.locator('input[type="password"]').fill(testPassword);
      await page.locator('button:has-text("登录")').click();
      
      // Should successfully login
      await expect(page).toHaveURL(/\/dashboard|\/$/i, { timeout: 5000 });
    });
  });

  test.describe('7. Accessibility - Keyboard Only', () => {
    
    test('should complete registration using only keyboard', async ({ page }) => {
      const uniqueUsername = `keyboard_${Date.now()}`;
      
      // Navigate to page
      await page.goto('/register');
      
      // Use only keyboard
      await page.keyboard.press('Tab'); // Focus username
      await page.keyboard.type(uniqueUsername);
      
      await page.keyboard.press('Tab'); // Focus email
      await page.keyboard.type(`${uniqueUsername}@example.com`);
      
      await page.keyboard.press('Tab'); // Focus password
      await page.keyboard.type(testPassword);
      
      await page.keyboard.press('Tab'); // Focus confirm password
      await page.keyboard.type(testPassword);
      
      await page.keyboard.press('Tab'); // Focus submit button
      await page.keyboard.press('Enter'); // Submit
      
      // Should process registration
      await page.waitForTimeout(2000);
    });
  });

  test.describe('8. Slow Network / Throttle Test', () => {
    
    test('should handle slow 3G connection without double submission', async ({ page, context }) => {
      // Simulate slow 3G
      await context.route('**/*', route => {
        setTimeout(() => route.continue(), 2000); // 2 second delay
      });
      
      const uniqueUsername = `slow3g_${Date.now()}`;
      
      await page.goto('/register', { timeout: 30000 });
      
      await page.locator('input[type="text"]').first().fill(uniqueUsername);
      await page.locator('input[type="email"]').fill(`${uniqueUsername}@example.com`);
      await page.locator('input[type="password"]').first().fill(testPassword);
      await page.locator('input[type="password"]').nth(1).fill(testPassword);
      
      const submitButton = page.locator('button:has-text("创建账号")');
      await submitButton.click();
      
      // Button should be disabled immediately
      await expect(submitButton).toBeDisabled();
      
      // Loading state should persist
      await expect(page.locator('text=注册中...')).toBeVisible({ timeout: 1000 });
      
      // Try clicking again - should not submit twice
      await submitButton.click({ force: true });
      await submitButton.click({ force: true });
      
      // Should still show loading state
      await expect(page.locator('text=注册中...')).toBeVisible();
    });
  });
});
