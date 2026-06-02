//! Game state management with interpolation

pub use proto::GameStateSnapshot;

/// Events from server for match lifecycle
#[derive(Debug, Clone, PartialEq)]
pub enum MatchEvent {
    None,
    MatchFound,
    Countdown(u8),
    GameStart,
    OpponentDisconnected,
    OpponentReconnecting,
    OpponentReconnected,
}

// Exponential-smoothing time constants (seconds), chosen so a 1/60s frame
// reproduces the original per-frame blend factors (ball 0.3, paddle 0.25):
//   tau = -(1/60) / ln(1 - factor).
// Deriving each frame's factor from dt as `1 - exp(-dt/tau)` makes convergence
// frame-rate independent, so remote motion feels the same at 30, 60, or 120 Hz.
const BALL_SMOOTHING_TAU: f32 = 0.0467;
const PADDLE_SMOOTHING_TAU: f32 = 0.0579;

/// Game state tracking with interpolation
pub struct GameState {
    // Current authoritative state from server
    current: GameStateSnapshot,
    // Time since last state update
    time_since_update: f32,
    // Score (doesn't need interpolation)
    score_left: u8,
    score_right: u8,
    pub my_player_id: Option<u8>,
    pub winner: Option<u8>,
    // Smooth correction state for ball position
    ball_display_x: f32,
    ball_display_y: f32,
    // Smooth correction state for paddle positions (opponent paddles)
    paddle_left_display_y: f32,
    paddle_right_display_y: f32,
    // Latest match event from server
    pub match_event: MatchEvent,
}

