use crate::{Ball, Config, GameMap, Paddle, PaddleIntent};
use hecs::World;

/// Apply paddle movement based on intents (Server-Side Validation)
pub fn move_paddles(world: &mut World, map: &GameMap, config: &Config, dt: f32) {
    for (_entity, (paddle, intent)) in world.query_mut::<(&mut Paddle, &mut PaddleIntent)>() {
        let start_y = paddle.y;

        // Calculate distance to target
        let diff = intent.target_y - paddle.y;

        // If already at target (within epsilon), do nothing
        if diff.abs() < 0.01 {
            paddle.y = intent.target_y;
        } else {
            // Cap movement by max speed
            let max_move = config.paddle_speed * dt;
            let move_dist = diff.clamp(-max_move, max_move);

            paddle.y += move_dist;
        }

        // Clamp to arena bounds (safety fallback)
        paddle.y = map.clamp_y(paddle.y, config.paddle_height / 2.0);

        // Record the realized vertical velocity (after clamping) so collisions can
        // impart the paddle's motion onto the ball ("english"/slice). At a wall the
        // paddle can't move, so velocity is 0 and no spin is added.
        intent.velocity_y = if dt > 0.0 {
            (paddle.y - start_y) / dt
        } else {
            0.0
        };
    }
}

/// Move ball based on velocity
pub fn move_ball(world: &mut World, dt: f32) {
    for (_entity, ball) in world.query_mut::<&mut Ball>() {
        ball.pos += ball.vel * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_paddle, Config, GameMap, Paddle, PaddleIntent, Time};

    fn setup_world() -> (World, Config, GameMap, Time) {
        let world = World::new();
        let config = Config::new();
        let map = GameMap::new();
        let time = Time::new(0.016, 0.0); // 60 Hz
        (world, config, map, time)
    }

    #[test]
    fn test_paddle_moves_towards_target_up() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set target UP (smaller Y)
        let target = 5.0;
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.target_y = target;
        }

        move_paddles(&mut world, &map, &config, time.dt);

        // Verify paddle moved towards target LIMITED by speed
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert!(paddle.y < paddle_y, "Paddle should move up");
            // Should move exactly max speed
            let expected_y = paddle_y - config.paddle_speed * time.dt;
            assert!((paddle.y - expected_y).abs() < 0.001);
        }
    }

    #[test]
    fn test_paddle_moves_towards_target_down() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set target DOWN (larger Y)
        let target = 18.0;
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.target_y = target;
        }

        move_paddles(&mut world, &map, &config, time.dt);

        // Verify paddle moved towards target LIMITED by speed
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert!(paddle.y > paddle_y, "Paddle should move down");
            // Should move exactly max speed
            let expected_y = paddle_y + config.paddle_speed * time.dt;
            assert!((paddle.y - expected_y).abs() < 0.001);
        }
    }

    #[test]
    fn test_paddle_teleport_attempt_throttled() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set target HUGE jump
        let target = 20.0; // 8 units away
        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.target_y = target;
        }

        move_paddles(&mut world, &map, &config, time.dt);

        // Verify paddle ONLY moved max speed, not teleported
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            let max_dist = config.paddle_speed * time.dt;
            assert!(
                (paddle.y - paddle_y).abs() <= max_dist + 0.001,
                "Paddle should be throttled"
            );
            assert!(
                paddle.y < target,
                "Paddle should not have reached target yet"
            );
        }
    }

    #[test]
    fn test_paddle_reaches_target_if_close() {
        let (mut world, config, map, time) = setup_world();
        let paddle_y = 12.0;
        create_paddle(&mut world, 0, paddle_y);

        // Set target very close (less than max move)
        let small_dist = 0.1;
        // Make sure this is less than speed * dt
        assert!(small_dist < config.paddle_speed * time.dt);

        let target = paddle_y + small_dist;

        for (_entity, (_paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.target_y = target;
        }

        move_paddles(&mut world, &map, &config, time.dt);

        // Verify reached exactly
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            assert!((paddle.y - target).abs() < 0.001, "Should snap to target");
        }
    }
}
