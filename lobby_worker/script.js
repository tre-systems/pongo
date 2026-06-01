// Sound effects using Web Audio API
const AudioContext = window.AudioContext || window.webkitAudioContext;
let audioCtx = null;

function initAudio() {
  if (!audioCtx) {
    audioCtx = new AudioContext();
  }
}

function playSound(type) {
  if (!audioCtx) return;

  const playTone = (freq, type, duration, vol = 0.1, volEnd = 0.01) => {
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.connect(gain);
    gain.connect(audioCtx.destination);

    if (Array.isArray(freq)) {
      // Arpeggio / sequence
      freq.forEach((f, i) => {
        osc.frequency.setValueAtTime(f, audioCtx.currentTime + i * 0.15);
      });
    } else {
      osc.frequency.setValueAtTime(freq, audioCtx.currentTime);
    }

    osc.type = type;
    gain.gain.setValueAtTime(vol, audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(volEnd, audioCtx.currentTime + duration);

    osc.start();
    osc.stop(audioCtx.currentTime + duration);
  };

  switch (type) {
    case "paddle":
      playTone(440, "sine", 0.1);
      break;
    case "wall":
      playTone(220, "sine", 0.08);
      break;
    case "score":
      const osc = audioCtx.createOscillator();
      const gain = audioCtx.createGain();
      osc.connect(gain);
      gain.connect(audioCtx.destination);
      osc.frequency.setValueAtTime(400, audioCtx.currentTime);
      osc.frequency.exponentialRampToValueAtTime(800, audioCtx.currentTime + 0.2);
      osc.type = "triangle";
      gain.gain.setValueAtTime(0.1, audioCtx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.01, audioCtx.currentTime + 0.3);
      osc.start();
      osc.stop(audioCtx.currentTime + 0.3);
      break;
    case "victory":
      [0, 0.15, 0.3].forEach((delay, i) => {
        const o = audioCtx.createOscillator();
        const g = audioCtx.createGain();
        o.connect(g);
        g.connect(audioCtx.destination);
        o.frequency.setValueAtTime([523, 659, 784][i], audioCtx.currentTime + delay);
        o.type = "triangle";
        g.gain.setValueAtTime(0.1, audioCtx.currentTime + delay);
        g.gain.exponentialRampToValueAtTime(0.01, audioCtx.currentTime + delay + 0.3);
        o.start(audioCtx.currentTime + delay);
        o.stop(audioCtx.currentTime + delay + 0.3);
      });
      break;
  }
}

const CACHE_BUST = new URLSearchParams(window.location.search).get("v") || Date.now();
const wasmUrl = "/client_wasm/client_wasm.js?v=" + CACHE_BUST;

let init, WasmClient, GameFsm, FsmState;
try {
  const module = await import(wasmUrl);
  init = module.default;
  WasmClient = module.WasmClient;
  GameFsm = module.GameFsm;
  FsmState = module.FsmState;

  // Initialize WASM before using any exports
  await init();
} catch (error) {
  console.error("Failed to load WASM:", error);
  document.getElementById("status").textContent = "Failed to load game resource";
  throw error;
}

let client = null;
let ws = null;
let lastScore = [0, 0];
let renderLoopId = null;
let pingIntervalId = null;
let inputIntervalId = null;
let currentMatchCode = null;

// ========================================
// Finite State Machine (Rust-backed)
// ========================================
// Map Rust FsmState enum to string names for compatibility
const GameState = {
  IDLE: FsmState.Idle,
  COUNTDOWN_LOCAL: FsmState.CountdownLocal,
  PLAYING_LOCAL: FsmState.PlayingLocal,
  PAUSED: FsmState.Paused,
  CONNECTING: FsmState.Connecting,
  WAITING: FsmState.Waiting,
  COUNTDOWN_MULTI: FsmState.CountdownMulti,
  PLAYING_MULTI: FsmState.PlayingMulti,
  GAME_OVER_LOCAL: FsmState.GameOverLocal,
  GAME_OVER_MULTI: FsmState.GameOverMulti,
  DISCONNECTED: FsmState.Disconnected,
  RECONNECTING: FsmState.Reconnecting,
};

// Reconnect state (multiplayer): retry the socket within the server's grace window.
let intentionalClose = false;
let reconnectAttempts = 0;
let reconnectTimerId = null;
const MAX_RECONNECT_ATTEMPTS = 10;
const RECONNECT_INTERVAL_MS = 1500;

// Create Rust FSM instance (after init())
const rustFsm = new GameFsm();

// Wrapper that maintains same API but uses Rust FSM
const FSM = {
  get state() {
    return rustFsm.state;
  },

  isTransitioning: false,

  async transition(action) {
    if (this.isTransitioning) {
      console.warn(`[FSM] Transition locked. Ignoring action: ${action}`);
      return false;
    }

    this.isTransitioning = true;
    try {
      const prevState = rustFsm.state;
      const result = rustFsm.transition_str(action);

      if (!result.success) {
        console.warn(`Invalid transition: ${rustFsm.state_string()} + ${action}`);
        return false;
      }

      console.log(`FSM: ${result.from_state} --[${action}]--> ${result.to_state}`);
      await this.exitState(prevState);
      await this.enterState(rustFsm.state, prevState);
      return true;
    } finally {
      this.isTransitioning = false;
    }
  },

  async exitState(state) {
    switch (state) {
      case FsmState.PlayingLocal:
      case FsmState.PlayingMulti:
        stopGameLoop();
        break;
      case FsmState.Waiting:
      case FsmState.CountdownMulti:
        stopEventPolling();
        hideCountdownNumber();
        break;
      case FsmState.GameOverMulti:
        stopEventPolling();
      // Fallthrough
      case FsmState.GameOverLocal:
        hideVictoryOverlay();
        break;
      case FsmState.Reconnecting:
        stopReconnect();
        break;
      case FsmState.Connecting:
        break;
    }
  },

  async enterState(state, prevState) {
    updateUIForState(state);
    switch (state) {
      case FsmState.Idle:
        await enterIdle();
        break;
      case FsmState.CountdownLocal:
        await enterCountdownLocal();
        break;
      case FsmState.PlayingLocal:
        // Resuming from a pause must not restart the match.
        enterPlayingLocal(prevState === FsmState.Paused);
        break;
      case FsmState.Connecting:
        await enterConnecting();
        break;
      case FsmState.Waiting:
        enterWaiting();
        break;
      case FsmState.CountdownMulti:
        await enterCountdownMulti();
        break;
      case FsmState.PlayingMulti:
        enterPlayingMulti();
        break;
      case FsmState.GameOverLocal:
      case FsmState.GameOverMulti:
        if (state === FsmState.GameOverMulti) {
          startEventPolling();
        }
        break;
      case FsmState.Reconnecting:
        await enterReconnecting();
        break;
      case FsmState.Disconnected:
        enterDisconnected();
        break;
    }
  },
};

// ========================================
// State Entry Handlers
// ========================================
async function enterIdle() {
  closeWebSocket();
  stopGameLoop();
  stopReconnect();
  lastScore = [0, 0];
  currentMatchCode = null;

  // Clear URL params
  const url = new URL(window.location);
  if (url.searchParams.has("code")) {
    url.searchParams.delete("code");
    window.history.pushState({}, "", url);
  }
  document.getElementById("matchCode").value = "";
}

async function enterCountdownLocal() {
  initAudio();
  const canvas = document.getElementById("canvas");
  if (!client) {
    client = await new WasmClient(canvas);
  }
  // Do NOT await here, so the transition lock releases.
  showCountdown().then(() => {
    FSM.transition("COUNTDOWN_DONE");
  });
}

function enterPlayingLocal(resuming = false) {
  if (resuming) {
    // Resuming after a pause: keep the in-progress game, just unfreeze cleanly
    // so the first frame doesn't accumulate a large dt and fast-forward.
    client.reset_sim_timing();
  } else {
    client.start_local_game();
    lastScore = [0, 0];
    updateScore(0, 0);
  }
  document.body.classList.add("game-active");
  setupInputIfNeeded();
  startRenderLoop();
}

async function enterConnecting() {
  initAudio();
  const canvas = document.getElementById("canvas");
  if (!client) {
    client = await new WasmClient(canvas);
  }

  const code = currentMatchCode || document.getElementById("matchCode").value.trim().toUpperCase();
  if (code.length !== 5) {
    FSM.transition("CONNECTION_FAILED");
    return;
  }
  currentMatchCode = code;

  // Update URL
  const url = new URL(window.location);
  if (url.searchParams.get("code") !== code) {
    url.searchParams.set("code", code);
    window.history.pushState({}, "", url);
  }

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${protocol}//${window.location.host}/ws/${code}`;

  try {
    ws = new WebSocket(wsUrl);
    ws.binaryType = "arraybuffer";

    ws.onopen = () => {
      try {
        ws.send(client.get_join_bytes(code));
        FSM.transition("CONNECTED");
      } catch (e) {
        console.error("Join error:", e);
        FSM.transition("CONNECTION_FAILED");
      }
    };

    ws.onmessage = onWsMessage;

    ws.onerror = () => {
      if (FSM.state === GameState.CONNECTING) {
        FSM.transition("CONNECTION_FAILED");
      }
    };

    ws.onclose = () => {
      if (intentionalClose) {
        intentionalClose = false;
        return;
      }
      if (FSM.state === GameState.PLAYING_MULTI || FSM.state === GameState.COUNTDOWN_MULTI) {
        // Recover within the server's grace window instead of ending the match.
        FSM.transition("CONNECTION_LOST");
      } else if (FSM.state === GameState.WAITING || FSM.state === GameState.GAME_OVER_MULTI) {
        FSM.transition("DISCONNECTED");
      } else if (FSM.state === GameState.CONNECTING) {
        FSM.transition("CONNECTION_FAILED");
      }
    };
  } catch (e) {
    FSM.transition("CONNECTION_FAILED");
  }
}

function enterWaiting() {
  // Start polling for events while waiting
  startEventPolling();
}

async function enterCountdownMulti() {
  // Reset client state for fresh start - critical for rematch smoothness
  if (client) {
    client.reset_for_multiplayer();
  }
  // Server drives the countdown - we just display what it sends
  // The countdown display is handled by handleMatchEvent
  // Wait here until game_start is received
  // (Transition to PLAYING_MULTI is triggered by handleMatchEvent)
}

// Handle match events from server
function handleMatchEvent(event) {
  console.log("Match event:", event, "State:", FSM.state);

  if (event === "match_found") {
    if (FSM.state === GameState.WAITING) {
      playSound("paddle");
      FSM.transition("OPPONENT_JOINED");
    }
  } else if (event.startsWith("countdown:")) {
    const seconds = parseInt(event.split(":")[1]);
    showCountdownNumber(seconds);

    if (FSM.state === GameState.GAME_OVER_MULTI) {
      FSM.transition("REMATCH_STARTED");
    }
  } else if (event === "game_start") {
    hideCountdownNumber();
    if (FSM.state === GameState.COUNTDOWN_MULTI) {
      FSM.transition("COUNTDOWN_DONE");
    }
  } else if (event === "opponent_disconnected") {
    // Handle opponent leaving at any multiplayer stage
    if (
      FSM.state === GameState.WAITING ||
      FSM.state === GameState.COUNTDOWN_MULTI ||
      FSM.state === GameState.PLAYING_MULTI ||
      FSM.state === GameState.GAME_OVER_MULTI
    ) {
      FSM.transition("DISCONNECTED");
    }
  } else if (event === "opponent_reconnecting") {
    // We're still connected; our opponent dropped and the match is paused.
    showReconnectOverlay("Opponent reconnecting…");
  } else if (event === "opponent_reconnected") {
    hideReconnectOverlay();
  }
}

// Show a single countdown number
function showCountdownNumber(n) {
  const el = document.getElementById("countdown");
  el.textContent = n > 0 ? n.toString() : "GO!";
  el.classList.add("show");
  playSound("paddle");
  setTimeout(() => el.classList.remove("show"), 800);
}

function hideCountdownNumber() {
  document.getElementById("countdown").classList.remove("show");
}

let eventPollingId = null;
function startEventPolling() {
  stopEventPolling();
  eventPollingId = setInterval(() => {
    if (client && ws && ws.readyState === WebSocket.OPEN) {
      const matchEvent = client.get_match_event();
      if (matchEvent) {
        handleMatchEvent(matchEvent);
      }
    }
  }, 100);
}

function stopEventPolling() {
  if (eventPollingId) {
    clearInterval(eventPollingId);
    eventPollingId = null;
  }
}

function enterPlayingMulti() {
  lastScore = [0, 0];
  document.body.classList.add("game-active");
  setupInputIfNeeded();
  startRenderLoop();
  startPingInterval();
  // Start input polling (must be called every game, not just once)
  if (!inputIntervalId) {
    inputIntervalId = setInterval(sendInput, 33);
  }
}

function enterDisconnected() {
  closeWebSocket();
  stopGameLoop();
  hideReconnectOverlay();
}

// ========================================
// UI Updates
// ========================================
function updateUIForState(state) {
  const playBtn = document.getElementById("playBtn");
  const lobbyControls = document.getElementById("lobbyControls");
  const activeMatchControls = document.getElementById("activeMatchControls");
  const quitBtn = document.getElementById("quitBtn");
  const pauseBtn = document.getElementById("pauseBtn");

  // Pause/Resume is local-only; hidden by default and shown in the cases below.
  pauseBtn.style.display = "none";

  switch (state) {
    case GameState.IDLE:
      playBtn.textContent = "Play Now";
      playBtn.disabled = false;
      playBtn.classList.remove("playing");
      lobbyControls.style.display = "block";
      activeMatchControls.style.display = "none";
      quitBtn.style.display = "none";
      document.body.classList.remove("game-active");
      break;

    case GameState.COUNTDOWN_LOCAL:
      playBtn.textContent = "Starting...";
      playBtn.disabled = true;
      lobbyControls.style.display = "none";
      quitBtn.style.display = "block";
      break;

    case GameState.PLAYING_LOCAL:
      playBtn.textContent = "Playing vs AI";
      playBtn.classList.add("playing");
      lobbyControls.style.display = "none";
      quitBtn.style.display = "block";
      pauseBtn.style.display = "block";
      pauseBtn.textContent = "Pause";
      break;

    case GameState.PAUSED:
      playBtn.textContent = "Paused";
      playBtn.classList.remove("playing");
      lobbyControls.style.display = "none";
      quitBtn.style.display = "block";
      pauseBtn.style.display = "block";
      pauseBtn.textContent = "Resume";
      break;

    case GameState.CONNECTING:
      playBtn.textContent = "Connecting...";
      playBtn.disabled = true;
      lobbyControls.style.display = "none";
      activeMatchControls.style.display = "block";
      break;

    case GameState.WAITING:
      playBtn.textContent = "Waiting...";
      break;

    case GameState.COUNTDOWN_MULTI:
      playBtn.textContent = "Get Ready!";
      activeMatchControls.style.display = "none";
      break;

    case GameState.PLAYING_MULTI:
      playBtn.textContent = "In Match";
      playBtn.classList.add("playing");
      lobbyControls.style.display = "none";
      activeMatchControls.style.display = "none";
      break;

    case GameState.GAME_OVER_LOCAL:
    case GameState.GAME_OVER_MULTI:
      playBtn.textContent = "Game Over";
      playBtn.classList.remove("playing");
      activeMatchControls.style.display = "none";
      break;

    case GameState.RECONNECTING:
      playBtn.textContent = "Reconnecting...";
      playBtn.disabled = true;
      playBtn.classList.remove("playing");
      lobbyControls.style.display = "none";
      activeMatchControls.style.display = "none";
      break;

    case GameState.DISCONNECTED:
      playBtn.textContent = "Disconnected";
      playBtn.disabled = false;
      playBtn.classList.remove("playing");
      document.body.classList.remove("game-active");
      break;
  }
}

function updateScore(left, right) {
  const el = document.getElementById("score");
  if (el) {
    if (left !== lastScore[0] || right !== lastScore[1]) {
      playSound("score");
      el.classList.add("flash");
      setTimeout(() => el.classList.remove("flash"), 200);
      lastScore = [left, right];
    }
    el.textContent = `${left} : ${right}`;
  }
}

function updateMetrics() {
  if (client) {
    try {
      const metrics = client.get_metrics();
      if (metrics.length >= 2) {
        document.getElementById("fps").textContent = Math.round(metrics[0]);
        document.getElementById("ping").textContent = Math.round(metrics[1]);
      }
    } catch (e) {
      console.error("Metrics update failed:", e);
    }
  }
}

// ========================================
// Game Loop
// ========================================
function startRenderLoop() {
  function render() {
    if (client) {
      try {
        client.render();
        updateMetrics();
        const score = client.get_score();
        if (score.length >= 2) {
          updateScore(score[0], score[1]);

          // Check for game over
          const winner = client.get_winner();
          if (winner) {
            showVictory(winner);
            FSM.transition("GAME_OVER");
            return;
          }
          if (score[0] >= 5 || score[1] >= 5) {
            const winnerName = score[0] >= 5 ? "you" : "opponent";
            showVictory(winnerName);
            if (FSM.state === GameState.PLAYING_LOCAL) {
              FSM.transition("GAME_OVER_LOCAL");
            } else {
              FSM.transition("GAME_OVER_MULTI");
            }
            return;
          }
        }
      } catch (e) {
        console.error("Render loop error:", e);
      }
    }

    if (FSM.state === GameState.PLAYING_LOCAL || FSM.state === GameState.PLAYING_MULTI) {
      renderLoopId = requestAnimationFrame(render);
    }
  }
  render();
}

function stopGameLoop() {
  if (renderLoopId) {
    cancelAnimationFrame(renderLoopId);
    renderLoopId = null;
  }
  if (pingIntervalId) {
    clearInterval(pingIntervalId);
    pingIntervalId = null;
  }
  if (inputIntervalId) {
    clearInterval(inputIntervalId);
    inputIntervalId = null;
  }
}

function startPingInterval() {
  pingIntervalId = setInterval(() => {
    if (ws && ws.readyState === WebSocket.OPEN && client) {
      try {
        const pingBytes = client.send_ping();
        ws.send(pingBytes);
      } catch (e) {
        console.error("Ping failed:", e);
      }
    }
  }, 2000);
}

// ========================================
// WebSocket & Input
// ========================================
function closeWebSocket() {
  if (ws) {
    // Mark this close as intentional so the socket's onclose doesn't try to reconnect.
    intentionalClose = true;
    ws.close();
    ws = null;
  }
}

// Shared handler for server messages, used by both the initial and reconnect sockets.
function onWsMessage(event) {
  if (!(event.data instanceof ArrayBuffer)) return;
  try {
    client.on_message(new Uint8Array(event.data));

    const matchEvent = client.get_match_event();
    if (matchEvent) {
      handleMatchEvent(matchEvent);
    }

    if (FSM.state === GameState.PLAYING_MULTI) {
      const score = client.get_score();
      if (score.length >= 2) {
        updateScore(score[0], score[1]);
      }
    }
  } catch (e) {
    console.error("Message error:", e);
  }
}

// ========================================
// Reconnect (multiplayer)
// ========================================
async function enterReconnecting() {
  stopGameLoop();
  showReconnectOverlay("Connection lost — reconnecting…");
  reconnectAttempts = 0;
  attemptReconnect();
}

function attemptReconnect() {
  reconnectTimerId = null;
  if (FSM.state !== GameState.RECONNECTING) return;

  if (!currentMatchCode || reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
    FSM.transition("RECONNECT_FAILED");
    return;
  }
  reconnectAttempts++;

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${protocol}//${window.location.host}/ws/${currentMatchCode}`;

  let sock;
  try {
    sock = new WebSocket(wsUrl);
  } catch (e) {
    scheduleReconnect();
    return;
  }
  sock.binaryType = "arraybuffer";

  sock.onopen = () => {
    ws = sock;
    try {
      sock.send(client.get_join_bytes(currentMatchCode));
    } catch (e) {
      console.error("Reconnect join failed:", e);
    }
  };

  sock.onmessage = (event) => {
    // The first message back means the server re-attached us to our slot.
    if (FSM.state === GameState.RECONNECTING) {
      FSM.transition("RECONNECTED");
    }
    onWsMessage(event);
  };

  sock.onerror = () => {};

  sock.onclose = () => {
    if (intentionalClose) {
      intentionalClose = false;
      return;
    }
    if (FSM.state === GameState.RECONNECTING) {
      scheduleReconnect();
    }
  };
}

function scheduleReconnect() {
  if (reconnectTimerId) clearTimeout(reconnectTimerId);
  reconnectTimerId = setTimeout(attemptReconnect, RECONNECT_INTERVAL_MS);
}

function stopReconnect() {
  if (reconnectTimerId) {
    clearTimeout(reconnectTimerId);
    reconnectTimerId = null;
  }
  hideReconnectOverlay();
}

function showReconnectOverlay(text) {
  const overlay = document.getElementById("reconnectOverlay");
  if (!overlay) return;
  document.getElementById("reconnectText").textContent = text;
  overlay.classList.add("show");
}

function hideReconnectOverlay() {
  const overlay = document.getElementById("reconnectOverlay");
  if (overlay) overlay.classList.remove("show");
}

function sendInput() {
  if (ws && ws.readyState === WebSocket.OPEN && client) {
    try {
      const bytes = client.get_input_bytes();
      if (bytes.length > 0) {
        ws.send(bytes);
      }
    } catch (e) {
      console.error("Send input failed:", e);
    }
  }
}

// ========================================
// Auto-detect Touch Support
// ========================================
if (navigator.maxTouchPoints > 0 || "ontouchstart" in window) {
  document.body.classList.add("touch-enabled");
}

// ========================================
// Pop Out Button Logic
// ========================================
const popOutBtn = document.getElementById("popOutBtn");
if (popOutBtn) {
  // Hide if we are already in a popup (check window size or opener)
  if (window.opener && window.innerWidth < 600) {
    popOutBtn.style.display = "none";
  }

  popOutBtn.addEventListener("click", () => {
    window.open(
      "/",
      "GamePopout",
      "width=380,height=800,menubar=no,toolbar=no,location=no,status=no,resizable=yes,scrollbars=no"
    );
  });
}

let inputSetup = false;
function setupInputIfNeeded() {
  if (inputSetup) return;
  inputSetup = true;

  const pressedKeys = new Set();
  window.addEventListener(
    "keydown",
    (e) => {
      const gameKeys = ["ArrowUp", "ArrowDown", "w", "W", "s", "S"];
      if (gameKeys.includes(e.key)) {
        e.preventDefault();
        const upKeys = ["ArrowUp", "w", "W"];
        const downKeys = ["ArrowDown", "s", "S"];
        if (upKeys.includes(e.key)) {
          downKeys.forEach((key) => {
            if (pressedKeys.has(key)) {
              pressedKeys.delete(key);
              if (client) client.handle_key_string(key, false);
            }
          });
          pressedKeys.add(e.key);
        } else if (downKeys.includes(e.key)) {
          upKeys.forEach((key) => {
            if (pressedKeys.has(key)) {
              pressedKeys.delete(key);
              if (client) client.handle_key_string(key, false);
            }
          });
          pressedKeys.add(e.key);
        }
      }
      if (client) {
        client.on_key_down(e);
        sendInput();
      }
    },
    { capture: true, passive: false }
  );

  window.addEventListener(
    "keyup",
    (e) => {
      const gameKeys = ["ArrowUp", "ArrowDown", "w", "W", "s", "S"];
      if (gameKeys.includes(e.key)) {
        e.preventDefault();
        pressedKeys.delete(e.key);
      }
      if (client) {
        client.on_key_up(e);
        sendInput();
      }
    },
    { capture: true, passive: false }
  );

  // --- Touch & Mouse Slide Controls ---
  let currentTouchDir = 0;
  let isPointerDown = false;

  const handlePointerUpdate = (x, y) => {
    const target = document.elementFromPoint(x, y);

    let newDir = 0;
    if (target && target.classList.contains("touch-btn")) {
      newDir = target.dataset.dir === "up" ? -1 : 1;
    }

    // Update visual state of all buttons
    document.querySelectorAll(".touch-btn").forEach((btn) => {
      if (newDir !== 0 && btn.dataset.dir === (newDir === -1 ? "up" : "down")) {
        btn.classList.add("pressed");
      } else {
        btn.classList.remove("pressed");
      }
    });

    // Update game input
    if (newDir !== currentTouchDir) {
      if (!client) return;

      // Release old direction
      if (currentTouchDir === -1) client.handle_key_string("ArrowUp", false);
      if (currentTouchDir === 1) client.handle_key_string("ArrowDown", false);

      // Press new direction
      if (newDir === -1) client.handle_key_string("ArrowUp", true);
      if (newDir === 1) client.handle_key_string("ArrowDown", true);

      currentTouchDir = newDir;
      sendInput();
    }
  };

  const handlePointerEnd = () => {
    isPointerDown = false;

    // Clear visuals
    document.querySelectorAll(".touch-btn").forEach((btn) => {
      btn.classList.remove("pressed");
    });

    // Release input
    if (currentTouchDir !== 0 && client) {
      if (currentTouchDir === -1) client.handle_key_string("ArrowUp", false);
      if (currentTouchDir === 1) client.handle_key_string("ArrowDown", false);
      currentTouchDir = 0;
      sendInput();
    }
  };

  // --- Touch Event Listeners ---
  document.addEventListener(
    "touchstart",
    (e) => {
      if (e.target.classList.contains("touch-btn")) {
        e.preventDefault();
        isPointerDown = true; // reusing checking flag for consistency
        if (e.touches.length > 0) {
          handlePointerUpdate(e.touches[0].clientX, e.touches[0].clientY);
        }
      }
    },
    { passive: false }
  );

  document.addEventListener(
    "touchmove",
    (e) => {
      // Allow sliding if we're "active" or simply if moving over buttons
      if (isPointerDown || e.target.classList.contains("touch-btn")) {
        e.preventDefault();
        if (e.touches.length > 0) {
          handlePointerUpdate(e.touches[0].clientX, e.touches[0].clientY);
        }
      }
    },
    { passive: false }
  );

  document.addEventListener("touchend", (e) => {
    if (isPointerDown) {
      handlePointerEnd();
    }
  });

  document.addEventListener("touchcancel", (e) => {
    if (isPointerDown) {
      handlePointerEnd();
    }
  });

  // --- Mouse Event Listeners (Desktop Testing) ---
  document.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("touch-btn")) {
      isPointerDown = true;
      handlePointerUpdate(e.clientX, e.clientY);
    }
  });

  document.addEventListener("mousemove", (e) => {
    if (isPointerDown) {
      e.preventDefault(); // Prevent text selection while dragging
      handlePointerUpdate(e.clientX, e.clientY);
    }
  });

  document.addEventListener("mouseup", (e) => {
    if (isPointerDown) {
      handlePointerEnd();
    }
  });
}

