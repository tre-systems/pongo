pub mod components;
pub mod config;
pub mod map;
pub mod resources;
pub mod systems;

pub use components::*;
pub use config::*;
pub use map::*;
pub use resources::*;
pub use systems::*;

use hecs::World;

/// Run the deterministic Pong game simulation
#[allow(clippy::too_many_arguments)]
pub fn step(
    world: &mut World,
    time: &mut Time,
    map: &GameMap,
    config: &Config,
    score: &mut Score,
    events: &mut Events,
    net_queue: &mut NetQueue,
    rng: &mut GameRng,
    respawn_state: &mut RespawnState,
) {
    // Clamp dt to prevent large jumps
    let clamped_dt = time.dt.min(Params::MAX_DT);

    // Clear events at start of frame
    events.clear();

    // Fixed micro-steps for stable physics
    let mut remaining_dt = clamped_dt;
    while remaining_dt > 0.0 {
        let step_dt = remaining_dt.min(Params::FIXED_DT);
        remaining_dt -= step_dt;

        // Update respawn timer
        respawn_state.update(step_dt);

        // 1. Ingest inputs (apply to paddle intents)
        ingest_inputs(world, net_queue);

        // 2. Handle ball respawn after delay
        if !respawn_state.can_respawn() {
            // During respawn delay: keep ball at center with zero velocity
            for (_entity, ball) in world.query_mut::<&mut Ball>() {
                let center = map.ball_spawn();
                ball.pos = center;
                ball.vel = glam::Vec2::ZERO;
            }
        } else {
            // Ball can move - check if it just finished respawn delay and needs velocity
            for (_entity, ball) in world.query_mut::<&mut Ball>() {
                // If ball has zero velocity but timer just expired, give it initial velocity
                if ball.vel.length_squared() < 0.01 {
                    ball.reset(config.ball_speed_initial, rng);
                }
            }

            // 3. Move ball
            move_ball(world, step_dt);

            systems::movement::move_paddles(world, map, config, step_dt);

            // 4. Check collisions (ball vs paddles, walls)
            check_collisions(world, map, config, events);

            // 5. Check scoring (ball exited arena)
            check_scoring(world, map, score, events, rng, config, respawn_state);
        }
    }

    // Update time
    time.now += clamped_dt;
}

/// Helper to create a paddle entity
pub fn create_paddle(world: &mut World, player_id: u8, y: f32) -> hecs::Entity {
    world.spawn((Paddle::new(player_id, y), PaddleIntent::with_target(y)))
}

