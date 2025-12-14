import { test } from '@playwright/test';

test('capture home page', async ({ page }) => {
  // 设置视口大小
  await page.setViewportSize({ width: 1920, height: 1080 });

  // 访问首页
  await page.goto('http://127.0.0.1:3000');

  // 等待页面加载
  await page.waitForLoadState('networkidle');
  await page.waitForSelector('main', { timeout: 5000 });
  await page.waitForTimeout(2000);

  // 截图（只截取可见区域，避免超过 2000px 限制）
  await page.screenshot({
    path: 'home-current.jpeg',
    type: 'jpeg',
    quality: 90,
    fullPage: false
  });

  console.log('Screenshot saved to home-current.jpeg');
});
