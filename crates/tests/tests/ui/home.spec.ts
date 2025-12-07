import { test, expect } from '@playwright/test';

test('Homepage should load', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveTitle(/BurnCloud|Index/);
});
