use crate::state::GameStateSnapshot;
use game_core::{create_ball, create_paddle, Params, PlayerId, Simulation};

#[allow(dead_code)]
pub struct ClientPredictor {
    pub input_seq: u32,

    // The predicted simulation. `None` when prediction is inactive — a single
    // source of truth, instead of nine parallel `Option` fields.
    sim: Option<Simulation>,

    // Reconciliation state
    pub last_reconciled_tick: u32,
    pub predicted_tick: u32,
    pub input_history: Vec<(u32, i8)>, // (seq, paddle_dir)

    // Timing
    pub accumulator: f32,
    pub last_update_time: f64, // ms
}

impl ClientPredictor {
    pub fn new() -> Self {
        Self {
            input_seq: 0,
            sim: None,
            last_reconciled_tick: 0,
            predicted_tick: 0,
            input_history: Vec::new(),
            accumulator: 0.0,
            last_update_time: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.sim.is_some()
    }

    pub fn initialize(&mut self, snapshot: &GameStateSnapshot, now_ms: f64) {
        let mut sim = Simulation::new(now_ms as u64);

        // Create paddles and the ball at the server's positions.
        create_paddle(&mut sim.world, PlayerId(0), snapshot.paddle_left_y);
        create_paddle(&mut sim.world, PlayerId(1), snapshot.paddle_right_y);
        create_ball(
            &mut sim.world,
            glam::f32::Vec2::new(snapshot.ball_x, snapshot.ball_y),
            glam::f32::Vec2::new(snapshot.ball_vx, snapshot.ball_vy),
        );

        self.sim = Some(sim);
        self.last_reconciled_tick = snapshot.tick;
        self.predicted_tick = snapshot.tick;
        self.accumulator = 0.0;
        self.last_update_time = now_ms;
    }

    /// Process local input immediately (prediction step)
    #[allow(dead_code)]
    pub fn process_input(&mut self, player_id: u8, paddle_dir: i8) {
        if let Some(sim) = &mut self.sim {
            let new_y = predicted_paddle_y(sim, player_id, paddle_dir);
            sim.net_queue.push_input(PlayerId(player_id), new_y);
            sim.step();
            self.predicted_tick += 1;
        }
    }

    /// Step prediction loop based on time delta
    pub fn update(&mut self, now_ms: f64, player_id: u8, current_input: i8) {
        if self.sim.is_none() {
            return;
        }

        // Init last time if needed
        if self.last_update_time == 0.0 {
            self.last_update_time = now_ms;
        }

        let frame_time_ms = (now_ms - self.last_update_time) / 1000.0;
        self.accumulator += frame_time_ms as f32;
        self.last_update_time = now_ms;

        while self.accumulator >= Params::FIXED_DT {
            self.accumulator -= Params::FIXED_DT;

            if let Some(sim) = &mut self.sim {
                sim.net_queue.clear();
                let new_y = predicted_paddle_y(sim, player_id, current_input);
                sim.net_queue.push_input(PlayerId(player_id), new_y);
                sim.step();
                self.predicted_tick += 1;
            }
        }
    }

    pub fn reconcile(&mut self, server_tick: u32) {
        if server_tick >= self.predicted_tick {
            // Server ahead or sync, reset prediction
            self.reset();
            self.last_reconciled_tick = server_tick;
            self.predicted_tick = server_tick;
            return;
        }

        let tick_diff = self.predicted_tick.saturating_sub(server_tick);
        if tick_diff > 20 {
            // Desync too large, reset
            self.reset();
            self.last_reconciled_tick = server_tick;
            self.predicted_tick = server_tick;
        } else {
            // Keep prediction
            self.last_reconciled_tick = server_tick;
        }
    }

    fn reset(&mut self) {
        self.sim = None;
    }

    #[allow(dead_code)]
    pub fn get_paddle_y(&self, player_id: u8) -> Option<f32> {
        if let Some(sim) = &self.sim {
            for (_entity, paddle) in sim.world.query::<&game_core::Paddle>().iter() {
                if paddle.player_id == PlayerId(player_id) {
                    return Some(paddle.y);
                }
            }
        }
        None
    }
}

/// Integrate a paddle's target Y one fixed step from a local input direction.
/// Shared by `process_input` and `update` so the prediction math lives in one place.
fn predicted_paddle_y(sim: &Simulation, player_id: u8, dir: i8) -> f32 {
    let mut current_y = 12.0;
    for (_entity, paddle) in sim.world.query::<&game_core::Paddle>().iter() {
        if paddle.player_id == PlayerId(player_id) {
            current_y = paddle.y;
            break;
        }
    }
    let new_y = current_y + (dir as f32) * sim.config.paddle_speed * Params::FIXED_DT;
    let half_height = sim.config.paddle_height / 2.0;
    new_y.clamp(half_height, sim.config.arena_height - half_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GameStateSnapshot;
    use wasm_bindgen_test::*;

    // Run in a headless browser so CI can execute these with `wasm-pack test --headless --chrome`.
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_initialization() {
        let mut predictor = ClientPredictor::new();
        let snapshot = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 5.0,
            ball_vy: 0.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            tick: 100,
            score_left: 0,
            score_right: 0,
        };

        predictor.initialize(&snapshot, 1000.0);

        assert!(predictor.is_active());
        assert_eq!(predictor.predicted_tick, 100);
        assert_eq!(predictor.last_reconciled_tick, 100);
        assert!(predictor.world.is_some());
    }

    #[wasm_bindgen_test]
    fn test_process_input() {
        let mut predictor = ClientPredictor::new();
        let snapshot = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 5.0,
            ball_vy: 0.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            tick: 100,
            score_left: 0,
            score_right: 0,
        };

        predictor.initialize(&snapshot, 1000.0);

        // Process input
        predictor.process_input(0, 1);

        assert_eq!(predictor.predicted_tick, 101);
    }

