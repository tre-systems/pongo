# Backlog

Forward-looking work and known limitations. Items here are intentional gaps, not surprises — read this before "fixing" something that is already a deliberate trade-off.

## Known limitations

- **Reconnect slot is claimed by code, not identity.** While a match is paused for a reconnect (`server_do`), the next `Join` for that match re-attaches to the held-open slot. In a private two-player room that's the dropped player returning; in principle a third party who has the link could take the slot during the grace window. Acceptable for casual play — tighten with a per-player reconnect token if it ever matters.
- **Gameplay e2e skips without WebGPU.** The Playwright gameplay specs (start-game, matchmaking) need WebGPU and `test.skip()` where it's unavailable, which includes some headless CI runners. The menu/WASM/match-code specs always run. Revisit with a WebGPU-capable runner or a reliable software-WebGPU setup to exercise gameplay in CI too.
- **Match codes are guessable in principle.** Codes are 5 chars over a 36-char alphabet (~60M combinations) generated with `thread_rng`. Fine for short-lived two-capacity rooms; add rate limiting or longer codes if room-crashing is ever observed.

## Patterns and refactors

See [ARCHITECTURE.md](ARCHITECTURE.md#patterns-to-adopt) for the rationale.

- **`Simulation` aggregate (parameter object).** Bundle the nine simulation fields (`world, time, map, config, score, events, net_queue, rng, respawn_state`) — currently re-declared in `GameState`, `LocalGame`, and `ClientPredictor`, and passed as nine args to `game_core::step` — into one `Simulation` type with `new(seed)` and `step(&mut self)`. Removes the duplication and the `too_many_arguments` allow.
- **Collapse the predictor's `Option` soup.** With the aggregate, `ClientPredictor`'s nine parallel `Option<T>` fields become one `Option<Simulation>`.
- **Single-source the fixed timestep.** Route `Time::default`, `LocalGame`, and the predictor through `Params::FIXED_DT` instead of `0.016` / `1.0/60.0`. Consolidate the two paddle-Y clamp helpers (`GameMap::clamp_y`, `Config::clamp_paddle_y`).
- **`PlayerId` newtype.** Replace bare `player_id: u8` with a `PlayerId(u8)` newtype to prevent mixing it with scores/ticks. Low priority.

## Nice to have

- **Reconnect resume polish.** On resume, the reconnecting client briefly shows a reset board (0:0) during the ready-countdown until the first state snapshot arrives. Could buffer the last snapshot to avoid the blip.
- **Dependency freshness.** A few crates trail latest (`rand` 0.8, `getrandom` 0.2, `glam` 0.27). None are security-relevant; bump opportunistically.
- **Spectators / >2 players.** The Durable Object is hard-coded to two players. A spectator (read-only) connection would be a natural extension.
