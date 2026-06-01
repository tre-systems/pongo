# Backlog

Forward-looking work and known limitations. Items here are intentional gaps or deferred fixes, not surprises — read this before "fixing" something that is already tracked.

## Correctness

- **WebSocket → player identity.** The Durable Object cannot map a `WebSocket` back to its player, so it works around it (`server_do/src/lib.rs`): `websocket_close` removes the _first_ player in the map rather than the one that actually closed, and `Ping` refreshes the idle timer for _all_ clients. In a strict 2-player room this is mostly benign, but a disconnect can evict the wrong player, and one active player can keep an idle opponent alive past the 2-minute timeout. Fix: attach a player id to each socket (serialized attachment / tags) and key cleanup and activity off that.
- **Dead paddle "english".** `PaddleIntent.dir` (`game_core/src/components.rs`) is always `0` and is never reassigned, so the `paddle_influence` term in `resolve_paddle_collision` (`game_core/src/systems/collision.rs`) is always `0` — the "slice the ball with paddle motion" mechanic never activates. Ball deflection still works via hit position (`y_deflection`); only the paddle-velocity contribution is dead. Either wire `dir` into the target-Y input model or remove the dead term and its comments.

## Testing

- **No JS / e2e / browser tests.** `lobby_worker/script.js` (~1k lines of FSM-driving glue — the riskiest, least-covered code) has no automated coverage. Add a Playwright (or similar) smoke that drives a local game and a two-tab multiplayer match through the FSM.
- **wasm-only tests don't run in CI.** The `#[wasm_bindgen_test]`s in `client_wasm/src/prediction.rs` only run under `wasm-pack test`; CI runs `cargo test` only. Add a `wasm-pack test --headless` step.

## CI / tooling

- **CI doesn't check non-Rust formatting.** `cargo fmt --check` runs in CI, but `prettier --check` does not, so JS/CSS/HTML formatting can drift on a direct push to `main`. Add `prettier --check .` to the `check` job (it is already part of `npm run test:all`).

## Gameplay / UX states

- **Pause state.** No way to pause a local game.
- **Reconnecting state.** A dropped multiplayer connection goes straight to `Disconnected`. A `Reconnecting` state with a short grace period would survive transient packet loss without ending the match.

## Cleanup

- **`script.js` rough edges.** `FSM.canTransition()` is a no-op that always returns `true` (callers gate on it pointlessly; the Rust FSM is the real guard). Several `catch {}` blocks in the render/input loops swallow errors silently, which makes browser-side debugging harder.
- **Duplicate build definition.** `wrangler.toml`'s `[build]` command only builds the server half and copies `index.html`; the complete and authoritative build path is the `npm run build` scripts (used by CI and `npm run dev`). Reconcile or drop the `wrangler.toml` build block to avoid confusion about which one runs.
