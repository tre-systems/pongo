# Backlog

Forward-looking work and known limitations. Items here are intentional gaps, not surprises — read this before "fixing" something that is already a deliberate trade-off.

## Known limitations

- **Reconnect slot is claimed by code, not identity.** While a match is paused for a reconnect (`server_do`), the next `Join` for that match re-attaches to the held-open slot. In a private two-player room that's the dropped player returning; in principle a third party who has the link could take the slot during the grace window. Acceptable for casual play — tighten with a per-player reconnect token if it ever matters.
- **Match codes are guessable in principle.** Codes are 5 chars over a 36-char alphabet (~60M combinations) generated with `thread_rng`. Fine for short-lived two-capacity rooms; add rate limiting or longer codes if room-crashing is ever observed.

## Nice to have

- **Reconnect resume polish.** On resume, the reconnecting client briefly shows a reset board (0:0) during the ready-countdown until the first state snapshot arrives. Could buffer the last snapshot to avoid the blip.
- **Dependency freshness.** A few crates trail latest (`rand` 0.8, `getrandom` 0.2, `glam` 0.27). None are security-relevant; bump opportunistically.
- **Spectators / >2 players.** The Durable Object is hard-coded to two players. A spectator (read-only) connection would be a natural extension.
