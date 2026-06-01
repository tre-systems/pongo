#![allow(unknown_lints)]
#![allow(clippy::manual_is_multiple_of)]
use game_core::*;
use js_sys::Date;
use proto::*;
use std::collections::HashMap;
use worker::*;

/// How long a dropped player's slot is held for reconnection before forfeiting (ms).
pub const RECONNECT_GRACE_MS: u64 = 20_000;

/// Server-side match lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchState {
    /// Waiting for players to join
    Waiting,
    /// Both players connected, counting down
    Countdown,
    /// Game in progress
    Playing,
    /// A player dropped mid-match; the sim is frozen awaiting their reconnect
    Paused,
    /// Game ended
    GameOver,
}

// Abstract connection for testing
pub trait GameClient {
    fn send_bytes(&self, bytes: &[u8]) -> Result<()>;
}

impl GameClient for WebSocket {
    fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
        self.send_with_bytes(bytes)
    }
}

// Abstract environment (Time, Logging)
pub trait Environment {
    fn now(&self) -> u64; // ms
    fn log(&self, msg: String);
}

pub struct WasmEnv;

impl Environment for WasmEnv {
    fn now(&self) -> u64 {
        Date::now() as u64
    }

    fn log(&self, msg: String) {
        // console_log! macro comes from worker crate and takes literal fmt string usually,
        // but we can pass formatted string if we use "%s".
        // Or actually console_log! invokes web_sys::console::log_1.
        console_log!("{}", msg);
    }
}

// Track client activity
pub struct ClientInfo {
    pub client: Box<dyn GameClient>,
    pub last_activity: u64, // Unix timestamp in seconds
    pub connected: bool,    // false while the slot is held open for a reconnect
}

// Game state wrapper for interior mutability
pub struct GameState {
    pub env: Box<dyn Environment>,
    pub sim: Simulation,
    pub clients: HashMap<u8, ClientInfo>, // player_id (0=left, 1=right) -> ClientInfo
    pub next_player_id: u8,
    pub match_state: MatchState,
    pub countdown_remaining: u8, // Countdown seconds remaining (3, 2, 1, 0)
    pub tick: u32,
    pub last_input: HashMap<u8, i8>, // Track last input per player to reduce logging
    pub last_tick_time: u64,         // Unix timestamp in ms
    pub accumulator: f32,            // For alarm loop catch-up timing
    pub reconnect_deadline_ms: u64,  // While Paused: forfeit once env.now() passes this
}

impl GameState {
    pub fn new(env: Box<dyn Environment>) -> Self {
        // Server uses a fixed seed; matches stay deterministic per Durable Object.
        let mut sim = Simulation::new(12345);

        // Ball starts at center (created by Simulation::new); serve toward the right.
        let initial = sim.config.ball_speed_initial;
        sim.ball.vel = glam::Vec2::new(initial, 0.0);

        let now = env.now();

        Self {
            env,
            sim,
            clients: HashMap::new(),
            next_player_id: 0,
            match_state: MatchState::Waiting,
            countdown_remaining: 3,
            tick: 0,
            last_input: HashMap::new(),
            last_tick_time: now,
            accumulator: 0.0,
            reconnect_deadline_ms: 0,
        }
    }

    /// Try to add a player. Returns (player_id, was_empty) if successful.
    pub fn add_player(&mut self, client: Box<dyn GameClient>) -> Option<(u8, bool)> {
        if self.clients.len() >= 2 {
            return None;
        }

        let player_id = self.next_player_id;
        self.next_player_id = (self.next_player_id + 1) % 2;

        let was_empty = self.clients.is_empty();
        let now = self.env.now() / 1000;

        self.clients.insert(
            player_id,
            ClientInfo {
                client,
                last_activity: now,
                connected: true,
            },
        );

        // Spawn paddle
        let paddle_y = self.sim.map.paddle_spawn(PlayerId(player_id)).y;
        self.sim.add_paddle(PlayerId(player_id), paddle_y);

        // Check if match can start
        if self.clients.len() == 2 && self.match_state == MatchState::Waiting {
            self.env
                .log("DO: Both players connected, starting countdown".to_string());
            self.match_state = MatchState::Countdown;
            self.countdown_remaining = 3;
            self.broadcast_to_all(&S2C::MatchFound);
        }

        Some((player_id, was_empty))
    }

