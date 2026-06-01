pub mod components;
pub mod config;
pub mod map;
pub mod resources;
pub mod simulation;
pub mod systems;

pub use components::*;
pub use config::*;
pub use map::*;
pub use resources::*;
pub use simulation::*;
pub use systems::*;

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// A simulation with a ball at center (moving) and both paddles, ready to step.
    fn setup_game() -> Simulation {
        let mut sim = Simulation::new(12345);
        sim.ball.vel = glam::Vec2::new(sim.config.ball_speed_initial, 0.0);
        let p0 = sim.map.paddle_spawn(PlayerId(0)).y;
        let p1 = sim.map.paddle_spawn(PlayerId(1)).y;
        sim.add_paddle(PlayerId(0), p0);
        sim.add_paddle(PlayerId(1), p1);
        sim
    }

    #[test]
    fn test_full_game_step() {
        let mut sim = setup_game();
        let center = sim.map.ball_spawn();
        sim.step();
        assert!(
            sim.ball.pos.x != center.x || sim.ball.pos.y != center.y,
            "Ball should move after step"
        );
    }

    #[test]
    fn test_game_step_with_paddle_input() {
        let mut sim = setup_game();
        let initial_y = sim.paddle(PlayerId(0)).unwrap().y;

        sim.net_queue.push_input(PlayerId(0), 5.0);
        sim.step();

        let y = sim.paddle(PlayerId(0)).unwrap().y;
        assert!(
            y < initial_y,
            "Paddle should move up toward target after input"
        );
    }

    #[test]
    fn test_ball_bounces_off_wall_during_step() {
        let mut sim = setup_game();
        let ball_radius = sim.config.ball_radius;
        sim.ball.pos = glam::Vec2::new(16.0, ball_radius + 0.1);
        sim.ball.vel = glam::Vec2::new(0.0, -8.0);

        for _ in 0..10 {
            sim.step();
            if sim.events.ball_hit_wall {
                break;
            }
        }

        assert!(sim.events.ball_hit_wall, "Ball should hit wall during step");
    }

    #[test]
    fn test_scoring_during_step() {
        let mut sim = setup_game();
        let width = sim.map.width;
        sim.ball.pos = glam::Vec2::new(width - 0.1, 12.0);
        sim.ball.vel = glam::Vec2::new(8.0, 0.0);

        sim.step();

        assert_eq!(sim.score.left, 1, "Left player should score");
        assert!(sim.events.left_scored, "Should trigger left_scored event");

        let center = sim.map.ball_spawn();
        assert!(
            (sim.ball.pos.x - center.x).abs() < 1.0 && (sim.ball.pos.y - center.y).abs() < 1.0,
            "Ball should reset to center after scoring"
        );
        assert!(
            sim.ball.vel.length_squared() < 0.01,
            "Ball should have zero velocity during respawn delay"
        );
    }

    #[test]
    fn test_win_condition() {
        let mut sim = setup_game();
        let win_score = sim.config.win_score;
        for _ in 0..win_score - 1 {
            sim.score.increment_left();
        }

        let width = sim.map.width;
        sim.ball.pos = glam::Vec2::new(width - 0.1, 12.0);
        sim.ball.vel = glam::Vec2::new(8.0, 0.0);

        sim.step();

        assert_eq!(
            sim.score.left, win_score,
            "Score should reach the win score"
        );
        assert_eq!(
            sim.score.has_winner(win_score),
            Some(PlayerId(0)),
            "Left player should win"
        );
    }

    #[test]
    fn test_multiple_steps_maintain_consistency() {
        let mut sim = setup_game();

        for _ in 0..100 {
            sim.step();

            let width = sim.map.width;
            let height = sim.map.height;
            assert!(
                sim.ball.pos.x > -5.0 && sim.ball.pos.x < width + 5.0,
                "Ball X should be within reasonable bounds"
            );
            assert!(
                sim.ball.pos.y > -5.0 && sim.ball.pos.y < height + 5.0,
                "Ball Y should be within reasonable bounds"
            );
            assert_eq!(sim.paddles.len(), 2, "Both paddles should exist");
        }
    }
}
