use glam::Vec2;

/// Identifies a player / side of the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u8);

impl PlayerId {
    /// The left player.
    pub const LEFT: PlayerId = PlayerId(0);
    /// The right player.
    pub const RIGHT: PlayerId = PlayerId(1);

    /// 0-based index (0 = left, 1 = right), for arrays and side-keyed lookups.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A player's paddle: its position plus the movement intent driving it.
#[derive(Debug, Clone, Copy)]
pub struct Paddle {
    pub player_id: PlayerId,
    pub y: f32,          // current Y (clamped to arena)
    pub target_y: f32,   // desired Y from the latest input
    pub velocity_y: f32, // realized vertical velocity from the last move (units/sec)
}

impl Paddle {
    pub fn new(player_id: PlayerId, y: f32) -> Self {
        Self {
            player_id,
            y,
            target_y: y,
            velocity_y: 0.0,
        }
    }
}

/// Ball - the pong ball
#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl Ball {
    pub fn new(pos: Vec2, vel: Vec2) -> Self {
        Self { pos, vel }
    }

    /// Reset ball to center with a random direction
    pub fn reset(&mut self, speed: f32, rng: &mut crate::GameRng) {
        self.pos = Vec2::new(16.0, 12.0); // Center of 32x24 arena

        // Random angle between -45° and 45°, or 135° and 225°
        use rand::Rng;
        let right = rng.0.gen_bool(0.5);
        let angle: f32 = if right {
            rng.0.gen_range(-0.785..0.785) // -45° to 45° in radians
        } else {
            rng.0.gen_range(2.356..3.927) // 135° to 225° in radians
        };

        self.vel = Vec2::new(angle.cos(), angle.sin()) * speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paddle_new() {
        let paddle = Paddle::new(PlayerId(0), 12.0);
        assert_eq!(paddle.player_id, PlayerId(0));
        assert_eq!(paddle.y, 12.0);
        assert_eq!(paddle.target_y, 12.0);
        assert_eq!(paddle.velocity_y, 0.0);
    }

    #[test]
    fn test_ball_new() {
        let pos = Vec2::new(16.0, 12.0);
        let vel = Vec2::new(8.0, 4.0);
        let ball = Ball::new(pos, vel);
        assert_eq!(ball.pos, pos);
        assert_eq!(ball.vel, vel);
    }

    #[test]
    fn test_ball_reset() {
        let mut ball = Ball::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let mut rng = crate::GameRng::new(12345);
        let speed = 8.0;

        ball.reset(speed, &mut rng);

        assert_eq!(ball.pos, Vec2::new(16.0, 12.0));
        assert!((ball.vel.length() - speed).abs() < 0.01);
        assert!(ball.vel.length() > 0.0);
    }
}
