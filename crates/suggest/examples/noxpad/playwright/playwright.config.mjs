import { defineConfig, devices } from '@playwright/test';

const baseURL = process.env.NOXPAD_BASE_URL ?? 'http://127.0.0.1:44253';
const manageServer = process.env.NOXPAD_MANAGED_SERVER !== '0';

export default defineConfig({
  testDir: './tests',
  timeout: 45_000,
  expect: {
    timeout: 10_000,
  },
  reporter: [['list']],
  use: {
    baseURL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'off',
  },
  webServer: manageServer
    ? {
        command:
          'dx serve -p noxpad --web --port 44253 --open false --interactive false --watch false --hot-reload false',
        url: baseURL,
        timeout: 180_000,
        reuseExistingServer: true,
        cwd: '../../../../../',
      }
    : undefined,
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
  ],
});