// ========================================
// Victory/Countdown Overlays
// ========================================
async function showCountdown() {
  const el = document.getElementById("countdown");
  for (const text of ["3", "2", "1", "GO!"]) {
    el.textContent = text;
    el.classList.add("show");
    playSound("paddle");
    await new Promise((r) => setTimeout(r, 600));
    el.classList.remove("show");
    await new Promise((r) => setTimeout(r, 100));
  }
}

function showVictory(winner) {
  playSound("victory");
  const overlay = document.getElementById("victoryOverlay");
  const text = document.getElementById("victoryText");

  if (winner === "you") {
    text.textContent = "VICTORY";
    text.className = "status-win";
  } else {
    text.textContent = "DEFEAT";
    text.className = "status-lose";
  }

  // Reset button
  const btn = document.getElementById("playAgainBtn");
  btn.textContent = "Play Again";
  btn.disabled = false;

  overlay.classList.add("show");
  document.body.classList.remove("game-active");
}

function hideVictoryOverlay() {
  document.getElementById("victoryOverlay").classList.remove("show");
}

// ========================================
// Public API / Event Handlers
// ========================================
const startLocalGame = async function () {
  if (FSM.state === GameState.IDLE) {
    await FSM.transition("START_LOCAL");
  } else if (FSM.state === GameState.GAME_OVER_LOCAL) {
    await FSM.transition("PLAY_AGAIN");
  }
};

