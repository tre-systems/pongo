use crate::{Ball, Config, Events, GameMap, Paddle, PaddleIntent, PlayerId};
use hecs::World;

/// Check ball collisions with walls and paddles
pub fn check_collisions(world: &mut World, map: &GameMap, config: &Config, events: &mut Events) {
    let mut ball_query = world.query::<&mut Ball>();
    let ball_opt = ball_query.iter().next().map(|(_, b)| b);

    if let Some(ball) = ball_opt {
        // Wall collisions
        handle_wall_collision(ball, map, config, events);

        // Paddle collisions
        // Collect paddle info first to avoid borrow conflicts
        let paddles: Vec<(PlayerId, f32, f32)> = world
            .query::<(&Paddle, &PaddleIntent)>()
            .iter()
            .map(|(_e, (p, intent))| (p.player_id, p.y, intent.velocity_y))
            .collect();

        for (player_id, paddle_y, paddle_velocity_y) in paddles {
            handle_paddle_collision(ball, player_id, paddle_y, paddle_velocity_y, config, events);
        }
    }
}

fn handle_wall_collision(ball: &mut Ball, map: &GameMap, config: &Config, events: &mut Events) {
    let half_height = config.ball_radius;
    let mut pos = ball.pos;
    let mut vel = ball.vel;

    if pos.y - half_height <= 0.0 || pos.y + half_height >= map.height {
        vel.y = -vel.y;
        if pos.y - half_height <= 0.0 {
            pos.y = half_height;
        }
        if pos.y + half_height >= map.height {
            pos.y = map.height - half_height;
        }

        ball.pos = pos;
        ball.vel = vel;
        events.ball_hit_wall = true;
    }
}

fn handle_paddle_collision(
    ball: &mut Ball,
    player_id: PlayerId,
    paddle_y: f32,
    paddle_velocity_y: f32,
    config: &Config,
    events: &mut Events,
) {
    let paddle_x = config.paddle_x(player_id);
    let paddle_half_width = config.paddle_width / 2.0;
    let paddle_half_height = config.paddle_height / 2.0;
    let ball_radius = config.ball_radius;

    let dx = (ball.pos.x - paddle_x).abs();
    let dy = (ball.pos.y - paddle_y).abs();

    if dx < paddle_half_width + ball_radius - config.ball_paddle_overlap
        && dy < paddle_half_height + ball_radius
    {
        let should_bounce = (player_id == PlayerId::LEFT && ball.vel.x < 0.0)
            || (player_id == PlayerId::RIGHT && ball.vel.x > 0.0);

        if should_bounce {
            resolve_paddle_collision(ball, player_id, paddle_y, paddle_velocity_y, config);
            events.ball_hit_paddle = true;
        }
    }
}

