# State Machine

The client game flow is driven by a finite state machine with a deliberate split:

- **Logic** — the valid states and transitions — lives in Rust, compiled to WASM: `client_wasm/src/fsm.rs`.
- **Side effects** — DOM, UI, and WebSocket I/O — live in JavaScript: the `FSM` wrapper in `lobby_worker/script.js`.

Rust answers "is this transition allowed?"; JavaScript performs "what happens on enter/exit". The Rust FSM holds only the current `FsmState` — no game data (scores, positions) and no side effects.

## States and actions

`FsmState` (`client_wasm/src/fsm.rs`): `Idle`, `CountdownLocal`, `PlayingLocal`, `GameOverLocal`, `Connecting`, `Waiting`, `CountdownMulti`, `PlayingMulti`, `GameOverMulti`, `Disconnected`.

`GameAction`: `StartLocal`, `CreateMatch`, `JoinMatch`, `CountdownDone`, `Quit`, `GameOver`, `Connected`, `ConnectionFailed`, `OpponentJoined`, `Disconnected`, `Leave`, `PlayAgain`, `RematchStarted`.

`GameFsm::get_next_state` is the single transition table; invalid `state + action` pairs are rejected. The FSM is exposed to JS via `wasm_bindgen` (`transition_str`, `state`) and is fully unit-tested in `fsm.rs`.

## Flow

```mermaid
stateDiagram-v2
    [*] --> Idle

    Idle --> CountdownLocal: StartLocal
    Idle --> Connecting: CreateMatch / JoinMatch

    state "Local Game" as Local {
        CountdownLocal --> PlayingLocal: CountdownDone
        PlayingLocal --> GameOverLocal: GameOver
        GameOverLocal --> CountdownLocal: PlayAgain
    }

    state "Multiplayer" as Multi {
        Connecting --> Waiting: Connected
        Connecting --> Idle: ConnectionFailed

        Waiting --> CountdownMulti: OpponentJoined
        CountdownMulti --> PlayingMulti: CountdownDone

        PlayingMulti --> GameOverMulti: GameOver
        GameOverMulti --> CountdownMulti: RematchStarted
    }

    Local --> Idle: Quit / Leave
    Multi --> Disconnected: Disconnected / Timeout
    Disconnected --> Idle: Leave
```

## The JS wrapper and the transition lock

`lobby_worker/script.js` wraps the Rust FSM instance. Each `transition(action)`:

1. Rejects the action if a transition is already running (an `isTransitioning` guard).
2. Calls the Rust FSM to validate and advance the state (synchronous).
3. Runs the async `exitState(prev)` then `enterState(next)` side effects.
4. Releases the lock in a `finally` block.

The `isTransitioning` lock serialises transitions so that rapid events — for example a disconnect arriving mid-countdown — cannot interleave the enter/exit handlers and desync the UI.

**Deadlock note:** because an enter-handler runs while the lock is held, it must not synchronously drive another transition through the wrapper, or it will deadlock against its own lock. Where a state needs to advance immediately (e.g. `CountdownLocal` firing `CountdownDone` once the countdown animation finishes), the handler schedules that next transition without awaiting it, so the current transition completes and releases the lock first. See `enterCountdownLocal` in `script.js`.

## Design notes

- The FSM is exported from `client_wasm`, but the Rust `Client` simulation does not read it: the render/sim loop keys off presence checks (`local_game.is_some()`, `predictor.is_active()`) rather than the FSM state. The FSM governs UI and flow in JS, not the Rust simulation.
- A `GameState` object in `script.js` mirrors `FsmState` for DOM/CSS use.

Forward-looking changes — a `Paused` state for local play, and a `Reconnecting` grace state instead of dropping straight to `Disconnected` on transient packet loss — are tracked in `docs/BACKLOG.md`.
