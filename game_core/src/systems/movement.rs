use crate::{Ball, Config, Paddle};

/// Move each paddle toward its target Y, capped by paddle_speed and clamped to the arena.
pub fn move_paddles(paddles: &mut [Paddle], config: &Config, dt: f32) {
    let half_height = config.paddle_height / 2.0;
    for paddle in paddles.iter_mut() {
        let start_y = paddle.y;

        let diff = paddle.target_y - paddle.y;
        if diff.abs() < 0.01 {
            paddle.y = paddle.target_y;
        } else {
            // Cap movement by max speed.
            let max_move = config.paddle_speed * dt;
            paddle.y += diff.clamp(-max_move, max_move);
        }

        // Clamp to arena bounds.
        paddle.y = paddle
            .y
            .clamp(half_height, config.arena_height - half_height);

        // Record the realized vertical velocity (after clamping) so collisions can
        // impart the paddle's motion onto the ball ("english"/slice). At a wall the
        // paddle can't move, so velocity is 0 and no spin is added.
        paddle.velocity_y = if dt > 0.0 {
            (paddle.y - start_y) / dt
        } else {
            0.0
        };
    }
}

/// Move the ball by its velocity.
pub fn move_ball(ball: &mut Ball, dt: f32) {
    ball.pos += ball.vel * dt;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, PlayerId};

    const DT: f32 = 0.016; // ~60 Hz

    #[test]
    fn test_paddle_moves_towards_target_up() {
        let config = Config::new();
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        paddles[0].target_y = 5.0;

        move_paddles(&mut paddles, &config, DT);

        assert!(paddles[0].y < 12.0, "Paddle should move up");
        let expected = 12.0 - config.paddle_speed * DT;
        assert!((paddles[0].y - expected).abs() < 0.001);
    }

    #[test]
    fn test_paddle_moves_towards_target_down() {
        let config = Config::new();
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        paddles[0].target_y = 18.0;

        move_paddles(&mut paddles, &config, DT);

        assert!(paddles[0].y > 12.0, "Paddle should move down");
        let expected = 12.0 + config.paddle_speed * DT;
        assert!((paddles[0].y - expected).abs() < 0.001);
    }

    #[test]
    fn test_paddle_teleport_attempt_throttled() {
        let config = Config::new();
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        paddles[0].target_y = 20.0; // far jump

        move_paddles(&mut paddles, &config, DT);

        let max_dist = config.paddle_speed * DT;
        assert!((paddles[0].y - 12.0).abs() <= max_dist + 0.001, "throttled");
        assert!(paddles[0].y < 20.0, "should not reach target yet");
    }

    #[test]
    fn test_paddle_reaches_target_if_close() {
        let config = Config::new();
        let small = 0.1;
        assert!(small < config.paddle_speed * DT);
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        paddles[0].target_y = 12.0 + small;

        move_paddles(&mut paddles, &config, DT);

        assert!(
            (paddles[0].y - (12.0 + small)).abs() < 0.001,
            "should snap to target"
        );
    }

    #[test]
    fn test_move_ball() {
        let mut ball = Ball::new(glam::Vec2::new(16.0, 12.0), glam::Vec2::new(10.0, -5.0));
        move_ball(&mut ball, DT);
        assert!((ball.pos.x - (16.0 + 10.0 * DT)).abs() < 0.001);
        assert!((ball.pos.y - (12.0 - 5.0 * DT)).abs() < 0.001);
    }
}
