use game_core::{Ball, Config, Paddle, Params, PlayerId, Simulation};

/// An offline VS-AI game. Wraps one [`Simulation`] and drives the right paddle
/// with a simple AI; the left paddle follows the local player's input.
pub struct LocalGame {
    pub sim: Simulation,
}

impl LocalGame {
    pub fn new(seed: u64) -> Self {
        let mut sim = Simulation::new(seed);

        // Both paddles spawn at the arena's mid-height.
        let center_y = sim.config.paddle_spawn_y();
        sim.add_paddle(PlayerId(0), center_y);
        sim.add_paddle(PlayerId(1), center_y);

        // Serve the ball in a random direction.
        let center = sim.config.ball_spawn();
        let initial_speed = sim.config.ball_speed_initial;
        sim.ball.reset(center, initial_speed, &mut sim.rng);

        Self { sim }
    }

    pub fn step(
        &mut self,
        my_paddle_y: f32,
    ) -> (
        Option<u8>,
        Option<(glam::Vec2, glam::Vec2)>,
        f32,
        f32,
        u8,
        u8,
    ) {
        // AI controls the right paddle (player 1).
        let ai_dir = calculate_ai_input(&self.sim.ball, &self.sim.paddles, &self.sim.config);
        let ai_y = self.sim.paddle(PlayerId(1)).map(|p| p.y).unwrap_or(12.0);
        let mut new_ai_y = ai_y + (ai_dir as f32) * self.sim.config.paddle_speed * Params::FIXED_DT;
        let half_height = self.sim.config.paddle_height / 2.0;
        new_ai_y = new_ai_y.clamp(half_height, self.sim.config.arena_height - half_height);

        // Both paddles are driven as absolute-target inputs through the queue.
        self.sim.net_queue.push_input(PlayerId(0), my_paddle_y);
        self.sim.net_queue.push_input(PlayerId(1), new_ai_y);

        self.sim.step();

        let winner = self
            .sim
            .score
            .has_winner(self.sim.config.win_score)
            .map(|p| p.0);
        let ball_data = Some((self.sim.ball.pos, self.sim.ball.vel));
        let paddle_left_y = self.sim.paddle(PlayerId(0)).map(|p| p.y).unwrap_or(12.0);
        let paddle_right_y = self.sim.paddle(PlayerId(1)).map(|p| p.y).unwrap_or(12.0);

        (
            winner,
            ball_data,
            paddle_left_y,
            paddle_right_y,
            self.sim.score.left,
            self.sim.score.right,
        )
    }
}

/// Calculate AI input for the opponent paddle.
///
/// Strategy:
/// 1. If the ball is moving towards us, predict the intersection Y and move there.
/// 2. If the ball is moving away, return to center to cover maximum area.
fn calculate_ai_input(ball: &Ball, paddles: &[Paddle], config: &Config) -> i8 {
    let paddle_y = match paddles.iter().find(|p| p.player_id == PlayerId(1)) {
        Some(p) => p.y,
        None => return 0,
    };

    if ball.vel.x > 0.0 {
        let paddle_x = config.paddle_x(PlayerId(1));
        let time_to_reach = (paddle_x - ball.pos.x) / ball.vel.x.max(0.1);
        let predicted_y = ball.pos.y + ball.vel.y * time_to_reach;

        let target_y = predicted_y + (ball.vel.y * 0.3);
        let diff = target_y - paddle_y;
        let deadzone = 0.3;

        if diff > deadzone {
            1
        } else if diff < -deadzone {
            -1
        } else {
            0
        }
    } else {
        let diff = 12.0 - paddle_y;
        if diff.abs() > 0.5 {
            if diff > 0.0 {
                1
            } else {
                -1
            }
        } else {
            0
        }
    }
}
