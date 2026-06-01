use crate::game_state::{Environment, GameClient, GameState, MatchState};
use game_core::PlayerId;
use proto::S2C;
use std::cell::RefCell;
use worker::*;

struct MockGameClient {
    pub sent_messages: RefCell<Vec<Vec<u8>>>,
}

impl MockGameClient {
    fn new() -> Self {
        Self {
            sent_messages: RefCell::new(Vec::new()),
        }
    }

    // Kept to avoid unused warnings/tests
    #[allow(dead_code)]
    fn last_message(&self) -> Option<Vec<u8>> {
        self.sent_messages.borrow().last().cloned()
    }

    #[allow(dead_code)]
    fn count(&self) -> usize {
        self.sent_messages.borrow().len()
    }

    #[allow(dead_code)]
    fn clear(&self) {
        self.sent_messages.borrow_mut().clear();
    }
}

impl GameClient for MockGameClient {
    fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
        self.sent_messages.borrow_mut().push(bytes.to_vec());
        Ok(())
    }
}

struct MockEnv {
    time_ms: u64,
}

impl MockEnv {
    fn new() -> Self {
        Self { time_ms: 1000 }
    }
}

impl Environment for MockEnv {
    fn now(&self) -> u64 {
        self.time_ms
    }
    fn log(&self, _msg: String) {
        // No-op for tests or println!(_msg)
    }
}

#[test]
fn test_game_initialization() {
    let gs = GameState::new(Box::new(MockEnv::new()));
    assert_eq!(gs.clients.len(), 0);
    assert_eq!(gs.next_player_id, 0);
    assert_eq!(gs.match_state, MatchState::Waiting);
}

#[test]
fn test_add_player_limit() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));

    // Add player 0
    let res0 = gs.add_player(Box::new(MockGameClient::new()));
    assert!(res0.is_some());
    let (pid0, empty0) = res0.unwrap();
    assert_eq!(pid0, 0);
    assert!(empty0); // Was empty

    // Add player 1
    let res1 = gs.add_player(Box::new(MockGameClient::new()));
    assert!(res1.is_some());
    let (pid1, empty1) = res1.unwrap();
    assert_eq!(pid1, 1);
    assert!(!empty1); // Was not empty

    // Add player 2 (should fail)
    let res2 = gs.add_player(Box::new(MockGameClient::new()));
    assert!(res2.is_none());
}

#[test]
fn test_game_start_condition() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));

    gs.add_player(Box::new(MockGameClient::new()));
    assert_eq!(gs.match_state, MatchState::Waiting);

    gs.add_player(Box::new(MockGameClient::new()));
    // With two players, match should transition to Countdown (not Playing directly)
    assert_eq!(gs.match_state, MatchState::Countdown);
}

#[test]
fn test_player_removal() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));

    gs.add_player(Box::new(MockGameClient::new()));
    gs.remove_player(0);

    assert_eq!(gs.clients.len(), 0);
}

#[test]
fn test_handle_input() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    let client0 = Box::new(MockGameClient::new());
    gs.add_player(client0);

    // Send input for player 0
    gs.handle_input(0, 1.0); // Move down

    // Check if input queue has it
    let inputs = gs.sim.net_queue.pop_inputs();
    assert!(!inputs.is_empty());
    assert_eq!(inputs[0].0, PlayerId(0));
    assert_eq!(inputs[0].1, 1.0);
}

#[test]
fn test_disconnect_midmatch_pauses_for_reconnect() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    gs.match_state = MatchState::Playing; // pretend the countdown finished

    gs.handle_disconnect(0);

    assert_eq!(gs.match_state, MatchState::Paused);
    assert_eq!(gs.clients.len(), 2, "the slot is held open during grace");
    assert!(!gs.clients.get(&0).unwrap().connected);
    assert!(gs.clients.get(&1).unwrap().connected);
    assert!(gs.reconnect_deadline_ms > 0);
}

#[test]
fn test_reconnect_resumes_match() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    gs.match_state = MatchState::Playing;
    gs.handle_disconnect(0);
    assert_eq!(gs.match_state, MatchState::Paused);

    let pid = gs.reconnect_player(Box::new(MockGameClient::new()));
    assert_eq!(pid, Some(0));
    assert_eq!(gs.match_state, MatchState::Countdown);
    assert_eq!(gs.clients.len(), 2);
    assert!(gs.clients.get(&0).unwrap().connected);
    assert_eq!(gs.reconnect_deadline_ms, 0);
}

#[test]
fn test_grace_expiry_forfeits_to_remaining_player() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    gs.match_state = MatchState::Playing;
    gs.handle_disconnect(0);
    assert_eq!(gs.match_state, MatchState::Paused);

    // Grace-expiry path (what the alarm does): remove the dropped player.
    gs.remove_player(0);
    assert_eq!(gs.match_state, MatchState::GameOver);
    assert_eq!(gs.clients.len(), 1);
    assert!(gs.clients.contains_key(&1));
}

#[test]
fn test_disconnect_while_waiting_removes_immediately() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));
    gs.add_player(Box::new(MockGameClient::new()));
    // Only one player and still Waiting: a disconnect removes rather than pausing.
    gs.handle_disconnect(0);
    assert_eq!(gs.match_state, MatchState::Waiting);
    assert_eq!(gs.clients.len(), 0);
}

#[test]
fn test_broadcast_state() {
    let mut gs = GameState::new(Box::new(MockEnv::new()));

    let messages = std::rc::Rc::new(RefCell::new(Vec::new()));

    struct SharedMock {
        msgs: std::rc::Rc<RefCell<Vec<Vec<u8>>>>,
    }

    impl GameClient for SharedMock {
        fn send_bytes(&self, bytes: &[u8]) -> Result<()> {
            self.msgs.borrow_mut().push(bytes.to_vec());
            Ok(())
        }
    }

    let client = Box::new(SharedMock {
        msgs: messages.clone(),
    });

    gs.add_player(client);

    gs.broadcast_state();

    assert_eq!(messages.borrow().len(), 1);

    // Verify it's a GameState message
    let bytes = &messages.borrow()[0];
    let msg = S2C::from_bytes(bytes).unwrap();
    match msg {
        S2C::GameState { .. } => (),
        _ => panic!("Expected GameState message"),
    }
}
