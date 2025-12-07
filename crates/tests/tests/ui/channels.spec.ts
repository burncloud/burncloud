import { test, expect } from '@playwright/test';

test.describe('Channel Management', () => {
  test.beforeEach(async ({ page, request }) => {
      const username = `chan-${Date.now()}`;
      const password = 'password123';
      
      await request.post('/console/api/user/register', { data: { username, password } });
      
      await page.goto('/login');
      await page.fill('input[type="text"]', username);
      await page.fill('input[type="password"]', password);
      await page.click('button');
      await expect(page).toHaveURL(/\/$/);
  });

  test('should list created channels', async ({ page, request }) => {
    const channelName = `UI-Test-${Date.now()}`;
    
    // Seed Channel via API
    const response = await request.post('/console/api/channel', {
        data: {
            type: 1,
            key: "sk-test-ui",
            name: channelName,
            base_url: "https://api.openai.com",
            models: "gpt-3.5-turbo",
            group: "default",
            weight: 10,
            priority: 100
        }
    });
    expect(response.ok()).toBeTruthy();

    // Navigate to channels page
    // Assuming there is a sidebar link or we can go directly
    // The old test clicked a link: tab.wait_for_element("a[href*='channels']")
    
    // Try clicking sidebar if it exists
    const channelLink = page.locator('a[href*="channels"]');
    if (await channelLink.isVisible()) {
        await channelLink.click();
    } else {
        // Fallback to direct navigation if link not found (e.g. mobile view or collapsed)
        await page.goto('/channels');
    }
    
    // Verify channel name is present in the table/list
    await expect(page.locator('body')).toContainText(channelName);
  });
});