const createMatch = async function () {
  if (FSM.state !== GameState.IDLE) return;
  try {
    const response = await fetch("/create");
    const data = await response.json();
    document.getElementById("matchCode").value = data.code;
    currentMatchCode = data.code;

    // Display game code prominently
    const codeDisplay = document.getElementById("gameCodeDisplay");
    if (codeDisplay) {
      codeDisplay.textContent = data.code;
    }

    // Build and copy invite URL
    const url = new URL(window.location);
    url.searchParams.set("code", data.code);
    const linkStr = url.toString();

    document.getElementById("shareLinkInput").value = linkStr;
    const feedback = document.getElementById("copyFeedback");

    try {
      await navigator.clipboard.writeText(linkStr);
      feedback.textContent = "LINK COPIED TO CLIPBOARD";
      feedback.style.color = "var(--accent)";
    } catch (e) {
      console.warn("Could not copy to clipboard:", e);
      feedback.textContent = "Share the code above";
      feedback.style.color = "var(--text-muted)";
    }

    await FSM.transition("CREATE_MATCH");
  } catch (e) {
    console.error("Create match error:", e);
  }
};

// Copy code button handler
document.getElementById("copyCodeBtn").addEventListener("click", async () => {
  const code = document.getElementById("gameCodeDisplay").textContent;
  const feedback = document.getElementById("copyFeedback");
  try {
    await navigator.clipboard.writeText(code);
    feedback.textContent = "✓ Code copied!";
  } catch (e) {
    // Fallback - copy the full link instead
    const link = document.getElementById("shareLinkInput").value;
    await navigator.clipboard.writeText(link);
    feedback.textContent = "✓ Link copied!";
  }
  // Clear feedback after 2 seconds
  setTimeout(() => {
    feedback.textContent = "";
  }, 2000);
});

