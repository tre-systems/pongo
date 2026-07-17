import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  activateWaitingServiceWorker,
  checkForServiceWorkerUpdate,
  installUpdateCheckTriggers,
  shouldRunUpdateCheck,
} from "../lobby_worker/pwa-lifecycle.mjs";

class FakeWorker extends EventTarget {
  messages = [];
  state = "installed";

  postMessage(message) {
    this.messages.push(message);
  }
}

const registrationWith = (overrides = {}) => ({
  installing: null,
  waiting: null,
  async update() {
    return this;
  },
  ...overrides,
});

test("foreground checks are cooldown-limited", () => {
  assert.equal(shouldRunUpdateCheck(10_000, 0), true);
  assert.equal(shouldRunUpdateCheck(65_000, 10_000), false);
  assert.equal(shouldRunUpdateCheck(70_000, 10_000), true);
});

test("checks sw.js without cache before updating", async () => {
  let updateCalls = 0;
  const registration = registrationWith({
    async update() {
      updateCalls += 1;
    },
  });
  let request;
  const result = await checkForServiceWorkerUpdate({
    registration,
    swUrl: "/sw.js",
    fetcher: async (...args) => {
      request = args;
      return new Response("worker", {
        headers: { "content-type": "text/javascript" },
      });
    },
  });

  assert.equal(result, "current");
  assert.deepEqual(request, [
    "/sw.js",
    {
      cache: "no-store",
      headers: { cache: "no-store", "cache-control": "no-cache" },
    },
  ]);
  assert.equal(updateCalls, 1);
});

test("surfaces a waiting worker without a network request", async () => {
  let fetched = false;
  const result = await checkForServiceWorkerUpdate({
    registration: registrationWith({ waiting: new FakeWorker() }),
    swUrl: "/sw.js",
    fetcher: async () => {
      fetched = true;
    },
  });
  assert.equal(result, "waiting");
  assert.equal(fetched, false);
});

test("installs all visible lifecycle triggers and removes them", () => {
  const windowTarget = new EventTarget();
  const documentTarget = new EventTarget();
  let visible = true;
  let checks = 0;
  let intervalCallback;
  const cleanup = installUpdateCheckTriggers({
    check: () => {
      checks += 1;
    },
    documentTarget,
    isVisible: () => visible,
    setIntervalFn: (callback) => {
      intervalCallback = callback;
      return 1;
    },
    clearIntervalFn: () => {},
    windowTarget,
  });

  documentTarget.dispatchEvent(new Event("visibilitychange"));
  windowTarget.dispatchEvent(new Event("focus"));
  windowTarget.dispatchEvent(new Event("online"));
  windowTarget.dispatchEvent(new Event("pageshow"));
  intervalCallback();
  assert.equal(checks, 5);

  visible = false;
  windowTarget.dispatchEvent(new Event("focus"));
  intervalCallback();
  assert.equal(checks, 5);

  cleanup();
  visible = true;
  windowTarget.dispatchEvent(new Event("online"));
  assert.equal(checks, 5);
});

test("activates the exact waiting worker and reloads", async () => {
  const waiting = new FakeWorker();
  let reloads = 0;
  const activation = activateWaitingServiceWorker({
    registration: registrationWith({ waiting }),
    reload: () => {
      reloads += 1;
    },
  });

  assert.deepEqual(waiting.messages, [{ type: "SKIP_WAITING" }]);
  waiting.state = "activated";
  waiting.dispatchEvent(new Event("statechange"));
  assert.equal(await activation, true);
  assert.equal(reloads, 1);
});

test("uses a bounded fallback if activation events are missed", async () => {
  const waiting = new FakeWorker();
  let reloads = 0;
  let fallback;
  const activation = activateWaitingServiceWorker({
    registration: registrationWith({ waiting }),
    reload: () => {
      reloads += 1;
    },
    setTimeoutFn: (callback) => {
      fallback = callback;
      return 1;
    },
    clearTimeoutFn: () => {},
  });

  fallback();
  assert.equal(await activation, true);
  assert.equal(reloads, 1);
});

test("the service worker waits until the page explicitly accepts the update", async () => {
  const source = await readFile(new URL("../lobby_worker/sw.js", import.meta.url), "utf8");
  const installHandler = source.slice(
    source.indexOf('self.addEventListener("install"'),
    source.indexOf('self.addEventListener("message"')
  );

  assert.doesNotMatch(installHandler, /skipWaiting/);
  assert.match(source, /event\.data\?\.type === "SKIP_WAITING"/);
});
