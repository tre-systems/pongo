#!/usr/bin/env node
// Stamp a unique cache version into the built service worker. Every deploy then
// ships a distinct sw.js, and that byte change is what triggers the browser's
// update flow (the "A new version is available" prompt in lobby_worker/pwa.js).
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { execSync } from "node:child_process";

const swPath = "worker/pkg/sw.js";
if (!existsSync(swPath)) {
  console.error(`stamp-sw: ${swPath} not found — run the build first.`);
  process.exit(1);
}

let version;
try {
  version = execSync("git rev-parse --short HEAD", {
    stdio: ["ignore", "pipe", "ignore"],
  })
    .toString()
    .trim();
} catch {
  version = String(Date.now());
}

const stamped = readFileSync(swPath, "utf8").replaceAll("__CACHE_VERSION__", version);
writeFileSync(swPath, stamped);
console.log(`stamped sw.js cache version: ${version}`);
