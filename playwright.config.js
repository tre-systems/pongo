const { defineConfig, devices } = require("@playwright/test");

// E2E smoke for Pongo. Core tests (menu, WASM load, match-code generation) run
// anywhere; gameplay tests need WebGPU and skip gracefully where it's unavailable.
// The dev server (wrangler) is started automatically and torn down after the run.
module.exports = defineConfig({
  testDir: "./tests/e2e",
  timeout: 30000,
  fullyParallel: false,
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
        launchOptions: {
          // Best-effort software WebGPU so gameplay tests can run headless.
          args: [
            "--enable-unsafe-webgpu",
            "--enable-features=Vulkan",
            "--use-angle=swiftshader",
            "--use-gl=angle",
          ],
        },
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
