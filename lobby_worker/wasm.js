// Loads the client_wasm module and initialises it, then re-exports its bindings.
// Importers get a ready-to-use WasmClient/GameFsm/FsmState (top-level await blocks
// until init() resolves). A load failure marks the Play button and rethrows.
const CACHE_BUST = new URLSearchParams(window.location.search).get("v") || Date.now();
const wasmUrl = "/client_wasm/client_wasm.js?v=" + CACHE_BUST;

let mod;
try {
  mod = await import(wasmUrl);
  await mod.default(); // init()
} catch (error) {
  console.error("Failed to load WASM:", error);
  const playBtn = document.getElementById("playBtn");
  if (playBtn) {
    playBtn.textContent = "Failed to load";
    playBtn.disabled = true;
  }
  throw error;
}

export const WasmClient = mod.WasmClient;
export const GameFsm = mod.GameFsm;
export const FsmState = mod.FsmState;
