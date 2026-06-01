// Countdown, victory, and reconnect overlays — pure DOM, no game/FSM state.
import { playSound } from "./audio.js";

// Animated 3-2-1-GO! countdown for local games. Resolves when finished.
export async function showCountdown() {
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

// Single countdown number flashed on a server-driven multiplayer countdown.
export function showCountdownNumber(n) {
  const el = document.getElementById("countdown");
  el.textContent = n > 0 ? n.toString() : "GO!";
  el.classList.add("show");
  playSound("paddle");
  setTimeout(() => el.classList.remove("show"), 800);
}

export function hideCountdownNumber() {
  document.getElementById("countdown").classList.remove("show");
}

export function showVictory(winner) {
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

export function hideVictoryOverlay() {
  document.getElementById("victoryOverlay").classList.remove("show");
}

export function showReconnectOverlay(text) {
  const overlay = document.getElementById("reconnectOverlay");
  if (!overlay) return;
  document.getElementById("reconnectText").textContent = text;
  overlay.classList.add("show");
}

export function hideReconnectOverlay() {
  const overlay = document.getElementById("reconnectOverlay");
  if (overlay) overlay.classList.remove("show");
}
