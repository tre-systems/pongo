//! Network message handling

use crate::state::{GameState, MatchEvent};
use proto::{C2S, S2C};

/// Handle incoming server message
pub fn handle_message(msg: S2C, game_state: &mut GameState) -> Result<(), String> {
    match msg {
        S2C::Welcome { player_id } => {
            game_state.set_player_id(player_id);
        }
        S2C::MatchFound => {
            game_state.reset();
            game_state.match_event = MatchEvent::MatchFound;
        }
        S2C::Countdown { seconds } => {
            game_state.reset();
            game_state.match_event = MatchEvent::Countdown(seconds);
        }
        S2C::GameStart => {
            game_state.reset();
            game_state.match_event = MatchEvent::GameStart;
        }
        S2C::GameState(snapshot) => {
            game_state.set_scores(snapshot.score_left, snapshot.score_right);
            game_state.set_current(snapshot);
        }
        S2C::GameOver { winner } => {
            game_state.set_winner(winner);
        }
        S2C::OpponentDisconnected => {
            game_state.match_event = MatchEvent::OpponentDisconnected;
        }
        S2C::OpponentReconnecting => {
            game_state.match_event = MatchEvent::OpponentReconnecting;
        }
        S2C::OpponentReconnected => {
            game_state.match_event = MatchEvent::OpponentReconnected;
        }
        S2C::Pong { t_ms: _ } => {
            // Ping response handled by caller, should not reach here
            return Err("Pong message should be handled separately".to_string());
        }
    }
    Ok(())
}

/// Build a Join message. The match code must be exactly 5 characters.
pub fn create_join_message(code: &str) -> Result<Vec<u8>, String> {
    let code_bytes: Vec<u8> = code.bytes().take(5).collect();
    if code_bytes.len() != 5 {
        return Err("Match code must be exactly 5 characters".to_string());
    }
    let mut code_array = [0u8; 5];
    code_array.copy_from_slice(&code_bytes);
    C2S::Join { code: code_array }
        .to_bytes()
        .map_err(|e| format!("Failed to serialize join message: {:?}", e))
}

pub fn create_input_message(player_id: u8, y: f32, seq: u32) -> Result<Vec<u8>, String> {
    C2S::Input { player_id, y, seq }
        .to_bytes()
        .map_err(|e| format!("Failed to serialize input message: {:?}", e))
}

pub fn create_restart_message() -> Result<Vec<u8>, String> {
    C2S::Restart
        .to_bytes()
        .map_err(|e| format!("Failed to serialize restart message: {:?}", e))
}

pub fn create_ping_message(t_ms: u32) -> Result<Vec<u8>, String> {
    C2S::Ping { t_ms }
        .to_bytes()
        .map_err(|e| format!("Failed to serialize ping message: {:?}", e))
}
