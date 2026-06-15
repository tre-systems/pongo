import * as Sentry from "@sentry/cloudflare";
import init, { fetch as wasmFetch, MatchDO as WasmMatchDO } from "./pkg/lobby_worker.js";
import wasmUrl from "./pkg/lobby_worker_bg.wasm";

let initPromise;
async function ensureInit() {
  if (!initPromise) initPromise = init(wasmUrl);
  await initPromise;
}

const sentryOptions = (env) => {
  if (!env.SENTRY_DSN) return undefined;
  return {
    dsn: env.SENTRY_DSN,
    environment: env.SENTRY_ENVIRONMENT || "production",
    release: env.SENTRY_RELEASE,
    sendDefaultPii: false,
    tracesSampleRate: env.SENTRY_ENVIRONMENT === "production" ? 0.01 : 0,
    enableRpcTracePropagation: true,
    beforeSend(event) {
      if (event.request) {
        delete event.request.cookies;
        delete event.request.data;
        if (event.request.headers) {
          for (const key of Object.keys(event.request.headers)) {
            const lowerKey = key.toLowerCase();
            if (lowerKey.includes("authorization") || lowerKey.includes("cookie")) {
              event.request.headers[key] = "[Filtered]";
            }
          }
        }
      }
      return event;
    },
  };
};

const handler = {
  async fetch(req, env, ctx) {
    try {
      await ensureInit();
      return await wasmFetch(req, env, ctx);
    } catch (error) {
      // Log details server-side; don't leak internals (message/stack) to clients.
      console.error("Worker fetch error:", error);
      if (env.SENTRY_DSN) {
        Sentry.captureException(error);
        ctx.waitUntil(Sentry.flush(2000));
      }
      return new Response("Internal error", {
        status: 500,
        headers: { "Content-Type": "text/plain" },
      });
    }
  },
};

export default Sentry.withSentry(sentryOptions, handler);

// The generated WasmMatchDO constructor is synchronous and needs the wasm
// module ready. The DO runtime can construct it before the top-level init()
// resolves, so defer construction behind ensureInit().
class MatchDOWrapper {
  constructor(state, env) {
    this._inner = ensureInit().then(() => new WasmMatchDO(state, env));
  }
  async fetch(req) {
    return (await this._inner).fetch(req);
  }
  async alarm() {
    return (await this._inner).alarm();
  }
  async webSocketMessage(ws, msg) {
    return (await this._inner).webSocketMessage(ws, msg);
  }
  async webSocketClose(ws, code, reason, wasClean) {
    return (await this._inner).webSocketClose(ws, code, reason, wasClean);
  }
  async webSocketError(ws, error) {
    return (await this._inner).webSocketError(ws, error);
  }
}

export const MatchDO = Sentry.instrumentDurableObjectWithSentry(
  (env) =>
    sentryOptions(env) || {
      environment: env.SENTRY_ENVIRONMENT || "production",
      sendDefaultPii: false,
      tracesSampleRate: 0,
      enableRpcTracePropagation: true,
    },
  MatchDOWrapper
);