// Native Share API handler
if (navigator.share) {
  document.getElementById("shareLinkBtn").style.display = "flex";
}

document.getElementById("shareLinkBtn").addEventListener("click", async () => {
  const link = document.getElementById("shareLinkInput").value;
  try {
    await navigator.share({
      title: "Lets Play Pong!",
      text: "Join me for a match of Pong!",
      url: link,
    });
  } catch (err) {
    console.error("Share failed:", err);
  }
});

// Join with Code handlers
document.getElementById("joinCodeToggle").addEventListener("click", () => {
  const toggle = document.getElementById("joinCodeToggle");
  const form = document.getElementById("joinCodeForm");

  if (form.classList.contains("show")) {
    form.classList.remove("show");
    toggle.textContent = "Join with Code";
  } else {
    form.classList.add("show");
    toggle.textContent = "Cancel";
    document.getElementById("joinCodeInput").focus();
  }
});

document.getElementById("joinCodeInput").addEventListener("input", (e) => {
  const value = e.target.value.toUpperCase().replace(/[^A-Z0-9]/g, "");
  e.target.value = value;
  document.getElementById("joinCodeBtn").disabled = value.length !== 5;
});

document.getElementById("joinCodeBtn").addEventListener("click", () => {
  const code = document.getElementById("joinCodeInput").value.trim().toUpperCase();
  if (code.length === 5) {
    // Navigate to the URL with the code parameter
    const url = new URL(window.location);
    url.searchParams.set("code", code);
    window.location.href = url.toString();
  }
});

