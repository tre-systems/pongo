//! Game State Machine
//!
//! Manages game state transitions for both local and multiplayer modes.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsmState {
    Idle,
    CountdownLocal,
    PlayingLocal,
    Paused,
    Connecting,
    Waiting,
    CountdownMulti,
    PlayingMulti,
    GameOverLocal,
    GameOverMulti,
    Disconnected,
    Reconnecting,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    StartLocal,
    CreateMatch,
    JoinMatch,
    CountdownDone,
    Quit,
    GameOver,
    Connected,
    ConnectionFailed,
    OpponentJoined,
    Disconnected,
    Leave,
    PlayAgain,
    RematchStarted,
    Pause,
    Resume,
    ConnectionLost,
    Reconnected,
    ReconnectFailed,
}

/// Result of a state transition
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct TransitionResult {
    success: bool,
    from_state: FsmState,
    to_state: FsmState,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl TransitionResult {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn success(&self) -> bool {
        self.success
    }

    // Named from_state to match the JS-read property; clippy would otherwise
    // flag the from_* getter as constructor-style.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    #[allow(clippy::wrong_self_convention)]
    pub fn from_state(&self) -> FsmState {
        self.from_state
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn to_state(&self) -> FsmState {
        self.to_state
    }
}

/// Game Finite State Machine
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GameFsm {
    state: FsmState,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GameFsm {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self {
            state: FsmState::Idle,
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn state(&self) -> FsmState {
        self.state
    }

    /// Current state as string (for JS interop)
    pub fn state_string(&self) -> String {
        format!("{:?}", self.state)
    }

    /// Attempt a transition
    pub fn transition(&mut self, action: GameAction) -> TransitionResult {
        let from_state = self.state;

        if let Some(next_state) = self.get_next_state(action) {
            self.state = next_state;
            TransitionResult {
                success: true,
                from_state,
                to_state: next_state,
            }
        } else {
            TransitionResult {
                success: false,
                from_state,
                to_state: from_state,
            }
        }
    }

    /// Transition using action string (for easier JS interop)
    pub fn transition_str(&mut self, action: &str) -> TransitionResult {
        let action = match action {
            "START_LOCAL" => GameAction::StartLocal,
            "CREATE_MATCH" => GameAction::CreateMatch,
            "JOIN_MATCH" => GameAction::JoinMatch,
            "COUNTDOWN_DONE" => GameAction::CountdownDone,
            "QUIT" => GameAction::Quit,
            "GAME_OVER" => GameAction::GameOver,
            "CONNECTED" => GameAction::Connected,
            "CONNECTION_FAILED" => GameAction::ConnectionFailed,
            "OPPONENT_JOINED" => GameAction::OpponentJoined,
            "DISCONNECTED" => GameAction::Disconnected,
            "LEAVE" => GameAction::Leave,
            "PLAY_AGAIN" => GameAction::PlayAgain,
            "REMATCH_STARTED" => GameAction::RematchStarted,
            "PAUSE" => GameAction::Pause,
            "RESUME" => GameAction::Resume,
            "CONNECTION_LOST" => GameAction::ConnectionLost,
            "RECONNECTED" => GameAction::Reconnected,
            "RECONNECT_FAILED" => GameAction::ReconnectFailed,
            // An unrecognized action string is a caller bug; report it as a
            // failed transition so JS's `if (!result.success)` branch fires.
            _ => {
                return TransitionResult {
                    success: false,
                    from_state: self.state,
                    to_state: self.state,
                };
            }
        };
        self.transition(action)
    }

    /// Next state for a given action, or None if the transition is invalid.
    fn get_next_state(&self, action: GameAction) -> Option<FsmState> {
        match (self.state, action) {
            // From Idle
            (FsmState::Idle, GameAction::StartLocal) => Some(FsmState::CountdownLocal),
            (FsmState::Idle, GameAction::CreateMatch) => Some(FsmState::Connecting),
            (FsmState::Idle, GameAction::JoinMatch) => Some(FsmState::Connecting),

            // From CountdownLocal
            (FsmState::CountdownLocal, GameAction::CountdownDone) => Some(FsmState::PlayingLocal),
            (FsmState::CountdownLocal, GameAction::Quit) => Some(FsmState::Idle),

            // From PlayingLocal
            (FsmState::PlayingLocal, GameAction::GameOver) => Some(FsmState::GameOverLocal),
            (FsmState::PlayingLocal, GameAction::Quit) => Some(FsmState::Idle),
            (FsmState::PlayingLocal, GameAction::Pause) => Some(FsmState::Paused),

            // From Paused (local games only)
            (FsmState::Paused, GameAction::Resume) => Some(FsmState::PlayingLocal),
            (FsmState::Paused, GameAction::Quit) => Some(FsmState::Idle),

            // From Connecting
            (FsmState::Connecting, GameAction::Connected) => Some(FsmState::Waiting),
            (FsmState::Connecting, GameAction::ConnectionFailed) => Some(FsmState::Idle),

            // From Waiting
            (FsmState::Waiting, GameAction::OpponentJoined) => Some(FsmState::CountdownMulti),
            (FsmState::Waiting, GameAction::Disconnected) => Some(FsmState::Idle),
            (FsmState::Waiting, GameAction::Leave) => Some(FsmState::Idle),

            // From CountdownMulti
            (FsmState::CountdownMulti, GameAction::CountdownDone) => Some(FsmState::PlayingMulti),
            (FsmState::CountdownMulti, GameAction::Disconnected) => Some(FsmState::Disconnected),
            (FsmState::CountdownMulti, GameAction::ConnectionLost) => Some(FsmState::Reconnecting),

            // From PlayingMulti
            (FsmState::PlayingMulti, GameAction::GameOver) => Some(FsmState::GameOverMulti),
            (FsmState::PlayingMulti, GameAction::Disconnected) => Some(FsmState::Disconnected),
            (FsmState::PlayingMulti, GameAction::ConnectionLost) => Some(FsmState::Reconnecting),

            // From Reconnecting (a dropped multiplayer client trying to return)
            (FsmState::Reconnecting, GameAction::Reconnected) => Some(FsmState::CountdownMulti),
            (FsmState::Reconnecting, GameAction::ReconnectFailed) => Some(FsmState::Disconnected),
            (FsmState::Reconnecting, GameAction::Disconnected) => Some(FsmState::Disconnected),
            (FsmState::Reconnecting, GameAction::Leave) => Some(FsmState::Idle),

            // From GameOverLocal
            (FsmState::GameOverLocal, GameAction::PlayAgain) => Some(FsmState::CountdownLocal),
            (FsmState::GameOverLocal, GameAction::Leave) => Some(FsmState::Idle),

            // From GameOverMulti
            (FsmState::GameOverMulti, GameAction::RematchStarted) => Some(FsmState::CountdownMulti),
            (FsmState::GameOverMulti, GameAction::Disconnected) => Some(FsmState::Disconnected),
            (FsmState::GameOverMulti, GameAction::Leave) => Some(FsmState::Idle),

            // From Disconnected
            (FsmState::Disconnected, GameAction::Leave) => Some(FsmState::Idle),

            // Invalid transition
            _ => None,
        }
    }
}

impl Default for GameFsm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let fsm = GameFsm::new();
        assert_eq!(fsm.state(), FsmState::Idle);
    }

    #[test]
    fn test_valid_transition() {
        let mut fsm = GameFsm::new();
        let result = fsm.transition(GameAction::StartLocal);
        assert!(result.success);
        assert_eq!(fsm.state(), FsmState::CountdownLocal);
    }

    #[test]
    fn test_invalid_transition() {
        let mut fsm = GameFsm::new();
        let result = fsm.transition(GameAction::GameOver);
        assert!(!result.success);
        assert_eq!(fsm.state(), FsmState::Idle);
    }

    #[test]
    fn test_local_game_flow() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::StartLocal);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingLocal);
        fsm.transition(GameAction::GameOver);
        assert_eq!(fsm.state(), FsmState::GameOverLocal);
        fsm.transition(GameAction::PlayAgain);
        assert_eq!(fsm.state(), FsmState::CountdownLocal);
    }

    #[test]
    fn test_pause_resume_local() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::StartLocal);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingLocal);

        assert!(fsm.transition(GameAction::Pause).success);
        assert_eq!(fsm.state(), FsmState::Paused);

        assert!(fsm.transition(GameAction::Resume).success);
        assert_eq!(fsm.state(), FsmState::PlayingLocal);
    }

    #[test]
    fn test_quit_from_paused() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::StartLocal);
        fsm.transition(GameAction::CountdownDone);
        fsm.transition(GameAction::Pause);
        assert!(fsm.transition(GameAction::Quit).success);
        assert_eq!(fsm.state(), FsmState::Idle);
    }

    #[test]
    fn test_multiplayer_cannot_pause() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::CreateMatch);
        fsm.transition(GameAction::Connected);
        fsm.transition(GameAction::OpponentJoined);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingMulti);
        // Pause is local-only; it must not apply during multiplayer.
        assert!(!fsm.transition(GameAction::Pause).success);
        assert_eq!(fsm.state(), FsmState::PlayingMulti);
    }

    #[test]
    fn test_multiplayer_flow() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::CreateMatch);
        assert_eq!(fsm.state(), FsmState::Connecting);
        fsm.transition(GameAction::Connected);
        assert_eq!(fsm.state(), FsmState::Waiting);
        fsm.transition(GameAction::OpponentJoined);
        assert_eq!(fsm.state(), FsmState::CountdownMulti);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingMulti);
    }

    #[test]
    fn test_reconnect_flow() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::CreateMatch);
        fsm.transition(GameAction::Connected);
        fsm.transition(GameAction::OpponentJoined);
        fsm.transition(GameAction::CountdownDone);
        assert_eq!(fsm.state(), FsmState::PlayingMulti);

        // Connection drops mid-match -> Reconnecting (not straight to Disconnected).
        assert!(fsm.transition(GameAction::ConnectionLost).success);
        assert_eq!(fsm.state(), FsmState::Reconnecting);

        // Successful reconnect resumes via the ready-countdown.
        assert!(fsm.transition(GameAction::Reconnected).success);
        assert_eq!(fsm.state(), FsmState::CountdownMulti);
    }

    #[test]
    fn test_reconnect_gives_up() {
        let mut fsm = GameFsm::new();
        fsm.transition(GameAction::CreateMatch);
        fsm.transition(GameAction::Connected);
        fsm.transition(GameAction::OpponentJoined);
        fsm.transition(GameAction::CountdownDone);
        fsm.transition(GameAction::ConnectionLost);
        assert_eq!(fsm.state(), FsmState::Reconnecting);

        assert!(fsm.transition(GameAction::ReconnectFailed).success);
        assert_eq!(fsm.state(), FsmState::Disconnected);
    }

    #[test]
    fn test_transition_str() {
        let mut fsm = GameFsm::new();
        let result = fsm.transition_str("START_LOCAL");
        assert!(result.success);
        assert_eq!(fsm.state(), FsmState::CountdownLocal);
    }
}