    #[wasm_bindgen_test]
    fn test_reconcile_sync() {
        let mut predictor = ClientPredictor::new();
        let snapshot = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 5.0,
            ball_vy: 0.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            tick: 100,
            score_left: 0,
            score_right: 0,
        };

        predictor.initialize(&snapshot, 1000.0);

        // Predict forward
        predictor.process_input(0, 1); // tick 101

        // Server confirms tick 101 (sync)
        predictor.reconcile(101);

        // Should reset prediction (assume server state will be re-applied in handle_message)
        assert!(!predictor.is_active());
        assert_eq!(predictor.last_reconciled_tick, 101);
    }

    #[wasm_bindgen_test]
    fn test_reconcile_behind_small() {
        let mut predictor = ClientPredictor::new();
        let snapshot = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 5.0,
            ball_vy: 0.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            tick: 100,
            score_left: 0,
            score_right: 0,
        };

        predictor.initialize(&snapshot, 1000.0);

        // Predict forward a bit
        for _ in 0..5 {
            predictor.process_input(0, 1);
        }
        // predicted_tick = 105

        // Server says it's at tick 103 (lag)
        predictor.reconcile(103);

        // Should keep prediction (active)
        assert!(predictor.is_active());
        assert_eq!(predictor.last_reconciled_tick, 103);
    }

    #[wasm_bindgen_test]
    fn test_reconcile_behind_large() {
        let mut predictor = ClientPredictor::new();
        let snapshot = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 5.0,
            ball_vy: 0.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            tick: 100,
            score_left: 0,
            score_right: 0,
        };

        predictor.initialize(&snapshot, 1000.0);

        // Predict forward A LOT (latency spike or stall)
        for _ in 0..30 {
            predictor.process_input(0, 1);
        }
        // predicted_tick = 130

        // Server says it's at tick 100 (frozen?)
        predictor.reconcile(100);

        // Should reset prediction
        assert!(!predictor.is_active());
        assert_eq!(predictor.last_reconciled_tick, 100);
    }
}