// Allow Enter key to submit the code
document.getElementById("joinCodeInput").addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    const code = e.target.value.trim().toUpperCase();
    if (code.length === 5) {
      const url = new URL(window.location);
      url.searchParams.set("code", code);
      window.location.href = url.toString();
    }
  }
});

const joinMatch = async function () {
  if (FSM.state !== GameState.IDLE) return;
  const code = document.getElementById("matchCode").value.trim().toUpperCase();
  if (code.length !== 5) return;
  currentMatchCode = code;
  await FSM.transition("JOIN_MATCH");
};

const leaveGame = function () {
  // The Rust FSM rejects invalid transitions, so this is safe to call from any state.
  FSM.transition("LEAVE");
};

// ========================================
// Initialization
// ========================================
document.getElementById("playAgainBtn").addEventListener("click", () => {
  if (FSM.state === GameState.GAME_OVER_LOCAL) {
    FSM.transition("PLAY_AGAIN");
  } else if (FSM.state === GameState.GAME_OVER_MULTI) {
    // Send restart request
    if (client && ws && ws.readyState === WebSocket.OPEN) {
      try {
        const bytes = client.get_restart_bytes();
        ws.send(bytes);
        // Immediate reset for better UX
        client.reset_local_state();
        hideVictoryOverlay();
      } catch (e) {
        console.error(e);
      }
    }
  }
});

