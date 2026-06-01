use game_core::{create_ball, create_paddle, Ball, Config, Paddle, Params, PlayerId, Simulation};
use hecs::World;

/// An offline VS-AI game. Wraps one [`Simulation`] and drives the right paddle
/// with a simple AI; the left paddle follows the local player's input.
pub struct LocalGame {
    pub sim: Simulation,
}

impl LocalGame {
    pub fn new(seed: u64) -> Self {
        let mut sim = Simulation::new(seed);

        // Create paddles
        let left_paddle_y = sim.map.paddle_spawn(PlayerId(0)).y;
        let right_paddle_y = sim.map.paddle_spawn(PlayerId(1)).y;
        create_paddle(&mut sim.world, PlayerId(0), left_paddle_y);
        create_paddle(&mut sim.world, PlayerId(1), right_paddle_y);

        // Create ball with a random initial direction
        let mut ball = Ball::new(glam::f32::Vec2::ZERO, glam::f32::Vec2::ZERO);
        ball.reset(sim.config.ball_speed_initial, &mut sim.rng);
        create_ball(&mut sim.world, ball.pos, ball.vel);

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
        // AI: Control right paddle (player_id=1)
        let ai_dir = calculate_ai_input(&self.sim.world, &self.sim.config);

        // Find the current AI paddle Y, then integrate it one fixed step.
        let mut ai_y = 12.0;
        for (_e, paddle) in self.sim.world.query::<&Paddle>().iter() {
            if paddle.player_id == PlayerId(1) {
                ai_y = paddle.y;
                break;
            }
        }
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

        // Extract data needed for visual updates
        let ball_data = self
            .sim
            .world
            .query::<&Ball>()
            .iter()
            .next()
            .map(|(_e, ball)| (ball.pos, ball.vel));

        let mut paddle_left_y = 12.0;
        let mut paddle_right_y = 12.0;
        for (_e, paddle) in self.sim.world.query::<&Paddle>().iter() {
            if paddle.player_id == PlayerId(0) {
                paddle_left_y = paddle.y;
            } else if paddle.player_id == PlayerId(1) {
                paddle_right_y = paddle.y;
            }
        }

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

/// Calculate AI input for opponent paddle
///
/// Strategy:
/// 1. Simple heuristic: if ball is moving towards us, predict intersection y.
/// 2. If intersection is significantly different from current y, move there.
/// 3. If ball moving away, return to center to cover maximum area.
fn calculate_ai_input(world: &World, config: &Config) -> i8 {
    let ball_data = world
        .query::<&Ball>()
        .iter()
        .next()
        .map(|(_e, ball)| (ball.pos, ball.vel));
    let paddle_data = world
        .query::<&Paddle>()
        .iter()
        .find(|(_e, p)| p.player_id == PlayerId(1))
        .map(|(_e, p)| p.y);

    if let (Some((ball_pos, ball_vel)), Some(paddle_y)) = (ball_data, paddle_data) {
        if ball_vel.x > 0.0 {
            let paddle_x = config.paddle_x(PlayerId(1));
            let time_to_reach = (paddle_x - ball_pos.x) / ball_vel.x.max(0.1);
            let predicted_y = ball_pos.y + ball_vel.y * time_to_reach;

            let target_y = predicted_y + (ball_vel.y * 0.3);
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
            let center_y = 12.0;
            let diff = center_y - paddle_y;
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
    } else {
        0
    }
}
