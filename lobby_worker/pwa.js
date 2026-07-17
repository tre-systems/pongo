import {
  activateWaitingServiceWorker,
  checkForServiceWorkerUpdate,
  installUpdateCheckTriggers,
  shouldRunUpdateCheck,
} from "./pwa-lifecycle.mjs";

// Progressive Web App wiring: register the service worker and let the player
// apply updates at a safe point. Registration is gated to production so local
// development is never affected by service-worker caching.
const isLocalhost = ["localhost", "127.0.0.1", "[::1]"].includes(location.hostname);

if ("serviceWorker" in navigator && !isLocalhost) {
  window.addEventListener("load", () => {
    navigator.serviceWorker
      .register("/sw.js")
      .then((registration) => {
        let checking = false;
        let lastCheckAt = 0;

        const showWaitingUpdate = () => {
          if (registration.waiting) showUpdateBanner(registration);
        };
        const checkForUpdate = async (force = false) => {
          if (
            checking ||
            !navigator.onLine ||
            document.visibilityState !== "visible" ||
            (!force && !shouldRunUpdateCheck(Date.now(), lastCheckAt))
          ) {
            return;
          }
          lastCheckAt = Date.now();
          checking = true;
          const result = await checkForServiceWorkerUpdate({
            registration,
            swUrl: "/sw.js",
          });
          checking = false;
          if (result === "waiting") showWaitingUpdate();
        };

        registration.addEventListener("updatefound", () => {
          const worker = registration.installing;
          if (!worker) return;
          worker.addEventListener("statechange", () => {
            if (worker.state === "installed" && navigator.serviceWorker.controller) {
              showWaitingUpdate();
            }
          });
        });
        installUpdateCheckTriggers({ check: () => void checkForUpdate() });
        showWaitingUpdate();
        void checkForUpdate(true);
      })
      .catch(() => {});
  });
}

function showUpdateBanner(registration) {
  let banner = document.getElementById("updateBanner");
  if (!banner) {
    banner = document.createElement("div");
    banner.id = "updateBanner";
    document.body.appendChild(banner);
  }

  const renderReady = () => {
    banner.classList.remove("update-deferred");
    banner.setAttribute("role", "status");
    banner.setAttribute("aria-live", "polite");
    banner.innerHTML =
      "<span>Update ready. Reload when you reach a safe point.</span>" +
      '<div class="update-actions">' +
      '<button class="update-now" type="button">Reload now</button>' +
      '<button class="update-later" type="button">Later</button>' +
      "</div>";
    banner.querySelector(".update-now").addEventListener("click", applyUpdate);
    banner.querySelector(".update-later").addEventListener("click", renderDeferred);
  };
  const renderDeferred = () => {
    banner.classList.add("update-deferred");
    banner.removeAttribute("role");
    banner.removeAttribute("aria-live");
    banner.innerHTML = '<button class="update-chip" type="button">Update ready</button>';
    banner.querySelector(".update-chip").addEventListener("click", renderReady);
  };
  const applyUpdate = async () => {
    const button = banner.querySelector(".update-now");
    if (button) {
      button.disabled = true;
      button.textContent = "Updating…";
    }
    const activated = await activateWaitingServiceWorker({ registration });
    if (!activated) renderReady();
  };

  if (!banner.classList.contains("update-deferred")) renderReady();
}
