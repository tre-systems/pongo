use crate::PlayerId;
use glam::Vec2;

/// Simple Pong arena - just the dimensions
#[derive(Debug, Clone)]
pub struct GameMap {
    pub width: f32,
    pub height: f32,
}

impl GameMap {
    /// Create standard Pong arena (32 x 24)
    pub fn new() -> Self {
        Self {
            width: crate::config::Params::ARENA_WIDTH,
            height: crate::config::Params::ARENA_HEIGHT,
        }
    }

    /// Get spawn position for paddle based on player ID
    pub fn paddle_spawn(&self, player_id: PlayerId) -> Vec2 {
        let x = if player_id == PlayerId::LEFT {
            1.0 // Left paddle
        } else {
            self.width - 1.0 // Right paddle
        };
        let y = self.height / 2.0; // Center vertically
        Vec2::new(x, y)
    }

    /// Get ball spawn position (center of arena)
    pub fn ball_spawn(&self) -> Vec2 {
        Vec2::new(self.width / 2.0, self.height / 2.0)
    }

    /// Check if Y position is within arena bounds
    pub fn is_valid_y(&self, y: f32, half_height: f32) -> bool {
        y >= half_height && y <= self.height - half_height
    }

    /// Clamp Y position to arena bounds
    pub fn clamp_y(&self, y: f32, half_height: f32) -> f32 {
        y.clamp(half_height, self.height - half_height)
    }
}

impl Default for GameMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_default_dimensions() {
        let map = GameMap::new();
        assert_eq!(map.width, 32.0);
        assert_eq!(map.height, 24.0);
    }

    #[test]
    fn test_paddle_spawn_left() {
        let map = GameMap::new();
        let pos = map.paddle_spawn(PlayerId(0));
        assert_eq!(pos.x, 1.0, "Left paddle should spawn at x=1");
        assert_eq!(pos.y, 12.0, "Paddle should spawn at center Y");
    }

    #[test]
    fn test_paddle_spawn_right() {
        let map = GameMap::new();
        let pos = map.paddle_spawn(PlayerId(1));
        assert_eq!(pos.x, 31.0, "Right paddle should spawn at x=31");
        assert_eq!(pos.y, 12.0, "Paddle should spawn at center Y");
    }

    #[test]
    fn test_ball_spawn() {
        let map = GameMap::new();
        let pos = map.ball_spawn();
        assert_eq!(pos.x, 16.0, "Ball should spawn at center X");
        assert_eq!(pos.y, 12.0, "Ball should spawn at center Y");
    }

    #[test]
    fn test_is_valid_y() {
        let map = GameMap::new();
        let half_height = 2.0;

        // Valid positions
        assert!(map.is_valid_y(12.0, half_height), "Center should be valid");
        assert!(
            map.is_valid_y(half_height, half_height),
            "Top boundary should be valid"
        );
        assert!(
            map.is_valid_y(map.height - half_height, half_height),
            "Bottom boundary should be valid"
        );

        // Invalid positions
        assert!(
            !map.is_valid_y(half_height - 0.1, half_height),
            "Above top should be invalid"
        );
        assert!(
            !map.is_valid_y(map.height - half_height + 0.1, half_height),
            "Below bottom should be invalid"
        );
    }

    #[test]
    fn test_clamp_y() {
        let map = GameMap::new();
        let half_height = 2.0;

        // Test clamping below minimum
        assert_eq!(map.clamp_y(0.0, half_height), half_height);

        // Test clamping above maximum
        assert_eq!(map.clamp_y(100.0, half_height), map.height - half_height);

        // Test valid value (no clamping)
        let valid_y = 12.0;
        assert_eq!(map.clamp_y(valid_y, half_height), valid_y);
    }
}
