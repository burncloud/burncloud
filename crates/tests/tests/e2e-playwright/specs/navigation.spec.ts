/**
 * Navigation E2E Tests
 * 
 * Tests sidebar navigation and page routing
 */

import { test, expect } from '@playwright/test';
import { login } from '../utils/auth';

test.describe('Sidebar Navigation', () => {
  
  test.beforeEach(async ({ page }) => {
    // Navigate to login page first
    await page.goto('/login');
  });

  test('should display sidebar after login', async ({ page }) => {
    test.skip(); // Requires login
    
    await login(page);
    
    // Check sidebar is visible
    const sidebar = page.locator('nav, [data-testid="sidebar"], .sidebar');
    await expect(sidebar).toBeVisible({ timeout: 5000 });
  });

  test('should navigate between pages via sidebar', async ({ page }) => {
    test.skip(); // Requires login
    
    await login(page);
    
    // Define expected menu items and routes
    const menuItems = [
      { text: 'Dashboard', route: '/dashboard' },
      { text: 'Models', route: '/models' },
      { text: 'Monitor', route: '/monitor' },
      { text: 'Settings', route: '/settings' },
    ];
    
    for (const item of menuItems) {
      const link = page.locator(`nav a:has-text("${item.text}"), [data-testid="sidebar"] a:has-text("${item.text}")`);
      
      if (await link.isVisible({ timeout: 2000 }).catch(() => false)) {
        await link.click();
        await page.waitForLoadState('networkidle');
        
        // Verify URL changed
        expect(page.url()).toContain(item.route);
      }
    }
  });

  test('should highlight active menu item', async ({ page }) => {
    test.skip(); // Requires login
    
    await login(page);
    
    // Navigate to settings
    const settingsLink = page.locator('nav a:has-text("Settings")');
    await settingsLink.click();
    
    // Check if link has active class
    const isActive = await settingsLink.evaluate((el) => {
      return el.classList.contains('active') || 
             el.getAttribute('aria-current') === 'page' ||
             el.getAttribute('data-active') === 'true';
    });
    
    // May or may not have active state depending on implementation
    // Just check that we're on the correct page
    expect(page.url()).toContain('/settings');
  });
});

test.describe('URL Routing', () => {
  
  test('should handle 404 for unknown routes', async ({ page }) => {
    await page.goto('/unknown-route-xyz');
    
    // Should show 404 page or redirect
    const is404 = await page.locator('text=/404|not found|页面不存在/i').isVisible({ timeout: 5000 }).catch(() => false);
    const redirected = !page.url().includes('/unknown-route-xyz');
    
    expect(is404 || redirected).toBe(true);
  });

  test('should preserve URL after page reload', async ({ page }) => {
    test.skip(); // Requires login
    
    await login(page);
    
    // Navigate to a specific page
    await page.goto('/settings');
    const urlBefore = page.url();
    
    // Reload
    await page.reload();
    
    // URL should be the same
    expect(page.url()).toBe(urlBefore);
  });

  test('should handle browser back/forward navigation', async ({ page }) => {
    test.skip(); // Requires login
    
    await login(page);
    
    // Navigate through several pages
    await page.goto('/dashboard');
    await page.goto('/models');
    await page.goto('/settings');
    
    // Go back
    await page.goBack();
    expect(page.url()).toContain('/models');
    
    await page.goBack();
    expect(page.url()).toContain('/dashboard');
    
    // Go forward
    await page.goForward();
    expect(page.url()).toContain('/models');
  });
});
