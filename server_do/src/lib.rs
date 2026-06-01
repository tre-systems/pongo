#![allow(unknown_lints)]
#![allow(clippy::manual_is_multiple_of)]
use proto::*;
use std::cell::RefCell;
use std::time::Duration;
use worker::*;

mod game_state;
use game_state::{GameState, MatchState, WasmEnv};

#[cfg(test)]
mod tests;

#[durable_object]
pub struct MatchDO {
    state: State,
    #[allow(dead_code)]
    env: Env,
    game_state: RefCell<GameState>,
}

impl DurableObject for MatchDO {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            game_state: RefCell::new(GameState::new(Box::new(WasmEnv))),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        console_log!("DO: Received request, method: {:?}", req.method());
        if let Ok(url) = req.url() {
            console_log!("DO: Request URL: {}", url);
        }

        // Check for WebSocket upgrade
        let upgrade_header = req.headers().get("Upgrade");
        console_log!("DO: Upgrade header result: {:?}", upgrade_header);

        match upgrade_header {
            Ok(Some(header)) if header.to_lowercase() == "websocket" => {
                console_log!("DO: Received WebSocket upgrade request");

                let pair = match WebSocketPair::new() {
                    Ok(pair) => pair,
                    Err(err) => {
                        console_error!("DO: Failed to create WebSocket pair: {:?}", err);
                        return Response::error("Failed to create WebSocket pair", 500);
                    }
                };

                let server = pair.server;
                let client = pair.client;

                #[allow(clippy::needless_borrows_for_generic_args)]
                self.state.accept_web_socket(&server);

                console_log!("DO: WebSocket accepted");

                match Response::from_websocket(client) {
                    Ok(resp) => {
                        console_log!("DO: Returning WebSocket 101 response");
                        Ok(resp)
                    }
                    Err(err) => {
                        console_error!("DO: Failed to create WebSocket response: {:?}", err);
                        Response::error("Failed to create WebSocket response", 500)
                    }
                }
            }
            Ok(header_opt) => {
                console_error!("DO: Unexpected Upgrade header state: {:?}", header_opt);
                Response::error("Expected WebSocket upgrade request", 426)
            }
            Err(err) => {
                console_error!("DO: Failed to read Upgrade header: {:?}", err);
                Response::error("Failed to read request headers", 500)
            }
        }
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: durable::WebSocketIncomingMessage,
    ) -> Result<()> {
        match message {
            durable::WebSocketIncomingMessage::String(_text) => {
                // Ignore text messages
            }
            durable::WebSocketIncomingMessage::Binary(bytes) => match C2S::from_bytes(&bytes) {
                Ok(c2s_msg) => {
                    if let Err(e) = Self::handle_c2s_message(self, ws, c2s_msg).await {
                        console_error!("Error handling C2S message: {e:?}");
                    }
                }
                Err(e) => {
                    console_error!("Failed to parse C2S message: {e:?}");
                }
            },
        }
        Ok(())
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        code: usize,
        reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        console_log!(
            "DO: WebSocket close event (code: {}, reason: {})",
            code,
            reason
        );

        // Recover which player owned this socket from its attachment.
        let player_id = ws.deserialize_attachment::<u8>().ok().flatten();

        let mut gs = self.game_state.borrow_mut();

        if let Some(player_id) = player_id {
            gs.env.log(format!("DO: Player {player_id} socket closed"));
            // Mid-match this starts a reconnect grace period; otherwise it removes the player.
            gs.handle_disconnect(player_id);
        } else {
            gs.env
                .log("DO: Close event with no attached player id; ignoring".to_string());
        }

        gs.env.log(format!(
            "DO: Remaining clients after cleanup: {}",
            gs.clients.len()
        ));
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, error: Error) -> Result<()> {
        console_error!("DO: WebSocket error: {:?}", error);
        Ok(())
    }

    #[allow(clippy::await_holding_refcell_ref)] // We drop the RefCell borrow before await
    async fn alarm(&self) -> Result<Response> {
        let mut gs = self.game_state.borrow_mut();

        // Check for idle clients and disconnect them (1 minute timeout)
        let now_ms = gs.env.now();
        let now_seconds = now_ms / 1000;
        let idle_timeout_seconds = 120; // 2 minutes
        let mut clients_to_remove = Vec::new();

        for (player_id, client_info) in gs.clients.iter() {
            let elapsed = now_seconds.saturating_sub(client_info.last_activity);
            if elapsed > idle_timeout_seconds {
                gs.env.log(format!(
                    "DO: Client {} idle for {}s (now: {}, last: {}), disconnecting",
                    player_id, elapsed, now_seconds, client_info.last_activity
                ));
                clients_to_remove.push(*player_id);
            }
        }

        // Remove idle clients
        for player_id in clients_to_remove {
            gs.remove_player(player_id);
        }

        // Check if we still have clients after cleanup
        let has_clients = !gs.clients.is_empty();
        if !has_clients {
            gs.env
                .log("DO: No clients remaining, stopping alarm loop".to_string());
            drop(gs);
            return Response::ok("No clients, stopping alarm loop");
        }

        // Handle based on current match state
        let current_state = gs.match_state;
        let next_alarm_ms = match current_state {
            MatchState::Waiting => {
                // Just keep alarm running at low frequency for idle checks
                500
            }
            MatchState::Countdown => {
                // Tick countdown every second
                gs.tick_countdown();
                1000
            }
            MatchState::Playing => {
                // Run game simulation at 60 Hz
                let tick_interval_ms = 16;

                let elapsed_ms = now_ms.saturating_sub(gs.last_tick_time);
                gs.last_tick_time = now_ms;

                // Add to accumulator, capped to avoid large jumps
                gs.accumulator += elapsed_ms.min(100) as f32;

                let mut steps_run = 0;
                const MAX_STEPS: u32 = 10;

                while gs.accumulator >= tick_interval_ms as f32 && steps_run < MAX_STEPS {
                    gs.step();
                    gs.accumulator -= tick_interval_ms as f32;
                    steps_run += 1;
                }

                if steps_run > 1 && gs.tick % 60 == 0 {
                    gs.env.log(format!(
                        "DO: Catching up, ran {steps_run} steps in one alarm"
                    ));
                }

                // Broadcast state regularly
                if gs.tick == 1 || gs.tick % 3 == 0 {
                    gs.broadcast_state();
                }

                tick_interval_ms
            }
            MatchState::Paused => {
                // Sim is frozen awaiting reconnect; forfeit once the grace window passes.
                if now_ms >= gs.reconnect_deadline_ms {
                    let dropped =
                        gs.clients
                            .iter()
                            .find_map(|(&p, info)| if !info.connected { Some(p) } else { None });
                    if let Some(pid) = dropped {
                        gs.env
                            .log("DO: Reconnect grace expired; forfeiting".to_string());
                        gs.remove_player(pid);
                    }
                }
                500
            }
            MatchState::GameOver => {
                // Low frequency, just for cleanup
                500
            }
        };

        drop(gs);

        // Schedule next alarm
        self.state
            .storage()
            .set_alarm(Duration::from_millis(next_alarm_ms))
            .await?;

        Response::ok("Alarm processed")
    }
}