/// Initial snapshot with the ball and both paddles centred in the arena.
fn initial_snapshot() -> GameStateSnapshot {
    GameStateSnapshot {
        ball_x: 16.0,
        ball_y: 12.0,
        paddle_left_y: 12.0,
        paddle_right_y: 12.0,
        ball_vx: 0.0,
        ball_vy: 0.0,
        tick: 0,
        score_left: 0,
        score_right: 0,
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current: initial_snapshot(),
            time_since_update: 0.0,
            score_left: 0,
            score_right: 0,
            my_player_id: None,
            winner: None,
            ball_display_x: 16.0,
            ball_display_y: 12.0,
            paddle_left_display_y: 12.0,
            paddle_right_display_y: 12.0,
            match_event: MatchEvent::None,
        }
    }

    pub fn reset(&mut self) {
        self.current = initial_snapshot();
        self.time_since_update = 0.0;
        self.score_left = 0;
        self.score_right = 0;
        self.winner = None;
        self.ball_display_x = 16.0;
        self.ball_display_y = 12.0;
        self.paddle_left_display_y = 12.0;
        self.paddle_right_display_y = 12.0;
        self.match_event = MatchEvent::None;
    }

    /// Update interpolation based on elapsed time.
    /// Server broadcasts at ~20Hz (every 3rd 60Hz tick); render runs at the
    /// display refresh via requestAnimationFrame.
    pub fn update_interpolation(&mut self, dt: f32) {
        self.time_since_update += dt;

        // Smoothly blend display position toward target using exponential smoothing
        // This prevents jarring jumps when new server state arrives
        let target_x = self.extrapolate_ball_internal(self.current.ball_x, self.current.ball_vx);
        let target_y = self.extrapolate_ball_internal(self.current.ball_y, self.current.ball_vy);

        // Frame-rate-independent exponential smoothing (see the TAU consts above):
        // blend toward the target by `1 - exp(-dt/tau)` so convergence tracks real
        // elapsed time rather than frame count.
        let smoothing = 1.0 - (-dt / BALL_SMOOTHING_TAU).exp();
        self.ball_display_x += (target_x - self.ball_display_x) * smoothing;
        self.ball_display_y += (target_y - self.ball_display_y) * smoothing;

        let paddle_smoothing = 1.0 - (-dt / PADDLE_SMOOTHING_TAU).exp();
        self.paddle_left_display_y +=
            (self.current.paddle_left_y - self.paddle_left_display_y) * paddle_smoothing;
        self.paddle_right_display_y +=
            (self.current.paddle_right_y - self.paddle_right_display_y) * paddle_smoothing;
    }

    /// Internal extrapolation with clamped time to prevent overshooting
    fn extrapolate_ball_internal(&self, pos: f32, vel: f32) -> f32 {
        // Clamp extrapolation to max 100ms to prevent large jumps on network delays
        let clamped_time = self.time_since_update.min(0.100);
        pos + vel * clamped_time
    }

    /// Get current ball X with smooth display position
    pub fn get_ball_x(&self) -> f32 {
        self.ball_display_x
    }

    /// Get current ball Y with smooth display position
    pub fn get_ball_y(&self) -> f32 {
        self.ball_display_y
    }

    pub fn get_paddle_left_y(&self) -> f32 {
        self.paddle_left_display_y
    }

    pub fn get_paddle_right_y(&self) -> f32 {
        self.paddle_right_display_y
    }

    pub fn set_current(&mut self, snapshot: GameStateSnapshot) {
        // Accept the snapshot as the new authoritative state and restart the
        // smoothing window.
        self.current = snapshot;
        self.time_since_update = 0.0;
    }

    pub fn set_scores(&mut self, left: u8, right: u8) {
        self.score_left = left;
        self.score_right = right;
    }

    pub fn get_scores(&self) -> (u8, u8) {
        (self.score_left, self.score_right)
    }

    pub fn set_player_id(&mut self, player_id: u8) {
        self.my_player_id = Some(player_id);
    }

    pub fn get_player_id(&self) -> Option<u8> {
        self.my_player_id
    }

    pub fn set_winner(&mut self, winner: u8) {
        self.winner = Some(winner);
    }

    pub fn get_current_snapshot(&self) -> GameStateSnapshot {
        self.current.clone()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_gamestate_has_initial_values() {
        let state = GameState::new();
        assert_eq!(state.get_ball_x(), 16.0);
        assert_eq!(state.get_ball_y(), 12.0);
        assert_eq!(state.get_paddle_left_y(), 12.0);
        assert_eq!(state.get_paddle_right_y(), 12.0);
        assert_eq!(state.get_scores(), (0, 0));
        assert!(state.get_player_id().is_none());
    }

    #[test]
    fn test_reset_clears_all_state() {
        let mut state = GameState::new();

        // Modify state
        state.set_scores(3, 5);
        state.set_player_id(1);
        state.set_winner(0);
        state.set_current(GameStateSnapshot {
            ball_x: 20.0,
            ball_y: 20.0,
            paddle_left_y: 5.0,
            paddle_right_y: 19.0,
            ball_vx: 10.0,
            ball_vy: -5.0,
            tick: 100,
            score_left: 3,
            score_right: 5,
        });

        // Reset
        state.reset();

        // Verify reset values
        assert_eq!(state.get_scores(), (0, 0));
        assert_eq!(state.get_ball_x(), 16.0);
        assert_eq!(state.get_ball_y(), 12.0);
        assert_eq!(state.get_paddle_left_y(), 12.0);
        assert_eq!(state.get_paddle_right_y(), 12.0);
        assert_eq!(state.match_event, MatchEvent::None);
    }

    #[test]
    fn test_paddle_smoothing_converges() {
        let mut state = GameState::new();

        // Set a new paddle position
        state.set_current(GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 20.0, // Target: 20
            paddle_right_y: 4.0, // Target: 4
            ball_vx: 0.0,
            ball_vy: 0.0,
            tick: 1,
            score_left: 0,
            score_right: 0,
        });

        // Initial display positions are at 12.0
        assert_eq!(state.get_paddle_left_y(), 12.0);
        assert_eq!(state.get_paddle_right_y(), 12.0);

        // Apply smoothing multiple times
        for _ in 0..20 {
            state.update_interpolation(0.016); // ~60fps
        }

        // After smoothing, paddles should be close to target
        assert!((state.get_paddle_left_y() - 20.0).abs() < 0.5);
        assert!((state.get_paddle_right_y() - 4.0).abs() < 0.5);
    }

    #[test]
    fn test_ball_display_smoothing() {
        let mut state = GameState::new();

        // Set ball at new position
        state.set_current(GameStateSnapshot {
            ball_x: 25.0,
            ball_y: 20.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            ball_vx: 10.0,
            ball_vy: 5.0,
            tick: 1,
            score_left: 0,
            score_right: 0,
        });

        // Initial display at 16, 12
        let initial_x = state.get_ball_x();
        let initial_y = state.get_ball_y();

        // Apply smoothing
        state.update_interpolation(0.016);

        // Ball should have moved toward target (with extrapolation)
        let after_x = state.get_ball_x();
        let after_y = state.get_ball_y();

        assert!(after_x > initial_x, "Ball X should increase toward target");
        assert!(after_y > initial_y, "Ball Y should increase toward target");
    }

    #[test]
    fn test_smoothing_is_frame_rate_independent() {
        // The same elapsed time must converge equally regardless of how many
        // frames it is split into — the whole point of the dt-based factor. Uses
        // the paddle, whose target is static (no extrapolation moving the goal).
        let snapshot = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 20.0,
            paddle_right_y: 12.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            tick: 1,
            score_left: 0,
            score_right: 0,
        };

        let mut coarse = GameState::new();
        coarse.set_current(snapshot.clone());
        coarse.update_interpolation(0.040); // one 40ms frame

        let mut fine = GameState::new();
        fine.set_current(snapshot);
        for _ in 0..4 {
            fine.update_interpolation(0.010); // four 10ms frames = same 40ms total
        }

        assert!(
            (coarse.get_paddle_left_y() - fine.get_paddle_left_y()).abs() < 1e-3,
            "convergence must match across frame rates: coarse={}, fine={}",
            coarse.get_paddle_left_y(),
            fine.get_paddle_left_y()
        );
    }
}
