import { writeFileSync } from "node:fs";
import { execSync } from "node:child_process";

function fallbackRelease() {
  try {
    return execSync("git rev-parse --short HEAD", { encoding: "utf8" }).trim();
  } catch {
    return String(Date.now());
  }
}

writeFileSync(
  "lobby_worker/sentry-config.js",
  `window.TRE_STATIC_SENTRY_CONFIG = ${JSON.stringify(
    {
      app: "pongo",
      dsn: process.env.SENTRY_DSN || "",
      environment: process.env.SENTRY_ENVIRONMENT || "production",
      release: process.env.SENTRY_RELEASE || process.env.GITHUB_SHA || fallbackRelease(),
    },
    null,
    2
  )};\n`
);
