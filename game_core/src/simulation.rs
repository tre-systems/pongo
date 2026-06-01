//! The simulation aggregate: the ball and paddles plus the resources that advance them.

use crate::systems::movement::{move_ball, move_paddles};
use crate::{
    check_collisions, check_scoring, ingest_inputs, Ball, Config, Events, GameRng, NetQueue,
    Paddle, Params, PlayerId, RespawnState, Score,
};
use glam::Vec2;

/// Everything needed to run the deterministic game simulation: the entities
/// (ball + paddles) and the resources the systems read and write.
///
/// One `Simulation` is embedded by each host that runs the game (the authoritative
/// server and the offline VS-AI game), and [`Simulation::step`] is the single entry
/// point that advances it. The game has a fixed, tiny entity set, so the entities are
/// plain fields rather than an ECS world.
pub struct Simulation {
    pub ball: Ball,
    pub paddles: Vec<Paddle>,
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
        let config = Config::new();
        let ball = Ball::new(config.ball_spawn(), Vec2::ZERO);
        Self {
            ball,
            paddles: Vec::new(),
            config,
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

        // Clear per-frame events at the start of the tick.
        self.events.clear();

        // Update respawn timer.
        self.respawn_state.update(dt);

        // Ingest inputs (apply to paddle targets).
        ingest_inputs(&mut self.paddles, &mut self.net_queue, &self.config);

        if !self.respawn_state.can_respawn() {
            // During the respawn delay: hold the ball at center with zero velocity.
            self.ball.pos = self.config.ball_spawn();
            self.ball.vel = Vec2::ZERO;
        } else {
            // A freshly-respawned ball is effectively stopped; serve it a fresh velocity.
            if self.ball.vel.length_squared() < 0.01 {
                let center = self.config.ball_spawn();
                let initial_speed = self.config.ball_speed_initial;
                self.ball.reset(center, initial_speed, &mut self.rng);
            }

            move_ball(&mut self.ball, dt);
            move_paddles(&mut self.paddles, &self.config, dt);
            check_collisions(
                &mut self.ball,
                &self.paddles,
                &self.config,
                &mut self.events,
            );
            check_scoring(
                &mut self.ball,
                &self.config,
                &mut self.score,
                &mut self.events,
                &mut self.respawn_state,
            );
        }
    }
}
