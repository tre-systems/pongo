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

use hecs::World;

/// Helper to create a paddle entity
pub fn create_paddle(world: &mut World, player_id: PlayerId, y: f32) -> hecs::Entity {
    world.spawn((Paddle::new(player_id, y), PaddleIntent::with_target(y)))
}

/// Helper to create the ball entity
pub fn create_ball(world: &mut World, pos: glam::Vec2, vel: glam::Vec2) -> hecs::Entity {
    world.spawn((Ball::new(pos, vel),))
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// A simulation with a ball at center and both paddles, ready to step.
    fn setup_game() -> Simulation {
        let mut sim = Simulation::new(12345);
        let ball_pos = sim.map.ball_spawn();
        let ball_vel = glam::Vec2::new(sim.config.ball_speed_initial, 0.0);
        let p0 = sim.map.paddle_spawn(PlayerId(0)).y;
        let p1 = sim.map.paddle_spawn(PlayerId(1)).y;
        create_ball(&mut sim.world, ball_pos, ball_vel);
        create_paddle(&mut sim.world, PlayerId(0), p0);
        create_paddle(&mut sim.world, PlayerId(1), p1);
        sim
    }

    #[test]
    fn test_full_game_step() {
        let mut sim = setup_game();
        sim.step();

        // Verify ball moved
        let center = sim.map.ball_spawn();
        let mut ball_found = false;
        for (_entity, ball) in sim.world.query::<&Ball>().iter() {
            ball_found = true;
            assert!(
                ball.pos.x != center.x || ball.pos.y != center.y,
                "Ball should move after step"
            );
        }
        assert!(ball_found, "Ball should exist in world");
    }

    #[test]
    fn test_game_step_with_paddle_input() {
        let mut sim = setup_game();

        // Get initial paddle position
        let mut initial_paddle_y = 0.0;
        for (_entity, paddle) in sim.world.query::<&Paddle>().iter() {
            if paddle.player_id == PlayerId(0) {
                initial_paddle_y = paddle.y;
            }
        }

        // Queue input to move paddle to 5.0, then step
        sim.net_queue.push_input(PlayerId(0), 5.0);
        sim.step();

        // Verify paddle moved towards target (UP = smaller Y)
        for (_entity, paddle) in sim.world.query::<&Paddle>().iter() {
            if paddle.player_id == PlayerId(0) {
                assert!(
                    paddle.y < initial_paddle_y,
                    "Paddle should move up after input"
                );
            }
        }
    }

    #[test]
    fn test_ball_bounces_off_wall_during_step() {
        let mut sim = setup_game();

        // Position ball near top wall, moving up
        let ball_radius = sim.config.ball_radius;
        for (_entity, ball) in sim.world.query_mut::<&mut Ball>() {
            ball.pos = glam::Vec2::new(16.0, ball_radius + 0.1);
            ball.vel = glam::Vec2::new(0.0, -8.0);
        }

        // Run steps until a wall collision is observed
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

        // Position ball to exit the right edge
        let width = sim.map.width;
        for (_entity, ball) in sim.world.query_mut::<&mut Ball>() {
            ball.pos = glam::Vec2::new(width - 0.1, 12.0);
            ball.vel = glam::Vec2::new(8.0, 0.0);
        }

        sim.step();

        assert_eq!(sim.score.left, 1, "Left player should score");
        assert!(sim.events.left_scored, "Should trigger left_scored event");

        // Ball should reset to center and be held (zero velocity) during respawn delay
        let center = sim.map.ball_spawn();
        for (_entity, ball) in sim.world.query::<&Ball>().iter() {
            assert!(
                (ball.pos.x - center.x).abs() < 1.0 && (ball.pos.y - center.y).abs() < 1.0,
                "Ball should reset to center after scoring"
            );
            assert!(
                ball.vel.length_squared() < 0.01,
                "Ball should have zero velocity during respawn delay"
            );
        }
    }

    #[test]
    fn test_win_condition() {
        let mut sim = setup_game();
        let win_score = sim.config.win_score;

        // One point from winning
        for _ in 0..win_score - 1 {
            sim.score.increment_left();
        }

        // Position ball to score the final point
        let width = sim.map.width;
        for (_entity, ball) in sim.world.query_mut::<&mut Ball>() {
            ball.pos = glam::Vec2::new(width - 0.1, 12.0);
            ball.vel = glam::Vec2::new(8.0, 0.0);
        }

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

            let mut ball_found = false;
            for (_entity, ball) in sim.world.query::<&Ball>().iter() {
                ball_found = true;
                assert!(
                    ball.pos.x > -5.0 && ball.pos.x < width + 5.0,
                    "Ball X should be within reasonable bounds"
                );
                assert!(
                    ball.pos.y > -5.0 && ball.pos.y < height + 5.0,
                    "Ball Y should be within reasonable bounds"
                );
            }
            assert!(ball_found, "Ball should always exist");

            let paddle_count: usize = sim.world.query::<&Paddle>().iter().count();
            assert_eq!(paddle_count, 2, "Both paddles should exist");
        }
    }
}