fn resolve_paddle_collision(
    ball: &mut Ball,
    player_id: PlayerId,
    paddle_y: f32,
    paddle_velocity_y: f32,
    config: &Config,
) {
    let paddle_half_height = config.paddle_height / 2.0;
    let hit_relative_y = ((ball.pos.y - paddle_y) / paddle_half_height).clamp(-1.0, 1.0);

    let base_speed = ball.vel.length();
    let new_speed = (base_speed * config.ball_speed_increase).min(config.ball_speed_max);

    // Gameplay Scale Factors:
    // 0.785 rad is approx 45 degrees. Hitting the edge of the paddle deflects the ball by up to 45 deg.
    let max_deflection_angle = 0.785;
    let y_deflection = hit_relative_y * max_deflection_angle * new_speed;

    // Paddle Influence:
    // Impart some of the paddle's actual vertical velocity to the ball (friction-like effect).
    // This lets players "slice" the ball or fight against its vertical momentum by moving the
    // paddle on contact. paddle_velocity_y is in [-paddle_speed, paddle_speed].
    let paddle_influence = paddle_velocity_y * 0.3;

    let new_vx = if player_id == PlayerId::LEFT {
        new_speed.abs()
    } else {
        -new_speed.abs()
    };
    let new_vy = y_deflection + paddle_influence;

    let new_vel = glam::Vec2::new(new_vx, new_vy).normalize() * new_speed;
    ball.vel = new_vel;

    // Resolve Overlap:
    // Force the ball to a safe position outside the paddle immediately.
    // This prevents the ball from getting "stuck" inside the paddle on the next frame
    // if it hasn't moved far enough to clear the collision box.
    let paddle_x = config.paddle_x(player_id);
    let paddle_half_width = config.paddle_width / 2.0;
    let overlap = config.ball_paddle_overlap;

    if player_id == PlayerId::LEFT {
        ball.pos.x = paddle_x + paddle_half_width + config.ball_radius - overlap;
    } else {
        ball.pos.x = paddle_x - paddle_half_width - config.ball_radius + overlap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        create_ball, create_paddle, Ball, Config, Events, GameMap, Paddle, PaddleIntent, PlayerId,
    };

    fn setup_world() -> (hecs::World, Config, GameMap, Events) {
        let world = hecs::World::new();
        let config = Config::new();
        let map = GameMap::new();
        let events = Events::new();
        (world, config, map, events)
    }

    #[test]
    fn test_ball_bounces_off_top_wall() {
        let (mut world, config, map, mut events) = setup_world();
        let ball_pos = glam::Vec2::new(16.0, config.ball_radius - 0.1); // Above top wall
        let ball_vel = glam::Vec2::new(8.0, -4.0); // Moving up
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (Y velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y > 0.0,
                "Ball should bounce down after hitting top wall"
            );
            assert_eq!(ball.vel.x, ball_vel.x, "X velocity should be unchanged");
            assert!(
                ball.pos.y >= config.ball_radius,
                "Ball should be pushed out of wall"
            );
        }
        assert!(events.ball_hit_wall, "Should trigger ball_hit_wall event");
    }

    #[test]
    fn test_ball_bounces_off_bottom_wall() {
        let (mut world, config, map, mut events) = setup_world();
        let ball_pos = glam::Vec2::new(16.0, map.height - config.ball_radius + 0.1); // Below bottom wall
        let ball_vel = glam::Vec2::new(8.0, 4.0); // Moving down
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (Y velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y < 0.0,
                "Ball should bounce up after hitting bottom wall"
            );
            assert_eq!(ball.vel.x, ball_vel.x, "X velocity should be unchanged");
            assert!(
                ball.pos.y <= map.height - config.ball_radius,
                "Ball should be pushed out of wall"
            );
        }
        assert!(events.ball_hit_wall, "Should trigger ball_hit_wall event");
    }

    #[test]
    fn test_ball_collides_with_left_paddle() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        // Position ball to hit paddle (inside collision bounds)
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(-8.0, 0.0); // Moving left toward paddle
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (X velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.x > 0.0,
                "Ball should bounce right after hitting left paddle"
            );
            assert!(ball.pos.x > paddle_x, "Ball should be pushed out of paddle");
        }
        assert!(
            events.ball_hit_paddle,
            "Should trigger ball_hit_paddle event"
        );
    }

    #[test]
    fn test_moving_paddle_imparts_spin() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        // Paddle moving downward (+y) at full speed when the ball strikes.
        for (_e, (_p, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            intent.velocity_y = config.paddle_speed;
        }

        // Ball hits the paddle dead-center (so positional deflection is ~0), moving left.
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y,
        );
        create_ball(&mut world, ball_pos, glam::Vec2::new(-8.0, 0.0));

        check_collisions(&mut world, &map, &config, &mut events);

        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(ball.vel.x > 0.0, "Ball should bounce off the paddle");
            assert!(
                ball.vel.y > 0.0,
                "Downward paddle motion should push the ball downward (+y), not stay flat"
            );
        }
    }

    #[test]
    fn test_ball_collides_with_right_paddle() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(1));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(1), paddle_y);

        // Position ball to hit paddle (inside collision bounds)
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x - paddle_half_width + config.ball_radius * 0.5,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(8.0, 0.0); // Moving right toward paddle
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball bounced (X velocity reversed)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.x < 0.0,
                "Ball should bounce left after hitting right paddle"
            );
            assert!(ball.pos.x < paddle_x, "Ball should be pushed out of paddle");
        }
        assert!(
            events.ball_hit_paddle,
            "Should trigger ball_hit_paddle event"
        );
    }

    #[test]
    fn test_ball_speed_increases_on_paddle_hit() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        let initial_speed = 8.0;
        let paddle_half_width = config.paddle_width / 2.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(-initial_speed, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball speed increased
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let new_speed = ball.vel.length();
            let expected_speed =
                (initial_speed * config.ball_speed_increase).min(config.ball_speed_max);
            assert!(
                (new_speed - expected_speed).abs() < 0.01,
                "Ball speed should increase by {}x, got {}",
                config.ball_speed_increase,
                new_speed
            );
        }
    }

    #[test]
    fn test_ball_speed_caps_at_max() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        // Start with speed near max
        let initial_speed = config.ball_speed_max - 1.0;
        let ball_pos = glam::Vec2::new(
            paddle_x + config.paddle_width / 2.0 + config.ball_radius,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(-initial_speed, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball speed doesn't exceed max
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let new_speed = ball.vel.length();
            assert!(
                new_speed <= config.ball_speed_max,
                "Ball speed should not exceed max {}",
                config.ball_speed_max
            );
        }
    }

    #[test]
    fn test_ball_trajectory_affected_by_hit_position() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        // Hit top of paddle (position ball inside collision bounds)
        let paddle_half_width = config.paddle_width / 2.0;
        let paddle_half_height = config.paddle_height / 2.0;
        let ball_pos_top = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y - paddle_half_height + 0.1,
        );
        let ball_vel = glam::Vec2::new(-8.0, 0.0);
        create_ball(&mut world, ball_pos_top, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball deflects upward
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y < 0.0,
                "Ball should deflect upward when hitting top of paddle"
            );
        }

        // Reset and test bottom hit
        world.clear();
        events.clear();
        create_paddle(&mut world, PlayerId(0), paddle_y);

        let ball_pos_bottom = glam::Vec2::new(
            paddle_x + paddle_half_width - config.ball_radius * 0.5,
            paddle_y + paddle_half_height - 0.1,
        );
        create_ball(&mut world, ball_pos_bottom, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball deflects downward
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert!(
                ball.vel.y > 0.0,
                "Ball should deflect downward when hitting bottom of paddle"
            );
        }
    }

    #[test]
    fn test_ball_does_not_bounce_when_moving_away_from_paddle() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        // Ball is at paddle but moving away (right)
        let ball_pos = glam::Vec2::new(
            paddle_x + config.paddle_width / 2.0 + config.ball_radius,
            paddle_y,
        );
        let ball_vel = glam::Vec2::new(8.0, 0.0); // Moving right (away from left paddle)
        create_ball(&mut world, ball_pos, ball_vel);

        check_collisions(&mut world, &map, &config, &mut events);

        // Verify ball didn't bounce
        for (_entity, ball) in world.query::<&Ball>().iter() {
            assert_eq!(
                ball.vel.x, ball_vel.x,
                "Ball should not bounce when moving away"
            );
        }
        assert!(
            !events.ball_hit_paddle,
            "Should not trigger collision when moving away"
        );
    }

    #[test]
    fn test_no_collision_when_no_ball() {
        let (mut world, config, map, mut events) = setup_world();
        create_paddle(&mut world, PlayerId(0), 12.0);

        // Should not panic or error
        check_collisions(&mut world, &map, &config, &mut events);

        assert!(!events.ball_hit_paddle);
        assert!(!events.ball_hit_wall);
    }

    #[test]
    fn test_ball_paddle_overlap() {
        let (mut world, config, map, mut events) = setup_world();
        let paddle_x = config.paddle_x(PlayerId(0));
        let paddle_y = 12.0;
        create_paddle(&mut world, PlayerId(0), paddle_y);

        let paddle_half_width = config.paddle_width / 2.0;
        let ball_radius = config.ball_radius;
        let overlap = config.ball_paddle_overlap;

        // Position ball such that it's just outside the overlap threshold
        let start_x = paddle_x + paddle_half_width + ball_radius - overlap + 0.01;
        let ball_pos = glam::Vec2::new(start_x, paddle_y);
        let ball_vel = glam::Vec2::new(-8.0, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);

        // First check: no collision yet
        check_collisions(&mut world, &map, &config, &mut events);
        assert!(!events.ball_hit_paddle);

        // Move ball slightly inside the threshold
        for (_e, ball) in world.query_mut::<&mut Ball>() {
            ball.pos.x -= 0.02;
        }

        // Second check: collision should trigger
        check_collisions(&mut world, &map, &config, &mut events);
        assert!(events.ball_hit_paddle);

        // Verify push-out position respects overlap
        for (_e, ball) in world.query::<&Ball>().iter() {
            let expected_x = paddle_x + paddle_half_width + ball_radius - overlap;
            assert!(
                (ball.pos.x - expected_x).abs() < 0.001,
                "Ball should be pushed out to the overlap point, got {}, expected {}",
                ball.pos.x,
                expected_x
            );
        }
    }
}
