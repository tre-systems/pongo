# Diagrams

Graphviz / DOT sources plus rendered PNGs. The `.dot` files are the source of truth; the PNGs are committed so they render in-browser on GitHub. Mermaid is used only for small inline diagrams in Markdown — anything with enough nodes to get crowded lives here as DOT.

## Files

| Diagram                       | Source                | Rendered              |
| ----------------------------- | --------------------- | --------------------- |
| System overview (shared core) | `system-overview.dot` | `system-overview.png` |
| Client state machine          | `client-fsm.dot`      | `client-fsm.png`      |
| Netcode: authoritative server | `netcode-loop.dot`    | `netcode-loop.png`    |

## Conventions

Color coding by domain:

- **Blue** — client / browser code (input, interpolation, render).
- **Green** — server code (Cloudflare Worker, Durable Object); a bold-green node is an active "playing" state.
- **Purple** — shared, pure Rust crates (`game_core`, `proto`), no I/O.
- **Yellow / orange** — time-driven (the 60Hz loop) or recovering (paused, reconnecting).
- **Red** — error / disconnected outcomes.
- **Gray** — neutral or terminal (idle, game over).
- **Dashed edges** — "the same code runs here", or secondary / optional flows.

Fonts: Avenir. Rendered at 220 DPI.

## Render

```
npm run diagrams          # render all .dot files to PNG next to the source
npm run check:diagrams    # verify each .dot renders cleanly and the PNG exists
```

Both assume Graphviz is on PATH (`brew install graphviz`). `npm run check:diagrams` runs as part of `npm run test:all` and in CI; on a machine without `dot` it skips with a clear message, so refresh the PNGs before committing diagram changes.

To render one manually:

```
dot -Tpng:cairo docs/diagrams/<name>.dot -Gdpi=220 -o docs/diagrams/<name>.png
```