    /// Broadcast a message to all connected clients
    pub fn broadcast_to_all(&self, msg: &S2C) {
        if let Ok(bytes) = msg.to_bytes() {
            for client_info in self.clients.values() {
                let _ = client_info.client.send_bytes(&bytes);
            }
        }
    }

    /// Handle a socket close. Mid-match this starts a reconnect grace period
    /// (slot kept, sim frozen); otherwise the player is removed immediately.
    pub fn handle_disconnect(&mut self, player_id: u8) {
        let mid_match = matches!(
            self.match_state,
            MatchState::Playing | MatchState::Countdown
        );
        if mid_match && self.clients.contains_key(&player_id) {
            if let Some(client_info) = self.clients.get_mut(&player_id) {
                client_info.connected = false;
            }
            self.match_state = MatchState::Paused;
            self.reconnect_deadline_ms = self.env.now() + RECONNECT_GRACE_MS;
            self.env.log(format!(
                "DO: Player {player_id} dropped mid-match; pausing for reconnect"
            ));
            self.broadcast_to_all(&S2C::OpponentReconnecting);
        } else {
            self.remove_player(player_id);
        }
    }

    /// Re-attach a returning player to their held-open slot and resume the match.
    /// Returns the reconnected player id, or None if there is no slot to resume.
    pub fn reconnect_player(&mut self, client: Box<dyn GameClient>) -> Option<u8> {
        let pid = self
            .clients
            .iter()
            .find_map(|(&p, info)| if !info.connected { Some(p) } else { None })?;
        let now = self.env.now() / 1000;
        if let Some(client_info) = self.clients.get_mut(&pid) {
            client_info.client = client;
            client_info.connected = true;
            client_info.last_activity = now;
        }
        self.env
            .log(format!("DO: Player {pid} reconnected; resuming match"));
        self.broadcast_to_all(&S2C::OpponentReconnected);
        // Resume with a fresh ready-countdown; the world (ball, paddles, score) is preserved.
        self.match_state = MatchState::Countdown;
        self.countdown_remaining = 3;
        self.reconnect_deadline_ms = 0;
        Some(pid)
    }

    pub fn remove_player(&mut self, player_id: u8) {
        self.clients.remove(&player_id);

        // Despawn paddle
        self.sim.remove_paddle(PlayerId(player_id));

        // Handle disconnection based on match state
        match self.match_state {
            MatchState::Playing => {
                // Forfeit: remaining player wins
                if let Some(&remaining_player) = self.clients.keys().next() {
                    self.broadcast_game_over(remaining_player);
                }
                self.match_state = MatchState::GameOver;
            }
            MatchState::Countdown => {
                // Cancel countdown, notify remaining player
                self.broadcast_to_all(&S2C::OpponentDisconnected);
                self.match_state = MatchState::Waiting;
                self.countdown_remaining = 3;
            }
            MatchState::GameOver => {
                // Notify remaining player that opponent left (won't rematch)
                self.broadcast_to_all(&S2C::OpponentDisconnected);
                // Reset to waiting state
                self.match_state = MatchState::Waiting;
            }
            MatchState::Paused => {
                // A reconnect grace period ended (or the other player also left).
                // Whoever is still in the room wins by forfeit.
                if let Some(&remaining_player) = self.clients.keys().next() {
                    self.broadcast_game_over(remaining_player);
                    self.match_state = MatchState::GameOver;
                } else {
                    self.match_state = MatchState::Waiting;
                }
            }
            MatchState::Waiting => {
                // Just update state
                if self.clients.is_empty() {
                    self.match_state = MatchState::Waiting;
                }
            }
        }
    }

    pub fn handle_input(&mut self, player_id: u8, y: f32) {
        if let Some(client_info) = self.clients.get_mut(&player_id) {
            let now = self.env.now() / 1000;
            client_info.last_activity = now;
            self.sim.net_queue.push_input(PlayerId(player_id), y);
        }
    }

