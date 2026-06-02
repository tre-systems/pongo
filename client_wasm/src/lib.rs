//! Canvas2D client for Pong game

// Platform-agnostic modules (pure Rust + proto): compiled everywhere and unit-
// tested natively. Only the wasm client consumes them, so dead_code is allowed
// off-wasm.
mod fsm;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod state;

// Re-export FSM types (always available)
pub use fsm::{FsmState, GameAction, GameFsm};

// Everything below requires wasm32
#[cfg(target_arch = "wasm32")]
mod canvas2d;
#[cfg(target_arch = "wasm32")]
mod input;
#[cfg(target_arch = "wasm32")]
mod network;
#[cfg(target_arch = "wasm32")]
mod simulation;

#[cfg(target_arch = "wasm32")]
use canvas2d::Renderer;
#[cfg(target_arch = "wasm32")]
use simulation::LocalGame;
#[cfg(target_arch = "wasm32")]
use state::GameState;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, HtmlCanvasElement, KeyboardEvent};

/// Main client state
#[cfg(target_arch = "wasm32")]
pub struct Client {
    renderer: Renderer,
    game_state: GameState,
    paddle_dir: i8, // -1 = up, 0 = stop, 1 = down
    last_frame_time: f64,
    last_sim_time: f64,
    sim_accumulator: f32,
    fps: f32,
    fps_frame_count: u32,
    fps_last_update: f64,
    ping_ms: f32,
    ping_pending: Option<f64>,
    local_game: Option<LocalGame>,
    // Own paddle: client-authoritative, integrated locally for a zero-latency feel
    local_paddle_y: f32,
    local_paddle_initialized: bool,
    input_seq: u32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmClient(Client);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmClient {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<WasmClient, JsValue> {
        console_error_panic_hook::set_once();

        let renderer = Renderer::new(canvas).map_err(|e| JsValue::from_str(&e))?;

        Ok(WasmClient(Client {
            renderer,
            game_state: GameState::new(),
            paddle_dir: 0,
            last_frame_time: 0.0,
            last_sim_time: 0.0,
            sim_accumulator: 0.0,
            fps: 0.0,
            fps_frame_count: 0,
            fps_last_update: 0.0,
            ping_ms: 0.0,
            ping_pending: None,
            local_game: None,
            local_paddle_y: 12.0,
            local_paddle_initialized: false,
            input_seq: 0,
        }))
    }

    fn performance_now() -> f64 {
        window()
            .and_then(|w| {
                js_sys::Reflect::get(&w, &JsValue::from_str("performance"))
                    .ok()
                    .and_then(|perf| {
                        js_sys::Reflect::get(&perf, &JsValue::from_str("now"))
                            .ok()
                            .and_then(|now_fn| {
                                let now_func: js_sys::Function = now_fn.dyn_into().ok()?;
                                now_func.call0(&perf).ok()?.as_f64()
                            })
                    })
            })
            .unwrap_or_else(|| js_sys::Date::now())
    }

    fn step_simulation(client: &mut Client) {
        if let Some(local_game) = &mut client.local_game {
            const SIM_FIXED_DT: f32 = game_core::Params::FIXED_DT;
            let now_ms = Self::performance_now();

            if client.last_sim_time == 0.0 {
                client.last_sim_time = now_ms;
                return;
            }

            let frame_time_ms = (now_ms - client.last_sim_time) / 1000.0;
            client.sim_accumulator += frame_time_ms as f32;
            client.last_sim_time = now_ms;

            while client.sim_accumulator >= SIM_FIXED_DT {
                client.sim_accumulator -= SIM_FIXED_DT;

                let (winner, ball_data, left_y, right_y, score_left, score_right) =
                    local_game.step(client.local_paddle_y);

                if let Some(w) = winner {
                    client.game_state.set_winner(w);
                }

                if let Some((ball_pos, ball_vel)) = ball_data {
                    client.game_state.set_current(proto::GameStateSnapshot {
                        ball_x: ball_pos.x,
                        ball_y: ball_pos.y,
                        paddle_left_y: left_y,
                        paddle_right_y: right_y,
                        ball_vx: ball_vel.x,
                        ball_vy: ball_vel.y,
                        tick: 0,
                        score_left,
                        score_right,
                    });
                }
                client.game_state.set_scores(score_left, score_right);
            }
        }
    }

    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        let client = &mut self.0;

        Self::step_simulation(client);

        let now_ms = Self::performance_now();
        let render_dt = if client.last_frame_time > 0.0 {
            ((now_ms - client.last_frame_time) / 1000.0) as f32
        } else {
            0.008
        };
        client.last_frame_time = now_ms;

        // Integrate the local paddle each frame; this drives our own paddle in
        // multiplayer and feeds the local-game input queue. Speed and bounds come
        // from game_core::Params so the client matches the authoritative server.
        const PADDLE_SPEED: f32 = game_core::Params::PADDLE_SPEED;
        const ARENA_HEIGHT: f32 = game_core::Params::ARENA_HEIGHT;
        const PADDLE_HEIGHT: f32 = game_core::Params::PADDLE_HEIGHT;
        let half_height = PADDLE_HEIGHT / 2.0;

        client.local_paddle_y += client.paddle_dir as f32 * PADDLE_SPEED * render_dt;
        client.local_paddle_y = client
            .local_paddle_y
            .clamp(half_height, ARENA_HEIGHT - half_height);

        // FPS calculation
        client.fps_frame_count += 1;
        let now_sec = now_ms / 1000.0;
        if now_sec - (client.fps_last_update / 1000.0) >= 1.0 {
            let time_diff_sec = (now_ms - client.fps_last_update) / 1000.0;
            client.fps = client.fps_frame_count as f32 / time_diff_sec as f32;
            client.fps_frame_count = 0;
            client.fps_last_update = now_ms;
        }

        client.game_state.update_interpolation(render_dt);

        client
            .renderer
            .draw(
                &client.game_state,
                client.local_paddle_y,
                client.local_game.is_some(),
            )
            .map_err(|e| JsValue::from_str(&e))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn on_message(&mut self, bytes: Vec<u8>) -> Result<(), JsValue> {
        let client = &mut self.0;
        let msg = proto::S2C::from_bytes(&bytes)
            .map_err(|e| format!("Failed to deserialize: {:?}", e))?;

        if let proto::S2C::Pong { .. } = msg {
            if let Some(sent_time) = client.ping_pending {
                let rtt = Self::performance_now() - sent_time;
                client.ping_ms = rtt as f32;
                client.ping_pending = None;
            }
            return Ok(());
        }

        // Check for state changes that require local reset
        match msg {
            proto::S2C::MatchFound | proto::S2C::Countdown { .. } => {
                client.local_paddle_y = 12.0;
                client.local_paddle_initialized = false;
                // Reset timing to prevent massive dt on first frame
                client.last_frame_time = 0.0;
                client.last_sim_time = 0.0;
                client.sim_accumulator = 0.0;
            }
            _ => {}
        }

        let is_game_state = matches!(msg, proto::S2C::GameState(_));

        network::handle_message(msg, &mut client.game_state)
            .map_err(|e| JsValue::from_str(&format!("Msg error: {}", e)))?;

        // On the first snapshot of a match, snap our own paddle to the server's position.
        if is_game_state && client.local_game.is_none() && !client.local_paddle_initialized {
            let snapshot = client.game_state.get_current_snapshot();
            let pid = client.game_state.get_player_id().unwrap_or(0);
            client.local_paddle_y = if pid == 0 {
                snapshot.paddle_left_y
            } else {
                snapshot.paddle_right_y
            };
            client.local_paddle_initialized = true;
        }

        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_join_bytes(&self, code: String) -> Vec<u8> {
        network::create_join_message(&code).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn get_restart_bytes(&self) -> Vec<u8> {
        network::create_restart_message().unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn reset_local_state(&mut self) {
        let client = &mut self.0;
        client.game_state.reset();
    }

    #[wasm_bindgen]
    pub fn get_input_bytes(&mut self) -> Vec<u8> {
        let client = &mut self.0;
        if client.local_game.is_some() {
            let pid = client.game_state.get_player_id().unwrap_or(0);
            return network::create_input_message(pid, client.local_paddle_y, 0)
                .unwrap_or_default();
        }

        // Client is authoritative over its own paddle: send the current Y.
        let pid = client.game_state.get_player_id().unwrap_or(0);
        let seq = client.input_seq;
        client.input_seq = seq.wrapping_add(1);

        network::create_input_message(pid, client.local_paddle_y, seq).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn get_score(&self) -> Vec<u8> {
        if let Some(local) = &self.0.local_game {
            vec![local.sim.score.left, local.sim.score.right]
        } else {
            let (l, r) = self.0.game_state.get_scores();
            vec![l, r]
        }
    }

    #[wasm_bindgen]
    pub fn get_winner(&self) -> Option<String> {
        if let Some(w) = self.0.game_state.winner {
            if Some(w) == self.0.game_state.my_player_id {
                Some("you".to_string())
            } else {
                Some("opponent".to_string())
            }
        } else {
            None
        }
    }

    /// Get and clear the latest match event from server
    /// Returns: "match_found", "countdown:3", "countdown:2", "countdown:1", "game_start",
    /// "opponent_disconnected", "opponent_reconnecting", "opponent_reconnected", or empty string
    #[wasm_bindgen]
    pub fn get_match_event(&mut self) -> String {
        use state::MatchEvent;
        let event = std::mem::replace(&mut self.0.game_state.match_event, MatchEvent::None);
        match event {
            MatchEvent::None => String::new(),
            MatchEvent::MatchFound => "match_found".to_string(),
            MatchEvent::Countdown(n) => format!("countdown:{}", n),
            MatchEvent::GameStart => "game_start".to_string(),
            MatchEvent::OpponentDisconnected => "opponent_disconnected".to_string(),
            MatchEvent::OpponentReconnecting => "opponent_reconnecting".to_string(),
            MatchEvent::OpponentReconnected => "opponent_reconnected".to_string(),
        }
    }

    #[wasm_bindgen]
    pub fn start_local_game(&mut self) {
        let seed = Self::performance_now() as u64;
        self.0.local_game = Some(LocalGame::new(seed));
        self.0.game_state.reset();
        self.0.game_state.set_player_id(0);
        // Reset simulation timing
        self.0.last_sim_time = 0.0;
        self.0.sim_accumulator = 0.0;
    }

    #[wasm_bindgen]
    pub fn stop_local_game(&mut self) {
        self.0.local_game = None;
        self.0.game_state.reset();
        self.0.paddle_dir = 0;
        self.0.local_paddle_y = 12.0;
    }

    /// Reset only the simulation clock (not the game). Call when resuming from a
    /// pause so the first frame doesn't accumulate a large dt and fast-forward.
    #[wasm_bindgen]
    pub fn reset_sim_timing(&mut self) {
        self.0.last_sim_time = 0.0;
        self.0.last_frame_time = 0.0;
        self.0.sim_accumulator = 0.0;
    }

    /// Reset client state for a new multiplayer session
    #[wasm_bindgen]
    pub fn reset_for_multiplayer(&mut self) {
        // Clear any local game
        self.0.local_game = None;
        // Reset game state
        self.0.game_state.reset();
        // Reset paddle state
        self.0.local_paddle_y = 12.0;
        self.0.local_paddle_initialized = false;
        // Reset timing
        self.0.last_frame_time = 0.0;
        self.0.last_sim_time = 0.0;
        self.0.sim_accumulator = 0.0;
    }

    #[wasm_bindgen]
    pub fn get_metrics(&self) -> Vec<f32> {
        if self.0.local_game.is_some() {
            vec![self.0.fps, 0.0]
        } else {
            vec![self.0.fps, self.0.ping_ms]
        }
    }

    #[wasm_bindgen]
    pub fn send_ping(&mut self) -> Vec<u8> {
        let now = Self::performance_now();
        self.0.ping_pending = Some(now);
        network::create_ping_message(now as u32).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn on_key_down(&mut self, event: KeyboardEvent) {
        self.0.paddle_dir = input::handle_key_down(&event.key(), self.0.paddle_dir);
    }

    #[wasm_bindgen]
    pub fn on_key_up(&mut self, event: KeyboardEvent) {
        self.0.paddle_dir = input::handle_key_up(&event.key(), self.0.paddle_dir);
    }

    #[wasm_bindgen]
    pub fn handle_key_string(&mut self, key: String, is_down: bool) {
        if is_down {
            self.0.paddle_dir = input::handle_key_down(&key, self.0.paddle_dir);
        } else {
            self.0.paddle_dir = input::handle_key_up(&key, self.0.paddle_dir);
        }
    }
}
