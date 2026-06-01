# Agent Notes

Operational guidance for Claude Code, Codex, and other repo agents working on Pongo.

## Project

Pongo is a real-time multiplayer Pong game. The simulation is written in Rust and compiled to WebAssembly, and the _same_ `game_core` crate runs on both the authoritative server (Cloudflare Durable Object) and the offline VS-AI game in the browser. The multiplayer client moves its own paddle locally and interpolates the opponent + ball from server snapshots; it renders with Canvas2D. The server runs one Durable Object per match with a 60Hz tick loop and broadcasts state at 20Hz over a binary WebSocket protocol. It deploys to Cloudflare Workers and is live at <https://pongo.tre.systems>.

Read before substantial work:

- `ARCHITECTURE.md` — system overview, codebase map, data flows, game constants, and the wire protocol.
- `docs/STATE_MACHINE.md` — the client FSM (Rust logic + JS side effects) and its transition lock.
- `docs/BACKLOG.md` — known limitations and forward-looking work. Read it before "fixing" something that is already a tracked, intentional gap.

## Workflow

- Work directly on `main`. Commit and push to `main` — no feature branches, worktrees, or PRs unless explicitly asked.
- Check `git status` before editing. Stage only the files owned by the current task; avoid `git add -A`.
- A push to `main` triggers CI, which **auto-deploys to Cloudflare**. Treat every push to `main` as a production deploy.
- After a code change: confirm CI is green, then smoke-test <https://pongo.tre.systems> in a browser. Docs-only changes just need commit + push.

## Verification

- Standard gate: `npm run test:all` — runs `prettier --check`, `cargo fmt --check`, `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, and `npm run check:diagrams`. This should pass before every push.
- Diagrams: edit the `.dot` sources in `docs/diagrams/`, then `npm run diagrams` to re-render the PNGs (needs Graphviz — `brew install graphviz`). `npm run check:diagrams` verifies they still render. Use Graphviz for anything non-trivial; reserve Mermaid for small inline diagrams. See [`docs/diagrams/README.md`](docs/diagrams/README.md).
- The husky `pre-commit` hook runs `lint-staged` + `cargo fmt --check` + `cargo check` + `cargo clippy -D warnings` + `cargo test --workspace`. Do not bypass with `--no-verify` unless the user explicitly asks.
- CI (`.github/workflows/ci.yml`): the `check` job runs `cargo fmt --check`, `prettier --check`, clippy, `cargo test --workspace`, and a `wasm32-unknown-unknown` build check, and gates the deploy on push to `main`. A separate non-gating `e2e` job runs the Playwright smoke, so a headless-browser hiccup never blocks a deploy.

## Build & Run

- `npm run build` — wasm-pack builds `lobby_worker` (server) and `client_wasm` (client) into `worker/pkg/`, and copies the static assets (`index.html`, `style.css`, `script.js`).
- `npm run dev` — `wrangler dev` on <http://localhost:8787>.
- `npm run deploy` — `wrangler deploy` (CI does this automatically on push to `main`).
- `npm run logs` — `wrangler tail` for live Worker logs.
- Prerequisites: Rust + `wasm-pack`, Node 20+, and a one-time `npx wrangler login` for Cloudflare auth.

## Architecture Rules

- `game_core` is the single source of truth for the simulation and must stay deterministic and platform-agnostic — no browser or Worker APIs. The same `step` runs on the server and in the browser's offline game.
- The server (`server_do`) is authoritative. Never trust the client for anything that affects another player: paddle input is an absolute Y position, clamped to the arena and speed-validated server-side (`game_core/src/systems/input.rs`, `movement.rs`).
- The wire protocol (`proto`) is binary `postcard`, which is not self-describing. A `C2S`/`S2C` change is a breaking change for clients already connected across a deploy — version it or accept mid-match desync for in-flight sessions.
- Each match is one Durable Object instance. DO handlers must not panic, and must not hold a `RefCell` borrow across an `.await` (see the alarm handler in `server_do/src/lib.rs`).
- The client FSM logic lives in Rust (`client_wasm/src/fsm.rs`); its side effects live in JS (`lobby_worker/script.js`). See `docs/STATE_MACHINE.md`.

## Code Map

| Path            | What                                                                                                                                                                 |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `game_core/`    | Shared struct-based simulation: entities (ball, paddles), systems (input → movement → collision → scoring), config, determinism. The heart.                          |
| `proto/`        | `postcard` wire protocol — `C2S`, `S2C`, `GameStateSnapshot`.                                                                                                        |
| `client_wasm/`  | Browser client: Canvas2D renderer (`canvas2d.rs`), snapshot interpolation (`state.rs`), FSM (`fsm.rs`), local AI game (`simulation.rs`).                             |
| `server_do/`    | Cloudflare Durable Object — authoritative match server (`MatchDO`, `game_state.rs`).                                                                                 |
| `lobby_worker/` | HTTP router (`/create`, `/join/:code`, `/ws/:code`), match-code generation, and the static front-end (`index.html`, `style.css`, `script.js`). Re-exports `MatchDO`. |
| `worker/`       | wasm-pack build output (`pkg/`, gitignored) plus `index.js`, the Worker entry shim that initialises the WASM and wires the Durable Object lifecycle hooks.           |

## Tests

- Rust tests are inline `#[cfg(test)]` modules co-located with the code. Run `cargo test --workspace`. `game_core` physics, the FSM transition table, and the server match lifecycle including reconnect (`server_do/src/tests.rs`) are well covered.
- End-to-end browser tests live in `tests/e2e/` (Playwright): `npm run test:e2e` (first run: `npm run test:e2e:install`). It boots `wrangler dev` automatically. All specs — menu, match-code, local gameplay/pause, and two-player matchmaking — run in CI (Canvas2D needs no GPU).

## Commits

- Match the existing history: short, outcome-focused, conventional-style (`feat:`, `fix:`, `chore:`, or a plain imperative summary). Reference `file.rs:line` where it helps a reader.
- No AI-attribution lines in commit messages or in code comments.
- On a pre-commit hook failure, fix the issue and make a NEW commit — do not blindly `--amend`, since the failed commit did not happen.

## Docs

- Docs describe current behaviour in the present tense. Keep history out — no changelog, "now implemented", or "used to be X" prose. Git history is the record.
- Forward-looking work goes in `docs/BACKLOG.md`, not as "TODO" narration inside reference docs.
- If a doc describes intended-but-unbuilt behaviour, say so explicitly or move it to the backlog.
