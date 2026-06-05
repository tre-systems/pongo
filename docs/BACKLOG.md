# Backlog

Forward-looking work and known limitations. Items here are intentional gaps, not surprises — read this before "fixing" something that is already a deliberate trade-off.

## Known limitations

- **Reconnect slot is claimed by code, not identity.** While a match is paused for a reconnect (`server_do`), the next `Join` for that match re-attaches to the held-open slot. In a private two-player room that's the dropped player returning; in principle a third party who has the link could take the slot during the grace window. Acceptable for casual play — tighten with a per-player reconnect token if it ever matters.
- **Match codes are guessable in principle.** Codes are 5 chars over a 36-char alphabet (~60M combinations) generated with `thread_rng`. Fine for short-lived two-capacity rooms; add rate limiting or longer codes if room-crashing is ever observed.

## Nice to have

- **Reconnect resume polish.** On resume, the reconnecting client briefly shows a reset board (0:0) during the ready-countdown until the first state snapshot arrives. Could buffer the last snapshot to avoid the blip.
- **Dependency freshness.** A few crates trail latest (`rand` 0.8, `getrandom` 0.2, `glam` 0.27). None are security-relevant; bump opportunistically.
- **Spectators / >2 players.** The Durable Object is hard-coded to two players. A spectator (read-only) connection would be a natural extension.
- **Region-aware DO placement.** Each match runs in whatever location Cloudflare first instantiates the DO (near whoever created it), so a player far from that region sees light-speed latency. Pass a `locationHint`/jurisdiction so both players land near a shared region.

## Code health

Behaviour-neutral cleanups deferred to keep changes small and safe to verify.

- **Collapse the CSS duplication.** `lobby_worker/style.css` declares many layout selectors twice — a desktop value, then an unguarded "mobile-first" block that always wins — so the desktop declarations are effectively dead. Rework into one mobile-first base plus a single `@media (min-width: …)` block; verify on mobile and desktop widths since it touches the live layout.
- **Tune the wasm release profile.** Add `[profile.release]` (e.g. `opt-level = "z"`, `lto = true`) to shrink and speed up the deployed wasm; confirm the shared simulation stays bit-identical so client/server determinism holds.
- **Cover the time-based server paths.** Give the test `MockEnv` an advanceable clock so the reconnect-grace expiry and idle-timeout triggers in the alarm loop are tested directly (today only their consequences are), and assert the broadcast bytes for the match-lifecycle messages.
- **Name `LocalGame::step`'s return.** Replace its six-element tuple (`client_wasm/src/simulation.rs`) with a small struct so call sites read by field, not position.
- **Bump the Workers `compatibility_date`** (pinned at `2024-01-01`) periodically, as a deliberate, smoke-tested change rather than a drive-by.
- **Dedupe paddle-input sends.** The client polls `sendInput()` on a ~30 Hz timer (`lobby_worker/script.js`) and `get_input_bytes()` (`client_wasm/src/lib.rs`) always re-serialises the current paddle Y with an incrementing `seq`, so a stationary paddle still streams ~30 `C2S::Input` messages/sec — each a no-op, since the server treats input as an absolute, idempotent Y and ignores `seq` (`server_do/src/lib.rs`). Track the last-sent Y and return an empty buffer when it is unchanged; the JS send path already drops empty buffers (`if (bytes.length > 0)`), and the 30 Hz timer stays as the send-rate cap. Gameplay is unaffected — it just trims redundant Durable-Object message wakeups. Safe against the 120 s idle reaper because the 2 s `Ping` independently refreshes `last_activity`.
- **Decouple the server alarm rate from the sim rate (hosting-cost lever).** The alarm reschedules every 16 ms (~60 Hz) during `Playing` (`server_do/src/lib.rs`), and each alarm invocation is a billed Durable-Object request — the dominant per-match cost. The alarm already runs an accumulator + catch-up loop that can step the sim several times per wake, so scheduling it at the 20 Hz broadcast cadence (~50 ms) and letting that loop run ~3 `FIXED_DT` steps keeps physics bit-identical while cutting alarm requests ~3×. Trade-off: input reaches the authoritative sim up to ~50 ms later — largely hidden because the client owns its own paddle — so verify game feel. This is the highest-leverage knob on the Durable-Object bill (far more than the input-dedup item above).