/// Helper to create the ball entity
pub fn create_ball(world: &mut World, pos: glam::Vec2, vel: glam::Vec2) -> hecs::Entity {
    world.spawn((Ball::new(pos, vel),))
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn setup_game() -> (
        World,
        Time,
        GameMap,
        Config,
        Score,
        Events,
        NetQueue,
        GameRng,
        RespawnState,
    ) {
        let mut world = World::new();
        let map = GameMap::new();
        let config = Config::new();
        let time = Time::new(0.020, 0.0);
        let score = Score::new();
        let events = Events::new();
        let net_queue = NetQueue::new();
        let rng = GameRng::new(12345);
        let respawn_state = RespawnState::new();

        // Create initial game state
        let ball_pos = map.ball_spawn();
        let ball_vel = glam::Vec2::new(config.ball_speed_initial, 0.0);
        create_ball(&mut world, ball_pos, ball_vel);
        create_paddle(&mut world, 0, map.paddle_spawn(0).y);
        create_paddle(&mut world, 1, map.paddle_spawn(1).y);

        (
            world,
            time,
            map,
            config,
            score,
            events,
            net_queue,
            rng,
            respawn_state,
        )
    }

    #[test]
    fn test_full_game_step() {
        let (
            mut world,
            mut time,
            map,
            config,
            mut score,
            mut events,
            mut net_queue,
            mut rng,
            mut respawn_state,
        ) = setup_game();

        // Run one step
        step(
            &mut world,
            &mut time,
            &map,
            &config,
            &mut score,
            &mut events,
            &mut net_queue,
            &mut rng,
            &mut respawn_state,
        );

        // Verify ball moved
        let mut ball_found = false;
        for (_entity, ball) in world.query::<&Ball>().iter() {
            ball_found = true;
            // Ball should have moved from center
            assert!(
                ball.pos.x != map.ball_spawn().x || ball.pos.y != map.ball_spawn().y,
                "Ball should move after step"
            );
        }
        assert!(ball_found, "Ball should exist in world");
    }

    #[test]
    fn test_game_step_with_paddle_input() {
        let (
            mut world,
            mut time,
            map,
            config,
            mut score,
            mut events,
            mut net_queue,
            mut rng,
            mut respawn_state,
        ) = setup_game();

        // Get initial paddle position
        let mut initial_paddle_y = 0.0;
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            if paddle.player_id == 0 {
                initial_paddle_y = paddle.y;
            }
        }

        // Queue input to move paddle to 5.0
        net_queue.push_input(0, 5.0);

        // Run step
        step(
            &mut world,
            &mut time,
            &map,
            &config,
            &mut score,
            &mut events,
            &mut net_queue,
            &mut rng,
            &mut respawn_state,
        );

        // Verify paddle moved towards target
        for (_entity, paddle) in world.query::<&Paddle>().iter() {
            if paddle.player_id == 0 {
                // Should have moved UP (smaller Y) towards 5.0
                assert!(
                    paddle.y < initial_paddle_y,
                    "Paddle should move up after input"
                );
                // Should not overshoot target if speed allows
                // (Depends on speed/dt, but 12.0 -> 5.0 is far, so it should just be closer)
            }
        }
    }

    #[test]
    fn test_ball_bounces_off_wall_during_step() {
        let (
            mut world,
            mut time,
            map,
            config,
            mut score,
            mut events,
            mut net_queue,
            mut rng,
            mut respawn_state,
        ) = setup_game();

        // Position ball near top wall
        for (_entity, ball) in world.query_mut::<&mut Ball>() {
            ball.pos = glam::Vec2::new(16.0, config.ball_radius + 0.1);
            ball.vel = glam::Vec2::new(0.0, -8.0); // Moving up
        }

        // Run multiple steps until collision
        for _ in 0..10 {
            step(
                &mut world,
                &mut time,
                &map,
                &config,
                &mut score,
                &mut events,
                &mut net_queue,
                &mut rng,
                &mut respawn_state,
            );
            if events.ball_hit_wall {
                break;
            }
            events.clear();
        }

        // Verify wall collision occurred
        assert!(events.ball_hit_wall, "Ball should hit wall during step");
    }

    #[test]
    fn test_scoring_during_step() {
        let (
            mut world,
            mut time,
            map,
            config,
            mut score,
            mut events,
            mut net_queue,
            mut rng,
            mut respawn_state,
        ) = setup_game();

        // Position ball to exit right edge (must be beyond width after movement)
        for (_entity, ball) in world.query_mut::<&mut Ball>() {
            ball.pos = glam::Vec2::new(map.width - 0.1, 12.0);
            ball.vel = glam::Vec2::new(8.0, 0.0); // Moving right
        }

        // Run step (ball will move and exit)
        step(
            &mut world,
            &mut time,
            &map,
            &config,
            &mut score,
            &mut events,
            &mut net_queue,
            &mut rng,
            &mut respawn_state,
        );

        // Verify scoring occurred
        assert_eq!(score.left, 1, "Left player should score");
        assert!(events.left_scored, "Should trigger left_scored event");

        // Verify ball was reset to center (but may be in respawn delay)
        for (_entity, ball) in world.query::<&Ball>().iter() {
            let center = map.ball_spawn();
            assert!(
                (ball.pos.x - center.x).abs() < 1.0 && (ball.pos.y - center.y).abs() < 1.0,
                "Ball should reset to center after scoring"
            );
            // Ball should have zero velocity during respawn delay
            assert!(
                ball.vel.length_squared() < 0.01,
                "Ball should have zero velocity during respawn delay"
            );
        }
    }

    #[test]
    fn test_win_condition() {
        let (
            mut world,
            mut time,
            map,
            config,
            mut score,
            mut events,
            mut net_queue,
            mut rng,
            mut respawn_state,
        ) = setup_game();

        // Set score to one point from winning
        let target = config.win_score - 1;
        for _ in 0..target {
            score.increment_left();
        }
        assert_eq!(
            score.left, target,
            "Score should be {target} before final point"
        );

        // Position ball to score (must exit right edge)
        for (_entity, ball) in world.query_mut::<&mut Ball>() {
            ball.pos = glam::Vec2::new(map.width - 0.1, 12.0);
            ball.vel = glam::Vec2::new(8.0, 0.0);
        }

        // Run step
        step(
            &mut world,
            &mut time,
            &map,
            &config,
            &mut score,
            &mut events,
            &mut net_queue,
            &mut rng,
            &mut respawn_state,
        );

        // Verify win condition
        assert_eq!(
            score.left, config.win_score,
            "Score should be {} after final point",
            config.win_score
        );
        assert_eq!(
            score.has_winner(config.win_score),
            Some(0),
            "Left player should win"
        );
    }

    #[test]
    fn test_multiple_steps_maintain_consistency() {
        let (
            mut world,
            mut time,
            map,
            config,
            mut score,
            mut events,
            mut net_queue,
            mut rng,
            mut respawn_state,
        ) = setup_game();

        // Run 100 steps
        for _ in 0..100 {
            step(
                &mut world,
                &mut time,
                &map,
                &config,
                &mut score,
                &mut events,
                &mut net_queue,
                &mut rng,
                &mut respawn_state,
            );
            events.clear();

            // Verify ball exists and is within reasonable bounds
            let mut ball_found = false;
            for (_entity, ball) in world.query::<&Ball>().iter() {
                ball_found = true;
                // Ball should be within reasonable bounds (allow some margin for scoring)
                assert!(
                    ball.pos.x > -5.0 && ball.pos.x < map.width + 5.0,
                    "Ball X should be within reasonable bounds"
                );
                assert!(
                    ball.pos.y > -5.0 && ball.pos.y < map.height + 5.0,
                    "Ball Y should be within reasonable bounds"
                );
            }
            assert!(ball_found, "Ball should always exist");

            // Verify paddles exist
            let paddle_count: usize = world.query::<&Paddle>().iter().count();
            assert_eq!(paddle_count, 2, "Both paddles should exist");
        }
    }
}