    /// Reset game state for a rematch
    pub fn restart_match(&mut self) {
        if self.match_state != MatchState::GameOver {
            return;
        }

        self.env.log("DO: Restarting match".to_string());

        // Reset game data
        self.sim.score = Score::new();
        self.sim.events = Events::new();
        self.tick = 0;
        self.last_input.clear();
        self.sim.net_queue = NetQueue::new();
        self.accumulator = 0.0;
        self.last_tick_time = self.env.now();
        self.sim.time = Time::default();

        // Reset entities (keep clients)
        let ball_pos = self.sim.map.ball_spawn();
        let initial = self.sim.config.ball_speed_initial;
        self.sim.ball = Ball::new(ball_pos, glam::Vec2::new(initial, 0.0));
        self.sim.paddles.clear();
        for &player_id in self.clients.keys() {
            let paddle_y = self.sim.map.paddle_spawn(PlayerId(player_id)).y;
            self.sim.add_paddle(PlayerId(player_id), paddle_y);
        }

        // Set state to countdown
        self.match_state = MatchState::Countdown;
        self.countdown_remaining = 3;

        // Notify clients
        self.broadcast_to_all(&S2C::Countdown { seconds: 3 });
    }

    /// Process one countdown tick. Returns true if countdown finished.
    pub fn tick_countdown(&mut self) -> bool {
        if self.match_state != MatchState::Countdown {
            return false;
        }

        if self.countdown_remaining > 0 {
            self.broadcast_to_all(&S2C::Countdown {
                seconds: self.countdown_remaining,
            });
            self.env
                .log(format!("DO: Countdown: {}", self.countdown_remaining));
            self.countdown_remaining -= 1;
            false
        } else {
            // Countdown finished, start game
            self.env
                .log("DO: Countdown complete, starting game!".to_string());
            self.match_state = MatchState::Playing;
            self.broadcast_to_all(&S2C::GameStart);
            true
        }
    }

    pub fn step(&mut self) -> Option<u8> {
        if self.match_state != MatchState::Playing {
            return None;
        }

        self.tick += 1;

        if self.tick % 60 == 0 {
            self.env.log(format!(
                "DO: Game running, tick={}, clients={}",
                self.tick,
                self.clients.len()
            ));
        }

        self.sim.step();

        // Return winner if any
        if let Some(winner) = self.sim.score.has_winner(self.sim.config.win_score) {
            self.broadcast_game_over(winner.0);
            self.match_state = MatchState::GameOver;
            return Some(winner.0);
        }

        None
    }

    pub fn generate_state_message(&self) -> S2C {
        // Ball position and velocity
        let ball = self.sim.ball;
        let (ball_x, ball_y, ball_vx, ball_vy) = (ball.pos.x, ball.pos.y, ball.vel.x, ball.vel.y);

        // Paddle positions
        let mut paddle_left_y = 12.0;
        let mut paddle_right_y = 12.0;
        for paddle in &self.sim.paddles {
            if paddle.player_id == PlayerId(0) {
                paddle_left_y = paddle.y;
            } else if paddle.player_id == PlayerId(1) {
                paddle_right_y = paddle.y;
            }
        }
        let paddle_count = self.sim.paddles.len();

        if self.tick % 60 == 0 {
            self.env.log(format!(
                "DO: Paddle state - count={paddle_count}, left_y={paddle_left_y:.1}, right_y={paddle_right_y:.1}"
            ));
        }

        S2C::GameState(GameStateSnapshot {
            tick: self.tick,
            ball_x,
            ball_y,
            ball_vx,
            ball_vy,
            paddle_left_y,
            paddle_right_y,
            score_left: self.sim.score.left,
            score_right: self.sim.score.right,
        })
    }

    pub fn broadcast_state(&self) {
        if self.clients.is_empty() {
            return;
        }

        let state_msg = self.generate_state_message();
        if let Ok(bytes) = state_msg.to_bytes() {
            for client_info in self.clients.values() {
                let _ = client_info.client.send_bytes(&bytes);
            }
        }
    }

    pub fn broadcast_game_over(&self, winner: u8) {
        let msg = S2C::GameOver { winner };
        if let Ok(bytes) = msg.to_bytes() {
            for client_info in self.clients.values() {
                let _ = client_info.client.send_bytes(&bytes);
            }
        }
    }
}
