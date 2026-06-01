const { test, expect } = require("@playwright/test");

// Returns true if the page's browser exposes a usable WebGPU adapter. The client
// renderer needs WebGPU, so gameplay tests skip where it isn't available.
async function hasWebGPU(page) {
  return page.evaluate(async () => {
    if (!navigator.gpu) return false;
    try {
      return !!(await navigator.gpu.requestAdapter());
    } catch {
      return false;
    }
  });
}

test.describe("Pongo smoke (no WebGPU required)", () => {
  test("menu loads, WASM initialises, no fatal errors", async ({ page }) => {
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await page.goto("/");
    await expect(page.locator("h1.game-title")).toHaveText("PONGO");

    // The Play button is disabled until the WASM module has initialised.
    await expect(page.locator("#playBtn")).toBeEnabled({ timeout: 15000 });

    expect(errors, `unexpected page errors: ${errors.join("; ")}`).toHaveLength(0);
  });

  test("'Challenge a friend' generates a 5-character match code", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#playBtn")).toBeEnabled({ timeout: 15000 });

    await page.locator("#createBtn").click();

    // /create returns a code that populates the displayed game code.
    await expect(page.locator("#gameCodeDisplay")).toHaveText(/^[A-Z0-9]{5}$/, {
      timeout: 15000,
    });
  });

  test("'Join with code' form toggles open", async ({ page }) => {
    await page.goto("/");
    await page.locator("#joinCodeToggle").click();
    await expect(page.locator("#joinCodeForm")).toHaveClass(/show/);
  });
});

test.describe("Pongo gameplay (requires WebGPU)", () => {
  test("start a local game, then pause and resume", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#playBtn")).toBeEnabled({ timeout: 15000 });
    test.skip(!(await hasWebGPU(page)), "WebGPU unavailable in this browser/runner");

    await page.locator("#playBtn").click();

    // After the ~3s countdown the local game runs and the Pause button appears.
    const pauseBtn = page.locator("#pauseBtn");
    await expect(pauseBtn).toBeVisible({ timeout: 15000 });
    await expect(pauseBtn).toHaveText("Pause");

    await pauseBtn.click();
    await expect(pauseBtn).toHaveText("Resume");
    await expect(page.locator("#playBtn")).toHaveText("Paused");

    await pauseBtn.click();
    await expect(pauseBtn).toHaveText("Pause");
  });

  test("two players are matched by code", async ({ browser }) => {
    const host = await browser.newContext();
    const guest = await browser.newContext();
    const hostPage = await host.newPage();
    const guestPage = await guest.newPage();

    try {
      await hostPage.goto("/");
      await expect(hostPage.locator("#playBtn")).toBeEnabled({ timeout: 15000 });
      test.skip(!(await hasWebGPU(hostPage)), "WebGPU unavailable in this browser/runner");

      await hostPage.locator("#createBtn").click();
      const codeLoc = hostPage.locator("#gameCodeDisplay");
      await expect(codeLoc).toHaveText(/^[A-Z0-9]{5}$/, { timeout: 15000 });
      const code = (await codeLoc.textContent()).trim();

      // Guest joins via the shared link; both should end up in the match.
      await guestPage.goto(`/?code=${code}`);

      await expect(hostPage.locator("#playBtn")).toHaveText("In Match", { timeout: 20000 });
      await expect(guestPage.locator("#playBtn")).toHaveText("In Match", { timeout: 20000 });
    } finally {
      await host.close();
      await guest.close();
    }
  });
});
