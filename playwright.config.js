const { defineConfig, devices } = require("@playwright/test");

// E2E smoke for Pongo. All tests — menu, WASM load, match-code generation, local
// gameplay/pause, and two-player matchmaking — run anywhere (Canvas2D needs no GPU).
// The dev server (wrangler) is started automatically and torn down after the run.
module.exports = defineConfig({
  testDir: "./tests/e2e",
  timeout: 30000,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: "list",
  use: {
    baseURL: "http://localhost:8787",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
      },
    },
  ],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:8787",
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
    env: { WRANGLER_SEND_METRICS: "false" },
  },
});