document.getElementById("leaveBtn").addEventListener("click", () => {
  leaveGame();
});

async function main() {
  try {
    // WASM already initialized after import
    document.getElementById("playBtn").disabled = false;
    document.getElementById("playBtn").addEventListener("click", startLocalGame);
    document.getElementById("createBtn").addEventListener("click", createMatch);
    document.getElementById("quitBtn").addEventListener("click", () => {
      // Stop the local game and return to idle
      stopGameLoop();
      if (client) {
        client.stop_local_game();
      }
      FSM.transition("QUIT");
    });

    // Pause/Resume (local games only)
    const togglePause = () => {
      if (FSM.state === GameState.PLAYING_LOCAL) {
        FSM.transition("PAUSE");
      } else if (FSM.state === GameState.PAUSED) {
        FSM.transition("RESUME");
      }
    };
    document.getElementById("pauseBtn").addEventListener("click", togglePause);
    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape" || e.key === "p" || e.key === "P") {
        togglePause();
      }
    });

    document.getElementById("quitGameBtn").addEventListener("click", async () => {
      // Close WebSocket connection and return to idle
      closeWebSocket();
      stopGameLoop();
      hideVictoryOverlay();
      await FSM.transition("LEAVE");
    });

    // Auto-join if code in URL
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get("code");
    if (code) {
      document.getElementById("matchCode").value = code;
      currentMatchCode = code;
      await FSM.transition("JOIN_MATCH");
    }
  } catch (error) {
    console.error("Init error:", error);
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
});
