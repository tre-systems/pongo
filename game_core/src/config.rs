use crate::PlayerId;
use glam::Vec2;

/// Game tuning parameters for Pong. A namespace for tuning constants; never instantiated.
pub struct Params;

impl Params {
    // Arena
    pub const ARENA_WIDTH: f32 = 32.0;
    pub const ARENA_HEIGHT: f32 = 24.0;

    // Paddle
    pub const PADDLE_WIDTH: f32 = 0.8;
    pub const PADDLE_HEIGHT: f32 = 4.0;
    pub const PADDLE_SPEED: f32 = 18.0;

    // Ball
    pub const BALL_RADIUS: f32 = 0.5;
    pub const BALL_SPEED_INITIAL: f32 = 12.0;
    pub const BALL_SPEED_MAX: f32 = 24.0;
    pub const BALL_SPEED_INCREASE: f32 = 1.05;
    pub const BALL_PADDLE_OVERLAP: f32 = 0.4;

    // Score
    pub const WIN_SCORE: u8 = 5;

    /// Delay (seconds) after a goal before the ball is re-served.
    pub const RESPAWN_DELAY: f32 = 1.5;

    // Physics
    /// The single fixed simulation timestep (60 Hz). Every host advances the
    /// simulation at this rate; see `Simulation::step`.
    pub const FIXED_DT: f32 = 1.0 / 60.0;
}

/// Game configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub arena_width: f32,
    pub arena_height: f32,
    pub paddle_width: f32,
    pub paddle_height: f32,
    pub paddle_speed: f32,
    pub ball_radius: f32,
    pub ball_speed_initial: f32,
    pub ball_speed_max: f32,
    pub ball_speed_increase: f32,
    pub ball_paddle_overlap: f32,
    pub win_score: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            arena_width: Params::ARENA_WIDTH,
            arena_height: Params::ARENA_HEIGHT,
            paddle_width: Params::PADDLE_WIDTH,
            paddle_height: Params::PADDLE_HEIGHT,
            paddle_speed: Params::PADDLE_SPEED,
            ball_radius: Params::BALL_RADIUS,
            ball_speed_initial: Params::BALL_SPEED_INITIAL,
            ball_speed_max: Params::BALL_SPEED_MAX,
            ball_speed_increase: Params::BALL_SPEED_INCREASE,
            ball_paddle_overlap: Params::BALL_PADDLE_OVERLAP,
            win_score: Params::WIN_SCORE,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    /// Paddle center X for a side (left at a fixed inset, right mirrored).
    pub fn paddle_x(&self, player_id: PlayerId) -> f32 {
        if player_id == PlayerId::LEFT {
            1.5
        } else {
            self.arena_width - 1.5
        }
    }

    /// Ball spawn point: the center of the arena.
    pub fn ball_spawn(&self) -> Vec2 {
        Vec2::new(self.arena_width / 2.0, self.arena_height / 2.0)
    }

    /// Paddle spawn Y: vertically centered (both sides start at the arena's mid-height).
    pub fn paddle_spawn_y(&self) -> f32 {
        self.arena_height / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_paddle_x() {
        let config = Config::new();
        assert_eq!(config.paddle_x(PlayerId(0)), 1.5, "Left paddle X position");
        assert_eq!(
            config.paddle_x(PlayerId(1)),
            30.5,
            "Right paddle X position"
        );
    }
}
