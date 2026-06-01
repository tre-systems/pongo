//! Canvas2D renderer. Draws the ball and paddles from the interpolated game
//! state. Canvas2D is universally supported and tiny — no GPU setup needed.

use crate::state::GameState;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

// Arena + entity sizes (units), mirroring game_core's Config.
const ARENA_W: f32 = 32.0;
const ARENA_H: f32 = 24.0;
const PADDLE_W: f32 = 0.8;
const PADDLE_H: f32 = 4.0;
const BALL_R: f32 = 0.5;
const PADDLE_LEFT_X: f32 = 1.5;
const PADDLE_RIGHT_X: f32 = ARENA_W - 1.5;

pub struct Renderer {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
}

impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        let ctx = canvas
            .get_context("2d")
            .map_err(|_| "failed to get 2d context".to_string())?
            .ok_or_else(|| "2d context not available".to_string())?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "object is not a CanvasRenderingContext2d".to_string())?;
        Ok(Self { ctx, width, height })
    }

    pub fn draw(
        &mut self,
        game_state: &GameState,
        local_paddle_y: f32,
        is_local_game: bool,
    ) -> Result<(), String> {
        let sx = self.width / ARENA_W as f64;
        let sy = self.height / ARENA_H as f64;

        // Background.
        self.ctx.set_fill_style_str("#000000");
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);

        // Paddles: in multiplayer our own paddle uses the locally-integrated Y for a
        // zero-latency feel; everything else comes from the interpolated server state.
        let my_player_id = game_state.get_player_id();
        let left_y = if !is_local_game && my_player_id == Some(0) {
            local_paddle_y
        } else {
            game_state.get_paddle_left_y()
        };
        let right_y = if !is_local_game && my_player_id == Some(1) {
            local_paddle_y
        } else {
            game_state.get_paddle_right_y()
        };

        self.ctx.set_fill_style_str("#00ff00");
        self.fill_paddle(PADDLE_LEFT_X, left_y, sx, sy);
        self.fill_paddle(PADDLE_RIGHT_X, right_y, sx, sy);

        // Ball.
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.begin_path();
        self.ctx
            .arc(
                game_state.get_ball_x() as f64 * sx,
                game_state.get_ball_y() as f64 * sy,
                BALL_R as f64 * sx,
                0.0,
                std::f64::consts::PI * 2.0,
            )
            .map_err(|_| "arc failed".to_string())?;
        self.ctx.fill();

        Ok(())
    }

    fn fill_paddle(&self, center_x: f32, center_y: f32, sx: f64, sy: f64) {
        let w = PADDLE_W as f64 * sx;
        let h = PADDLE_H as f64 * sy;
        self.ctx.fill_rect(
            center_x as f64 * sx - w / 2.0,
            center_y as f64 * sy - h / 2.0,
            w,
            h,
        );
    }
}
