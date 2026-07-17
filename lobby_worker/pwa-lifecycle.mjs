export const UPDATE_CHECK_INTERVAL_MS = 60 * 60 * 1000;
export const UPDATE_CHECK_COOLDOWN_MS = 60 * 1000;
export const UPDATE_ACTIVATION_FALLBACK_MS = 4 * 1000;

export const shouldRunUpdateCheck = (now, lastCheckAt, cooldownMs = UPDATE_CHECK_COOLDOWN_MS) =>
  lastCheckAt === 0 || now - lastCheckAt >= cooldownMs;

export const checkForServiceWorkerUpdate = async ({ registration, swUrl, fetcher = fetch }) => {
  if (registration.waiting) return "waiting";
  if (registration.installing) return "installing";

  try {
    const response = await fetcher(swUrl, {
      cache: "no-store",
      headers: {
        cache: "no-store",
        "cache-control": "no-cache",
      },
    });
    const contentType = response.headers.get("content-type")?.toLowerCase() ?? "";
    if (!response.ok || !contentType.includes("javascript")) return "unavailable";

    await registration.update();
    return registration.waiting ? "waiting" : "current";
  } catch {
    return "unavailable";
  }
};

export const installUpdateCheckTriggers = ({
  check,
  documentTarget = document,
  intervalMs = UPDATE_CHECK_INTERVAL_MS,
  isVisible = () => document.visibilityState === "visible",
  setIntervalFn = window.setInterval.bind(window),
  clearIntervalFn = window.clearInterval.bind(window),
  windowTarget = window,
}) => {
  const checkWhenVisible = () => {
    if (isVisible()) check();
  };
  const intervalId = setIntervalFn(checkWhenVisible, intervalMs);

  documentTarget.addEventListener("visibilitychange", checkWhenVisible);
  windowTarget.addEventListener("focus", checkWhenVisible);
  windowTarget.addEventListener("online", checkWhenVisible);
  windowTarget.addEventListener("pageshow", checkWhenVisible);

  return () => {
    clearIntervalFn(intervalId);
    documentTarget.removeEventListener("visibilitychange", checkWhenVisible);
    windowTarget.removeEventListener("focus", checkWhenVisible);
    windowTarget.removeEventListener("online", checkWhenVisible);
    windowTarget.removeEventListener("pageshow", checkWhenVisible);
  };
};

export const activateWaitingServiceWorker = async ({
  registration,
  reload = () => window.location.reload(),
  fallbackMs = UPDATE_ACTIVATION_FALLBACK_MS,
  setTimeoutFn = setTimeout,
  clearTimeoutFn = clearTimeout,
}) => {
  const waiting = registration.waiting;
  if (!waiting) return false;

  return new Promise((resolve) => {
    let settled = false;
    const finish = (shouldReload) => {
      if (settled) return;
      settled = true;
      clearTimeoutFn(fallbackId);
      waiting.removeEventListener("statechange", onStateChange);
      if (shouldReload) reload();
      resolve(shouldReload);
    };
    const onStateChange = () => {
      if (waiting.state === "activated") finish(true);
      if (waiting.state === "redundant") finish(false);
    };
    const fallbackId = setTimeoutFn(() => finish(true), fallbackMs);

    waiting.addEventListener("statechange", onStateChange);
    waiting.postMessage({ type: "SKIP_WAITING" });
    onStateChange();
  });
};
