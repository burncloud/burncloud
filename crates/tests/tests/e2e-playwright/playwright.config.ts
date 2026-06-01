import { defineConfig, devices } from '@playwright/test';

/**
 * BurnCloud E2E Test Configuration
 * 
 * Tests the Dioxus web frontend compiled to WebAssembly
 * 
 * Prerequisites:
 * 1. Build the frontend: cd crates/client && dx build --release --features web
 * 2. Start the server: ./target/release/burncloud server
 * 3. Run tests: npm test
 */
export default defineConfig({
  testDir: './specs',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  
  use: {
    baseURL: process.env.E2E_BASE_URL || 'http://localhost:3334',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
  ],

  webServer: {
    command: process.env.E2E_SERVER_COMMAND || '../../../../target/release/burncloud server',
    url: 'http://localhost:3334/health',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