impl MatchDO {
    /// Handle incoming C2S message
    async fn handle_c2s_message(&self, ws: WebSocket, msg: C2S) -> Result<()> {
        let should_start_alarm = {
            let mut gs = self.game_state.borrow_mut();
            match msg {
                C2S::Join { code: _, .. } if gs.match_state == MatchState::Paused => {
                    // A player is returning to a slot held open during the grace period.
                    if let Some(player_id) = gs.reconnect_player(Box::new(ws.clone())) {
                        if let Err(e) = ws.serialize_attachment(player_id) {
                            console_error!("DO: Failed to attach player id: {e:?}");
                        }
                        let welcome = S2C::Welcome { player_id };
                        if let Ok(bytes) = welcome.to_bytes() {
                            let _ = ws.send_with_bytes(&bytes);
                        }
                        let state_msg = gs.generate_state_message();
                        if let Ok(bytes) = state_msg.to_bytes() {
                            let _ = ws.send_with_bytes(&bytes);
                        }
                    } else {
                        gs.env
                            .log("DO: Reconnect Join but no open slot to resume".to_string());
                    }
                    // The alarm loop is already running while paused.
                    None
                }
                C2S::Join { code: _, .. } => {
                    // We need to clone WS here because add_player takes ownership
                    if let Some((player_id, was_empty)) = gs.add_player(Box::new(ws.clone())) {
                        // Tag this socket with its player id so we can identify it on
                        // close/ping (the hibernation API gives us back the same socket).
                        if let Err(e) = ws.serialize_attachment(player_id) {
                            console_error!("DO: Failed to attach player id: {e:?}");
                        }
                        gs.env.log(format!(
                            "DO: Player {player_id} joining (clients was empty: {was_empty})"
                        ));
                        // Send Welcome message
                        let welcome = S2C::Welcome { player_id };
                        if let Ok(bytes) = welcome.to_bytes() {
                            let _ = ws.send_with_bytes(&bytes);
                        }

                        // Send initial state
                        let state_msg = gs.generate_state_message();
                        if let Ok(bytes) = state_msg.to_bytes() {
                            // Broadcast to all
                            for client_info in gs.clients.values() {
                                let _ = client_info.client.send_bytes(&bytes);
                            }
                        }
                        Some(was_empty)
                    } else {
                        gs.env
                            .log("DO: Match full, rejecting new player".to_string());
                        None
                    }
                }
                C2S::Input {
                    player_id,
                    y,
                    seq: _,
                } => {
                    gs.handle_input(player_id, y);
                    None
                }
                C2S::Restart => {
                    gs.restart_match();
                    None
                }
                C2S::Ping { t_ms } => {
                    // Refresh activity for the sender only, identified by socket attachment.
                    let now = gs.env.now() / 1000;
                    if let Ok(Some(player_id)) = ws.deserialize_attachment::<u8>() {
                        if let Some(client_info) = gs.clients.get_mut(&player_id) {
                            client_info.last_activity = now;
                        }
                    }

                    let pong = S2C::Pong { t_ms };
                    if let Ok(bytes) = pong.to_bytes() {
                        let _ = ws.send_with_bytes(&bytes);
                    }
                    None
                }
            }
        };

        // Start game loop if this was the first player
        if let Some(true) = should_start_alarm {
            self.state
                .storage()
                .set_alarm(Duration::from_millis(16)) // 60 Hz
                .await?;
        }

        Ok(())
    }
}
