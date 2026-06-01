import init, { fetch as wasmFetch, MatchDO as WasmMatchDO } from "./pkg/lobby_worker.js";
import wasmUrl from "./pkg/lobby_worker_bg.wasm";

let initPromise;
async function ensureInit() {
  if (!initPromise) initPromise = init(wasmUrl);
  await initPromise;
}

export default {
  async fetch(req, env, ctx) {
    try {
      await ensureInit();
      return await wasmFetch(req, env, ctx);
    } catch (error) {
      return new Response(`Error: ${error.message}\n${error.stack}`, {
        status: 500,
        headers: { "Content-Type": "text/plain" },
      });
    }
  },
};

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

export { MatchDOWrapper as MatchDO };
