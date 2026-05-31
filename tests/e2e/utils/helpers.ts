/**
 * General helper utilities for E2E tests
 */

import { Page, expect } from '@playwright/test';

/**
 * Wait for a specified time (use sparingly, prefer built-in auto-waiting)
 */
export async function waitFor(ms: number): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Check if element exists
 */
export async function elementExists(page: Page, selector: string): Promise<boolean> {
  const count = await page.locator(selector).count();
  return count > 0;
}

/**
 * Click an element and wait for navigation
 */
export async function clickAndWait(page: Page, selector: string, waitForUrl?: string | RegExp): Promise<void> {
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'networkidle' }),
    page.click(selector),
  ]);
}

/**
 * Fill a form with multiple fields
 */
export async function fillForm(page: Page, fields: Record<string, string>): Promise<void> {
  for (const [selector, value] of Object.entries(fields)) {
    await page.locator(selector).fill(value);
  }
}

/**
 * Check if API response is successful
 */
export async function checkApiSuccess(page: Page, expectedPath: string): Promise<void> {
  const response = await page.waitForResponse(resp => 
    resp.url().includes(expectedPath) && resp.ok()
  );
  expect(response.ok()).toBeTruthy();
}

/**
 * Navigate to a page using the sidebar menu
 */
export async function navigateViaSidebar(page: Page, menuText: string): Promise<void> {
  const menuLink = page.locator(`nav a:has-text("${menuText}"), [data-testid="sidebar"] a:has-text("${menuText}")`);
  
  if (await menuLink.isVisible({ timeout: 2000 }).catch(() => false)) {
    await menuLink.click();
    await page.waitForLoadState('networkidle');
  } else {
    throw new Error(`Menu item "${menuText}" not found`);
  }
}

/**
 * Handle confirmation modal
 */
export async function confirmModal(page: Page, confirm: boolean = true): Promise<void> {
  const buttonText = confirm ? 'Confirm' : 'Cancel';
  const button = page.locator(`.modal button:has-text("${buttonText}"), [role="dialog"] button:has-text("${buttonText}")`);
  
  if (await button.isVisible({ timeout: 2000 }).catch(() => false)) {
    await button.click();
  }
}

/**
 * Get text content of an element
 */
export async function getElementText(page: Page, selector: string): Promise<string> {
  return await page.locator(selector).textContent() || '';
}

/**
 * Check if toast/notification appears
 */
export async function checkToast(page: Page, expectedText: string): Promise<boolean> {
  const toast = page.locator(`.toast, [role="alert"], .notification:has-text("${expectedText}")`);
  return await toast.isVisible({ timeout: 3000 }).catch(() => false);
}

/**
 * Table utilities
 */
export async function getTableRowCount(page: Page, tableSelector: string): Promise<number> {
  return await page.locator(`${tableSelector} tbody tr, ${tableSelector} tr`).count();
}

export async function clickTableRow(page: Page, tableSelector: string, rowText: string): Promise<void> {
  const row = page.locator(`${tableSelector} tr:has-text("${rowText}")`);
  await row.click();
}

/**
 * Screenshot helper for debugging
 */
export async function takeDebugScreenshot(page: Page, name: string): Promise<void> {
  await page.screenshot({ path: `test-results/debug-${name}.png`, fullPage: true });
}
