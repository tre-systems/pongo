//! Network protocol for Pong game
//!
//! Uses postcard for efficient binary serialization

use postcard::{from_bytes, to_allocvec};

// ============================================================================
// Shared Structures
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameStateSnapshot {
    pub tick: u32,
    pub ball_x: f32,
    pub ball_y: f32,
    pub ball_vx: f32,
    pub ball_vy: f32,
    pub paddle_left_y: f32,
    pub paddle_right_y: f32,
    pub score_left: u8,
    pub score_right: u8,
}

// ============================================================================
// C2S Messages (Client to Server)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum C2S {
    /// Join a match with code
    Join { code: [u8; 5] },

    /// Paddle input: absolute Y position
    /// seq: Client-side sequence number
    Input { player_id: u8, y: f32, seq: u32 },

    /// Ping for latency measurement
    Ping { t_ms: u32 },

    /// Request to restart the match (valid only in GameOver state)
    Restart,
}

// ============================================================================
// S2C Messages (Server to Client)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum S2C {
    /// Welcome message with player assignment
    Welcome {
        player_id: u8, // 0 = left, 1 = right
    },

    /// Opponent has connected, match is ready
    MatchFound,

    /// Synchronized countdown tick (3, 2, 1)
    Countdown { seconds: u8 },

    /// Game is starting now - begin playing
    GameStart,

    /// Game state snapshot (only sent during PLAYING)
    GameState(GameStateSnapshot),

    /// Game over message
    GameOver {
        winner: u8, // 0 = left, 1 = right
    },

    /// Opponent disconnected
    OpponentDisconnected,

    /// Pong response to ping
    Pong { t_ms: u32 },

    /// Opponent dropped; the match is paused awaiting their reconnect.
    /// (New variants are appended to keep postcard variant indices stable.)
    OpponentReconnecting,

    /// Opponent reconnected; the match is about to resume.
    OpponentReconnected,
}

// ============================================================================
// Serialization Helpers
// ============================================================================

impl C2S {
    /// Serialize C2S message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        to_allocvec(self)
    }

    /// Deserialize C2S message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        from_bytes(bytes)
    }
}

impl S2C {
    /// Serialize S2C message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        to_allocvec(self)
    }

    /// Deserialize S2C message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c2s_serialization() {
        let msg = C2S::Input {
            player_id: 0,
            y: 10.0,
            seq: 1,
        };
        let bytes = msg.to_bytes().expect("Serialization should succeed");
        let decoded = C2S::from_bytes(&bytes).expect("Deserialization should succeed");
        match (msg, decoded) {
            (
                C2S::Input {
                    player_id: p1,
                    y: y1,
                    seq: s1,
                },
                C2S::Input {
                    player_id: p2,
                    y: y2,
                    seq: s2,
                },
            ) => {
                assert_eq!(p1, p2);
                assert!((y1 - y2).abs() < f32::EPSILON);
                assert_eq!(s1, s2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_s2c_serialization() {
        let msg = S2C::GameState(GameStateSnapshot {
            tick: 100,
            ball_x: 16.0,
            ball_y: 12.0,
            ball_vx: 8.0,
            ball_vy: 4.0,
            paddle_left_y: 12.0,
            paddle_right_y: 12.0,
            score_left: 5,
            score_right: 3,
        });
        let bytes = msg.to_bytes().expect("Serialization should succeed");
        let decoded = S2C::from_bytes(&bytes).expect("Deserialization should succeed");
        match decoded {
            S2C::GameState(snapshot) => {
                assert_eq!(snapshot.tick, 100);
                assert_eq!(snapshot.ball_x, 16.0);
            }
            _ => panic!("Message type mismatch"),
        }
    }
}
