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

/// Game state tracking with interpolation
pub struct GameState {
    // Current authoritative state from server
    current: GameStateSnapshot,
    // Previous state for interpolation
    previous: GameStateSnapshot,
    // Interpolation time (0.0 = previous, 1.0 = current)
    interpolation_alpha: f32,
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

impl GameState {
    pub fn new() -> Self {
        let initial = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            tick: 0,
            score_left: 0,
            score_right: 0,
        };
        Self {
            current: initial.clone(),
            previous: initial,
            interpolation_alpha: 1.0,
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
        let initial = GameStateSnapshot {
            ball_x: 16.0,
            ball_y: 12.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            tick: 0,
            score_left: 0,
            score_right: 0,
        };
        self.current = initial.clone();
        self.previous = initial;
        self.interpolation_alpha = 1.0;
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

    /// Update interpolation based on elapsed time
    /// Target: 60fps render, 20-60Hz server updates
    pub fn update_interpolation(&mut self, dt: f32) {
        self.time_since_update += dt;
        // Server sends updates at 20Hz (50ms). Use 100ms (2x) for jitter tolerance.
        let interpolation_duration = 0.100;
        self.interpolation_alpha = (self.time_since_update / interpolation_duration).min(1.0);

        // Smoothly blend display position toward target using exponential smoothing
        // This prevents jarring jumps when new server state arrives
        let target_x = self.extrapolate_ball_internal(self.current.ball_x, self.current.ball_vx);
        let target_y = self.extrapolate_ball_internal(self.current.ball_y, self.current.ball_vy);

        // Smoothing factor: higher = faster convergence (0.3 = ~3 frames to 90% convergence)
        let smoothing = 0.3;
        self.ball_display_x += (target_x - self.ball_display_x) * smoothing;
        self.ball_display_y += (target_y - self.ball_display_y) * smoothing;

        // Apply same exponential smoothing to paddle positions for smooth opponent movement
        // Lower smoothing (0.25) = smoother motion, slightly more latency
        let paddle_smoothing = 0.25;
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
        // Simple version: just accept all incoming snapshots
        self.previous = self.current.clone();
        self.current = snapshot;
        self.time_since_update = 0.0;
        self.interpolation_alpha = 0.0;
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

    pub fn time_since_update(&self) -> f32 {
        self.time_since_update
    }

    pub fn get_current_snapshot(&self) -> Option<GameStateSnapshot> {
        Some(self.current.clone())
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
}
