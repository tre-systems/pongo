//! The simulation aggregate: the ball and paddles plus the resources that advance them.

use crate::systems::movement::{move_ball, move_paddles};
use crate::{
    check_collisions, check_scoring, ingest_inputs, Ball, Config, Events, GameMap, GameRng,
    NetQueue, Paddle, Params, PlayerId, RespawnState, Score, Time,
};
use glam::Vec2;

/// Everything needed to run the deterministic game simulation: the entities
/// (ball + paddles) and the resources the systems read and write.
///
/// One `Simulation` is embedded by each host (the authoritative server, the
/// offline VS-AI game, and the client), and [`Simulation::step`] is the single
/// entry point that advances it. The game has a fixed, tiny entity set, so the
/// entities are plain fields rather than an ECS world.
pub struct Simulation {
    pub ball: Ball,
    pub paddles: Vec<Paddle>,
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
    /// Create a simulation with a ball at center (no velocity yet) and no paddles.
    /// Hosts add paddles and set the ball's initial velocity to match their start.
    pub fn new(seed: u64) -> Self {
        let map = GameMap::new();
        let ball = Ball::new(map.ball_spawn(), Vec2::ZERO);
        Self {
            ball,
            paddles: Vec::new(),
            time: Time::default(),
            map,
            config: Config::new(),
            score: Score::new(),
            events: Events::new(),
            net_queue: NetQueue::new(),
            rng: GameRng::new(seed),
            respawn_state: RespawnState::new(),
        }
    }

    /// Add a paddle for a player at the given Y.
    pub fn add_paddle(&mut self, player_id: PlayerId, y: f32) {
        self.paddles.push(Paddle::new(player_id, y));
    }

    /// Remove a player's paddle (if present).
    pub fn remove_paddle(&mut self, player_id: PlayerId) {
        self.paddles.retain(|p| p.player_id != player_id);
    }

    /// Find a player's paddle.
    pub fn paddle(&self, player_id: PlayerId) -> Option<&Paddle> {
        self.paddles.iter().find(|p| p.player_id == player_id)
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

        // 1. Ingest inputs (apply to paddle targets).
        ingest_inputs(&mut self.paddles, &mut self.net_queue);

        if !self.respawn_state.can_respawn() {
            // During the respawn delay: hold the ball at center with zero velocity.
            self.ball.pos = self.map.ball_spawn();
            self.ball.vel = Vec2::ZERO;
        } else {
            // Give a freshly-respawned ball its initial velocity.
            if self.ball.vel.length_squared() < 0.01 {
                let initial_speed = self.config.ball_speed_initial;
                self.ball.reset(initial_speed, &mut self.rng);
            }

            // 2. Move ball and paddles.
            move_ball(&mut self.ball, dt);
            move_paddles(&mut self.paddles, &self.config, dt);

            // 3. Check collisions (ball vs paddles, walls).
            check_collisions(
                &mut self.ball,
                &self.paddles,
                &self.map,
                &self.config,
                &mut self.events,
            );

            // 4. Check scoring (ball exited arena).
            check_scoring(
                &mut self.ball,
                &self.map,
                &mut self.score,
                &mut self.events,
                &mut self.respawn_state,
            );
        }

        // Advance simulation time.
        self.time.now += dt;
    }
}
