# Pongo

A multiplayer Pong game built with **Rust + WebAssembly** (client) and **Cloudflare Durable Objects** (server).

**[Play now →](https://pongo.tre.systems)**

![Pongo gameplay](screenshot.png)

<div align="center">
  <a href='https://ko-fi.com/N4N31DPNUS' target='_blank'><img height='36' style='border:0px;height:36px;' src='https://storage.ko-fi.com/cdn/kofi2.png?v=6' border='0' alt='Buy Me a Coffee at ko-fi.com' /></a>
</div>

## Quick Start

```bash
cargo install wasm-pack       # Prerequisites: Rust, Node 20+
npx wrangler login            # One-time Cloudflare auth
npm run build && npm run dev  # http://localhost:8787
```

## How to Play

| Mode            | How                                     |
| --------------- | --------------------------------------- |
| **Multiplayer** | Click **CHALLENGE** → share link → JOIN |
| **VS AI**       | Click **PLAY**                          |

**Controls:** Arrow keys or W/S · Touch on mobile  
**Rules:** First to 5. Hit position affects ball trajectory.

## Architecture

See **[ARCHITECTURE.md](ARCHITECTURE.md)** for the system diagram and component deep dive.

**Key design decisions:**

- **Shared `game_core`** — The same deterministic physics runs the offline game and the authoritative server
- **Binary protocol** — Minimal `postcard` serialization over WebSocket
- **Durable Objects** — Each match is a stateful instance with 60Hz game loop

## Project Structure

```
pongo/
├── game_core/       # Game logic (plain structs) — shared by client/server
├── proto/           # Network protocol (postcard)
├── client_wasm/     # Canvas2D renderer + client
├── server_do/       # Durable Object server
├── lobby_worker/    # HTTP endpoints + routing
└── worker/          # Built WASM + assets
```

## Commands

```bash
npm run build        # Build WASM
npm run dev          # Local server
npm run test         # Run tests
npm run test:all     # Full pre-push gate (prettier, fmt, check, clippy, tests, diagrams)
npm run deploy       # Deploy to Cloudflare
```

## Troubleshooting

| Issue       | Fix                                  |
| ----------- | ------------------------------------ |
| Build fails | `cargo install wasm-pack`            |
| Port in use | Kill process or edit `wrangler.toml` |
| Reset state | Delete `.wrangler/state/`            |

See **[ARCHITECTURE.md](ARCHITECTURE.md)** for technical details and game constants.

## Contributing

- **[AGENTS.md](AGENTS.md)** — workflow, verification gate, architecture rules, and code map (start here).
- **[docs/STATE_MACHINE.md](docs/STATE_MACHINE.md)** — the client finite state machine.
- **[docs/BACKLOG.md](docs/BACKLOG.md)** — known limitations and forward-looking work.

## License

MIT
