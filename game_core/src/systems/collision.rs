use crate::{Ball, Config, Events, Paddle, PlayerId};

/// Check ball collisions with walls and paddles
pub fn check_collisions(ball: &mut Ball, paddles: &[Paddle], config: &Config, events: &mut Events) {
    handle_wall_collision(ball, config, events);

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

fn handle_wall_collision(ball: &mut Ball, config: &Config, events: &mut Events) {
    let r = config.ball_radius;
    let height = config.arena_height;

    // The arena is far taller than the ball, so only one wall can be hit per tick.
    if ball.pos.y - r <= 0.0 {
        ball.vel.y = -ball.vel.y;
        ball.pos.y = r;
        events.ball_hit_wall = true;
    } else if ball.pos.y + r >= height {
        ball.vel.y = -ball.vel.y;
        ball.pos.y = height - r;
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

    // Hitting the paddle edge deflects the ball by up to ~45 deg (0.785 rad). The
    // *new_speed factor keeps this hit-position term scaled relative to the
    // fixed-scale paddle slice (paddle_influence) before the vector is re-normalized.
    let deflection_gain = 0.785;
    let y_deflection = hit_relative_y * deflection_gain * new_speed;

    // Impart some of the paddle's vertical velocity to the ball (friction-like "slice"),
    // letting players steer with paddle motion. paddle_velocity_y is in [-paddle_speed, paddle_speed].
    let paddle_influence = paddle_velocity_y * 0.3;

    // new_speed is non-negative, so the sign alone sets the outgoing direction.
    let new_vx = if player_id == PlayerId::LEFT {
        new_speed
    } else {
        -new_speed
    };
    let new_vy = y_deflection + paddle_influence;

    let new_vel = glam::Vec2::new(new_vx, new_vy).normalize() * new_speed;
    ball.vel = new_vel;

    // Push the ball just outside the paddle so it can't get "stuck" inside next frame.
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
    use crate::{Ball, Config, Events, Paddle, PlayerId};

    fn setup() -> (Config, Events) {
        (Config::new(), Events::new())
    }

    fn paddle_at(player: PlayerId, y: f32, velocity_y: f32) -> Paddle {
        let mut p = Paddle::new(player, y);
        p.velocity_y = velocity_y;
        p
    }

    #[test]
    fn test_ball_bounces_off_top_wall() {
        let (config, mut events) = setup();
        let mut ball = Ball::new(
            glam::Vec2::new(16.0, config.ball_radius - 0.1),
            glam::Vec2::new(8.0, -4.0),
        );

        check_collisions(&mut ball, &[], &config, &mut events);

        assert!(ball.vel.y > 0.0, "Ball should bounce down off the top wall");
        assert_eq!(ball.vel.x, 8.0, "X velocity should be unchanged");
        assert!(ball.pos.y >= config.ball_radius, "Ball pushed out of wall");
        assert!(events.ball_hit_wall);
    }

    #[test]
    fn test_ball_bounces_off_bottom_wall() {
        let (config, mut events) = setup();
        let mut ball = Ball::new(
            glam::Vec2::new(16.0, config.arena_height - config.ball_radius + 0.1),
            glam::Vec2::new(8.0, 4.0),
        );

        check_collisions(&mut ball, &[], &config, &mut events);

        assert!(
            ball.vel.y < 0.0,
            "Ball should bounce up off the bottom wall"
        );
        assert_eq!(ball.vel.x, 8.0, "X velocity should be unchanged");
        assert!(ball.pos.y <= config.arena_height - config.ball_radius);
        assert!(events.ball_hit_wall);
    }

    #[test]
    fn test_ball_collides_with_left_paddle() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x + half_w - config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(-8.0, 0.0),
        );

        check_collisions(&mut ball, &paddles, &config, &mut events);

        assert!(
            ball.vel.x > 0.0,
            "Ball should bounce right off the left paddle"
        );
        assert!(ball.pos.x > paddle_x, "Ball pushed out of paddle");
        assert!(events.ball_hit_paddle);
    }

    #[test]
    fn test_moving_paddle_imparts_spin() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        // Paddle moving downward (+y) at full speed when the ball strikes dead-center.
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, config.paddle_speed)];
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x + half_w - config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(-8.0, 0.0),
        );

        check_collisions(&mut ball, &paddles, &config, &mut events);

        assert!(ball.vel.x > 0.0, "Ball should bounce off the paddle");
        assert!(
            ball.vel.y > 0.0,
            "Downward paddle motion should push the ball downward (+y), not stay flat"
        );
    }

    #[test]
    fn test_ball_collides_with_right_paddle() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::RIGHT);
        let paddles = vec![paddle_at(PlayerId::RIGHT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x - half_w + config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );

        check_collisions(&mut ball, &paddles, &config, &mut events);

        assert!(
            ball.vel.x < 0.0,
            "Ball should bounce left off the right paddle"
        );
        assert!(ball.pos.x < paddle_x, "Ball pushed out of paddle");
        assert!(events.ball_hit_paddle);
    }

    #[test]
    fn test_ball_speed_increases_on_paddle_hit() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let initial_speed = 8.0;
        let half_w = config.paddle_width / 2.0;
        let mut ball = Ball::new(
            glam::Vec2::new(paddle_x + half_w - config.ball_radius * 0.5, 12.0),
            glam::Vec2::new(-initial_speed, 0.0),
        );

        check_collisions(&mut ball, &paddles, &config, &mut events);

        let new_speed = ball.vel.length();
        let expected = (initial_speed * config.ball_speed_increase).min(config.ball_speed_max);
        assert!((new_speed - expected).abs() < 0.01, "speed should increase");
    }

    #[test]
    fn test_ball_speed_caps_at_max() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let initial_speed = config.ball_speed_max - 1.0;
        let mut ball = Ball::new(
            glam::Vec2::new(
                paddle_x + config.paddle_width / 2.0 - config.ball_radius * 0.5,
                12.0,
            ),
            glam::Vec2::new(-initial_speed, 0.0),
        );

        check_collisions(&mut ball, &paddles, &config, &mut events);

        assert!(
            ball.vel.length() <= config.ball_speed_max,
            "speed capped at max"
        );
    }

    #[test]
    fn test_ball_trajectory_affected_by_hit_position() {
        let (config, mut events) = setup();
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
        check_collisions(&mut ball, &paddles, &config, &mut events);
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
        check_collisions(&mut ball, &paddles, &config, &mut events);
        assert!(ball.vel.y > 0.0, "bottom hit deflects downward");
    }

    #[test]
    fn test_ball_does_not_bounce_when_moving_away_from_paddle() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let mut ball = Ball::new(
            glam::Vec2::new(
                paddle_x + config.paddle_width / 2.0 - config.ball_radius * 0.5,
                12.0,
            ),
            glam::Vec2::new(8.0, 0.0), // moving away
        );

        check_collisions(&mut ball, &paddles, &config, &mut events);

        assert_eq!(ball.vel.x, 8.0, "no bounce when moving away");
        assert!(!events.ball_hit_paddle);
    }

    #[test]
    fn test_no_collision_when_ball_in_open_space() {
        let (config, mut events) = setup();
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let mut ball = Ball::new(glam::Vec2::new(16.0, 12.0), glam::Vec2::new(8.0, 0.0));

        check_collisions(&mut ball, &paddles, &config, &mut events);

        assert!(!events.ball_hit_paddle);
        assert!(!events.ball_hit_wall);
    }

    #[test]
    fn test_ball_paddle_overlap() {
        let (config, mut events) = setup();
        let paddle_x = config.paddle_x(PlayerId::LEFT);
        let paddles = vec![paddle_at(PlayerId::LEFT, 12.0, 0.0)];
        let half_w = config.paddle_width / 2.0;
        let ball_radius = config.ball_radius;
        let overlap = config.ball_paddle_overlap;

        // Just outside the overlap threshold: no collision.
        let start_x = paddle_x + half_w + ball_radius - overlap + 0.01;
        let mut ball = Ball::new(glam::Vec2::new(start_x, 12.0), glam::Vec2::new(-8.0, 0.0));
        check_collisions(&mut ball, &paddles, &config, &mut events);
        assert!(!events.ball_hit_paddle);

        // Nudge inside the threshold: collision triggers and pushes out to the overlap point.
        ball.pos.x -= 0.02;
        check_collisions(&mut ball, &paddles, &config, &mut events);
        assert!(events.ball_hit_paddle);
        let expected_x = paddle_x + half_w + ball_radius - overlap;
        assert!(
            (ball.pos.x - expected_x).abs() < 0.001,
            "pushed out to overlap point"
        );
    }
}
