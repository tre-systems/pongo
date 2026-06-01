use crate::PlayerId;

/// Time resource for tracking simulation time
#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub dt: f32,  // Delta time for this step
    pub now: f32, // Total elapsed time
}

impl Time {
    pub fn new(dt: f32, now: f32) -> Self {
        Self { dt, now }
    }
}

impl Default for Time {
    fn default() -> Self {
        Self {
            dt: crate::Params::FIXED_DT,
            now: 0.0,
        }
    }
}

/// Game score tracking
#[derive(Debug, Clone, Copy, Default)]
pub struct Score {
    pub left: u8,  // Left player score
    pub right: u8, // Right player score
}

impl Score {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_left(&mut self) {
        self.left += 1;
    }

    pub fn increment_right(&mut self) {
        self.right += 1;
    }

    pub fn has_winner(&self, win_score: u8) -> Option<PlayerId> {
        if self.left >= win_score {
            Some(PlayerId::LEFT)
        } else if self.right >= win_score {
            Some(PlayerId::RIGHT)
        } else {
            None
        }
    }
}

/// Random number generator
pub struct GameRng(pub rand::rngs::StdRng);

impl GameRng {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        Self(rand::rngs::StdRng::seed_from_u64(seed))
    }
}

impl Default for GameRng {
    fn default() -> Self {
        Self::new(12345)
    }
}

/// Events that occurred during this frame
#[derive(Debug, Clone, Default)]
pub struct Events {
    pub left_scored: bool,
    pub right_scored: bool,
    pub ball_hit_paddle: bool,
    pub ball_hit_wall: bool,
}

/// Respawn state for managing ball respawn delays after scoring
#[derive(Debug, Clone, Copy, Default)]
pub struct RespawnState {
    pub timer: f32, // Time remaining before ball respawns (0 = ready to respawn)
}

impl RespawnState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_delay(&mut self, delay: f32) {
        self.timer = delay;
    }

    pub fn update(&mut self, dt: f32) {
        if self.timer > 0.0 {
            self.timer = (self.timer - dt).max(0.0);
        }
    }

    pub fn can_respawn(&self) -> bool {
        self.timer <= 0.0
    }
}

impl Events {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.left_scored = false;
        self.right_scored = false;
        self.ball_hit_paddle = false;
        self.ball_hit_wall = false;
    }
}

/// Network input queue (placeholder for network inputs)
#[derive(Debug, Clone, Default)]
pub struct NetQueue {
    pub inputs: Vec<(PlayerId, f32)>, // (player_id, y_absolute)
}

impl NetQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.inputs.clear();
    }

    pub fn push_input(&mut self, player_id: PlayerId, y: f32) {
        self.inputs.push((player_id, y));
    }

    pub fn pop_inputs(&mut self) -> Vec<(PlayerId, f32)> {
        let inputs = self.inputs.clone();
        self.inputs.clear();
        inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_increment_left() {
        let mut score = Score::new();
        assert_eq!(score.left, 0);
        score.increment_left();
        assert_eq!(score.left, 1);
        score.increment_left();
        assert_eq!(score.left, 2);
    }

    #[test]
    fn test_score_increment_right() {
        let mut score = Score::new();
        assert_eq!(score.right, 0);
        score.increment_right();
        assert_eq!(score.right, 1);
        score.increment_right();
        assert_eq!(score.right, 2);
    }

    #[test]
    fn test_score_has_winner_left() {
        let mut score = Score::new();
        for _ in 0..11 {
            score.increment_left();
        }
        assert_eq!(
            score.has_winner(11),
            Some(PlayerId(0)),
            "Left player should win at 11"
        );
    }

    #[test]
    fn test_score_has_winner_right() {
        let mut score = Score::new();
        for _ in 0..11 {
            score.increment_right();
        }
        assert_eq!(
            score.has_winner(11),
            Some(PlayerId(1)),
            "Right player should win at 11"
        );
    }

    #[test]
    fn test_score_no_winner_below_threshold() {
        let mut score = Score::new();
        for _ in 0..10 {
            score.increment_left();
        }
        assert_eq!(score.has_winner(11), None, "No winner below threshold");
    }

    #[test]
    fn test_events_clear() {
        let mut events = Events::new();
        events.left_scored = true;
        events.right_scored = true;
        events.ball_hit_paddle = true;
        events.ball_hit_wall = true;

        events.clear();

        assert!(!events.left_scored);
        assert!(!events.right_scored);
        assert!(!events.ball_hit_paddle);
        assert!(!events.ball_hit_wall);
    }

    #[test]
    fn test_net_queue_push_input() {
        let mut queue = NetQueue::new();
        queue.push_input(PlayerId(0), 10.0);
        queue.push_input(PlayerId(1), 14.0);

        assert_eq!(queue.inputs.len(), 2);
        assert_eq!(queue.inputs[0], (PlayerId(0), 10.0));
        assert_eq!(queue.inputs[1], (PlayerId(1), 14.0));
    }

    #[test]
    fn test_net_queue_clear() {
        let mut queue = NetQueue::new();
        queue.push_input(PlayerId(0), 10.0);
        queue.push_input(PlayerId(1), 14.0);

        queue.clear();
        assert_eq!(queue.inputs.len(), 0);
    }
}
