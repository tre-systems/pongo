// Progressive Web App wiring: register the service worker and let the player
// apply updates on their terms. Registration is gated to production so local dev
// and the Playwright e2e suite are never affected by service-worker caching.
const isLocalhost = ["localhost", "127.0.0.1", "[::1]"].includes(location.hostname);

if ("serviceWorker" in navigator && !isLocalhost) {
  window.addEventListener("load", () => {
    navigator.serviceWorker
      .register("/sw.js")
      .then((registration) => {
        registration.addEventListener("updatefound", () => {
          const worker = registration.installing;
          if (!worker) return;
          worker.addEventListener("statechange", () => {
            // A new worker installed while an old one still controls the page →
            // an update is ready. Prompt rather than reloading out from under play.
            if (worker.state === "installed" && navigator.serviceWorker.controller) {
              showUpdateBanner();
            }
          });
        });
      })
      .catch(() => {});
  });
}

function showUpdateBanner() {
  if (document.getElementById("updateBanner")) return;
  const banner = document.createElement("div");
  banner.id = "updateBanner";
  banner.innerHTML =
    "<span>A new version of Pongo is available.</span>" +
    '<div class="update-actions">' +
    '<button class="update-now" type="button">Update</button>' +
    '<button class="update-later" type="button">Later</button>' +
    "</div>";
  document.body.appendChild(banner);
  banner.querySelector(".update-now").addEventListener("click", () => location.reload());
  banner.querySelector(".update-later").addEventListener("click", () => banner.remove());
}
