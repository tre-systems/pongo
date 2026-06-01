use crate::{Ball, Config, Events, GameMap, Paddle, PlayerId};

/// Check ball collisions with walls and paddles
pub fn check_collisions(
    ball: &mut Ball,
    paddles: &[Paddle],
    map: &GameMap,
    config: &Config,
    events: &mut Events,
) {
    handle_wall_collision(ball, map, config, events);

    for paddle in paddles {
        handle_paddle_collision(
            ball,
            paddle.player_id,
            paddle.y,
            paddle.velocity_y,
            config,
            events,
        );
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
    // Force the ball to a safe position outside the paddle immediately, so it can't
    // get "stuck" inside the paddle on the next frame.
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
    use crate::{Ball, Config, Events, GameMap, Paddle, PlayerId};

    fn setup() -> (Config, GameMap, Events) {
        (Config::new(), GameMap::new(), Events::new())
    }

    fn paddle_at(player: PlayerId, y: f32, velocity_y: f32) -> Paddle {
        let mut p = Paddle::new(player, y);
        p.velocity_y = velocity_y;
        p
    }

    #[test]
    fn test_ball_bounces_off_top_wall() {
        let (config, map, mut events) = setup();
        let mut ball = Ball::new(
            glam::Vec2::new(16.0, config.ball_radius - 0.1),
            glam::Vec2::new(8.0, -4.0),
        );

        check_collisions(&mut ball, &[], &map, &config, &mut events);

        assert!(ball.vel.y > 0.0, "Ball should bounce down off the top wall");
        assert_eq!(ball.vel.x, 8.0, "X velocity should be unchanged");
        assert!(ball.pos.y >= config.ball_radius, "Ball pushed out of wall");
        assert!(events.ball_hit_wall);
    }

    #[test]
    fn test_ball_bounces_off_bottom_wall() {
        let (config, map, mut events) = setup();
        let mut ball = Ball::new(
            glam::Vec2::new(16.0, map.height - config.ball_radius + 0.1),
            glam::Vec2::new(8.0, 4.0),
        );

        check_collisions(&mut ball, &[], &map, &config, &mut events);

        assert!(
            ball.vel.y < 0.0,
            "Ball should bounce up off the bottom wall"
        );
        assert_eq!(ball.vel.x, 8.0, "X velocity should be unchanged");
        assert!(ball.pos.y <= map.height - config.ball_radius);
        assert!(events.ball_hit_wall);
    }

    #[test]
    fn test_ball_collides_with_left_paddle() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x + half_w - config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(-8.0, 0.0),
        );

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        assert!(
            ball.vel.x > 0.0,
            "Ball should bounce right off the left paddle"
        );
        assert!(ball.pos.x > paddle_x, "Ball pushed out of paddle");
        assert!(events.ball_hit_paddle);
    }

    #[test]
    fn test_moving_paddle_imparts_spin() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        // Paddle moving downward (+y) at full speed when the ball strikes dead-center.
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, config.paddle_speed)];
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x + half_w - config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(-8.0, 0.0),
        );

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        assert!(ball.vel.x > 0.0, "Ball should bounce off the paddle");
        assert!(
            ball.vel.y > 0.0,
            "Downward paddle motion should push the ball downward (+y), not stay flat"
        );
    }

    #[test]
    fn test_ball_collides_with_right_paddle() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::RIGHT);
        let paddles = vec![paddle_at(PlayerId::RIGHT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x - half_w + config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        assert!(
            ball.vel.x < 0.0,
            "Ball should bounce left off the right paddle"
        );
        assert!(ball.pos.x < paddle_x, "Ball pushed out of paddle");
        assert!(events.ball_hit_paddle);
    }

    #[test]
    fn test_ball_speed_increases_on_paddle_hit() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let initial_speed = 8.0;
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x + half_w - config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(-initial_speed, 0.0),
        );

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        let new_speed = ball.vel.length();
        let expected = (initial_speed * config.ball_speed_increase).min(config.ball_speed_max);
        assert!((new_speed - expected).abs() < 0.01, "speed should increase");
    }

    #[test]
    fn test_ball_speed_caps_at_max() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let initial_speed = config.ball_speed_max - 1.0;
        let mut ball = Ball::new(
            glam::Vec2::new(
                paddle_x + config.paddle_width / 2.0 + config.ball_radius,
                12.0,
            ),
            glam::Vec2::new(-initial_speed, 0.0),
        );

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        assert!(
            ball.vel.length() <= config.ball_speed_max,
            "speed capped at max"
        );
    }

    #[test]
    fn test_ball_trajectory_affected_by_hit_position() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let half_h = config.paddle_height / 2.0;

        // Hit top of paddle -> deflect upward
        let mut ball = Ball::new(
            glam::Vec2::new(
                paddle_x + half_w - config.ball_radius * 0.5,
                12.0 - half_h + 0.1,
            ),
            glam::Vec2::new(-8.0, 0.0),
        );
        check_collisions(&mut ball, &paddles, &map, &config, &mut events);
        assert!(ball.vel.y < 0.0, "top hit deflects upward");

        // Hit bottom of paddle -> deflect downward
        events.clear();
        let mut ball = Ball::new(
            glam::Vec2::new(
                paddle_x + half_w - config.ball_radius * 0.5,
                12.0 + half_h - 0.1,
            ),
            glam::Vec2::new(-8.0, 0.0),
        );
        check_collisions(&mut ball, &paddles, &map, &config, &mut events);
        assert!(ball.vel.y > 0.0, "bottom hit deflects downward");
    }

    #[test]
    fn test_ball_does_not_bounce_when_moving_away_from_paddle() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let mut ball = Ball::new(
            glam::Vec2::new(
                paddle_x + config.paddle_width / 2.0 + config.ball_radius,
                12.0,
            ),
            glam::Vec2::new(8.0, 0.0), // moving away
        );

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        assert_eq!(ball.vel.x, 8.0, "no bounce when moving away");
        assert!(!events.ball_hit_paddle);
    }

    #[test]
    fn test_no_collision_when_ball_in_open_space() {
        let (config, map, mut events) = setup();
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let mut ball = Ball::new(glam::Vec2::new(16.0, 12.0), glam::Vec2::new(8.0, 0.0));

        check_collisions(&mut ball, &paddles, &map, &config, &mut events);

        assert!(!events.ball_hit_paddle);
        assert!(!events.ball_hit_wall);
    }

    #[test]
    fn test_ball_paddle_overlap() {
        let (config, map, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let ball_radius = config.ball_radius;
        let overlap = config.ball_paddle_overlap;

        // Just outside the overlap threshold: no collision.
        let start_x = paddle_x + half_w + ball_radius - overlap + 0.01;
        let mut ball = Ball::new(glam::Vec2::new(start_x, 12.0), glam::Vec2::new(-8.0, 0.0));
        check_collisions(&mut ball, &paddles, &map, &config, &mut events);
        assert!(!events.ball_hit_paddle);

        // Nudge inside the threshold: collision triggers and pushes out to the overlap point.
        ball.pos.x -= 0.02;
        check_collisions(&mut ball, &paddles, &map, &config, &mut events);
        assert!(events.ball_hit_paddle);
        let expected_x = paddle_x + half_w + ball_radius - overlap;
        assert!(
            (ball.pos.x - expected_x).abs() < 0.001,
            "pushed out to overlap point"
        );
    }
}
