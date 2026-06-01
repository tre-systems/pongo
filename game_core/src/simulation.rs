//! The simulation aggregate: the ECS `World` plus the resources that advance it.

use crate::systems::movement::move_paddles;
use crate::{
    check_collisions, check_scoring, ingest_inputs, move_ball, Ball, Config, Events, GameMap,
    GameRng, NetQueue, Params, RespawnState, Score, Time,
};
use hecs::World;

/// Everything needed to run the deterministic game simulation: the ECS `World`
/// and the resources the systems read and write.
///
/// One `Simulation` is embedded by each host (the authoritative server, the
/// offline VS-AI game, and the client predictor) instead of nine loose fields,
/// and [`Simulation::step`] is the single entry point that advances it.
pub struct Simulation {
    pub world: World,
    pub time: Time,
    pub map: GameMap,
    pub config: Config,
    pub score: Score,
    pub events: Events,
    pub net_queue: NetQueue,
    pub rng: GameRng,
    pub respawn_state: RespawnState,
}

impl Simulation {
    /// Create a simulation with no entities yet. Hosts populate the `world` with
    /// the ball and paddles to match their starting conditions.
    pub fn new(seed: u64) -> Self {
        Self {
            world: World::new(),
            time: Time::default(),
            map: GameMap::new(),
            config: Config::new(),
            score: Score::new(),
            events: Events::new(),
            net_queue: NetQueue::new(),
            rng: GameRng::new(seed),
            respawn_state: RespawnState::new(),
        }
    }

    /// Advance the simulation by exactly one fixed timestep ([`Params::FIXED_DT`]).
    /// Hosts call this once per tick and drive the cadence from their own
    /// real-time accumulator.
    pub fn step(&mut self) {
        let dt = Params::FIXED_DT;
        self.time.dt = dt;

        // Clear per-frame events at the start of the tick.
        self.events.clear();

        // Update respawn timer.
        self.respawn_state.update(dt);

        // 1. Ingest inputs (apply to paddle intents).
        ingest_inputs(&mut self.world, &mut self.net_queue);

        // 2. Handle ball respawn after a score.
        if !self.respawn_state.can_respawn() {
            // During the respawn delay: keep the ball at center with zero velocity.
            let center = self.map.ball_spawn();
            for (_entity, ball) in self.world.query_mut::<&mut Ball>() {
                ball.pos = center;
                ball.vel = glam::Vec2::ZERO;
            }
        } else {
            // Give a freshly-respawned ball its initial velocity.
            let initial_speed = self.config.ball_speed_initial;
            for (_entity, ball) in self.world.query_mut::<&mut Ball>() {
                if ball.vel.length_squared() < 0.01 {
                    ball.reset(initial_speed, &mut self.rng);
                }
            }

            // 3. Move ball and paddles.
            move_ball(&mut self.world, dt);
            move_paddles(&mut self.world, &self.map, &self.config, dt);

            // 4. Check collisions (ball vs paddles, walls).
            check_collisions(&mut self.world, &self.map, &self.config, &mut self.events);

            // 5. Check scoring (ball exited arena).
            check_scoring(
                &mut self.world,
                &self.map,
                &mut self.score,
                &mut self.events,
                &mut self.rng,
                &self.config,
                &mut self.respawn_state,
            );
        }

        // Advance simulation time.
        self.time.now += dt;
    }
}
