use crate::{Ball, Events, GameMap, RespawnState, Score};

/// Respawn delay after scoring (in seconds)
const RESPAWN_DELAY: f32 = 1.5;

/// Check if the ball left the arena (scoring). On a score, the ball is reset to
/// center with zero velocity and a respawn delay is started; `Simulation::step`
/// gives it a fresh velocity once the delay elapses.
pub fn check_scoring(
    ball: &mut Ball,
    map: &GameMap,
    score: &mut Score,
    events: &mut Events,
    respawn_state: &mut RespawnState,
) {
    if ball.pos.x < 0.0 {
        // Right player scores
        score.increment_right();
        events.right_scored = true;
        ball.pos = map.ball_spawn();
        ball.vel = glam::Vec2::ZERO;
        respawn_state.start_delay(RESPAWN_DELAY);
    } else if ball.pos.x > map.width {
        // Left player scores
        score.increment_left();
        events.left_scored = true;
        ball.pos = map.ball_spawn();
        ball.vel = glam::Vec2::ZERO;
        respawn_state.start_delay(RESPAWN_DELAY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ball, Events, GameMap, Score};

    fn setup() -> (GameMap, Score, Events, RespawnState) {
        (
            GameMap::new(),
            Score::new(),
            Events::new(),
            RespawnState::new(),
        )
    }

    #[test]
    fn test_right_player_scores_when_ball_exits_left() {
        let (map, mut score, mut events, mut respawn) = setup();
        let mut ball = Ball::new(glam::Vec2::new(-0.1, 12.0), glam::Vec2::new(-8.0, 0.0));

        check_scoring(&mut ball, &map, &mut score, &mut events, &mut respawn);

        assert_eq!(score.right, 1, "Right player should score");
        assert_eq!(score.left, 0);
        assert!(events.right_scored);
    }

    #[test]
    fn test_left_player_scores_when_ball_exits_right() {
        let (map, mut score, mut events, mut respawn) = setup();
        let mut ball = Ball::new(
            glam::Vec2::new(map.width + 0.1, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );

        check_scoring(&mut ball, &map, &mut score, &mut events, &mut respawn);

        assert_eq!(score.left, 1, "Left player should score");
        assert_eq!(score.right, 0);
        assert!(events.left_scored);
    }

    #[test]
    fn test_ball_resets_after_scoring() {
        let (map, mut score, mut events, mut respawn) = setup();
        let mut ball = Ball::new(glam::Vec2::new(-0.1, 12.0), glam::Vec2::new(-8.0, 0.0));

        check_scoring(&mut ball, &map, &mut score, &mut events, &mut respawn);

        let center = map.ball_spawn();
        assert!((ball.pos.x - center.x).abs() < 0.1 && (ball.pos.y - center.y).abs() < 0.1);
        assert!(
            ball.vel.length_squared() < 0.01,
            "zero velocity during respawn delay"
        );
        assert!(respawn.timer > 0.0, "respawn delay should be active");
    }

    #[test]
    fn test_no_scoring_when_ball_in_bounds() {
        let (map, mut score, mut events, mut respawn) = setup();
        let mut ball = Ball::new(glam::Vec2::new(16.0, 12.0), glam::Vec2::new(8.0, 4.0));

        check_scoring(&mut ball, &map, &mut score, &mut events, &mut respawn);

        assert_eq!(score.left, 0);
        assert_eq!(score.right, 0);
        assert!(!events.left_scored && !events.right_scored);
    }

    #[test]
    fn test_multiple_scores_accumulate() {
        let (map, mut score, mut events, mut respawn) = setup();

        let mut ball = Ball::new(
            glam::Vec2::new(map.width + 0.1, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );
        check_scoring(&mut ball, &map, &mut score, &mut events, &mut respawn);
        events.clear();

        let mut ball = Ball::new(
            glam::Vec2::new(map.width + 0.1, 12.0),
            glam::Vec2::new(8.0, 0.0),
        );
        check_scoring(&mut ball, &map, &mut score, &mut events, &mut respawn);

        assert_eq!(score.left, 2, "Scores should accumulate");
        assert_eq!(score.right, 0);
    }
}
