// Keyboard, touch, and mouse input wiring. `client` is the WasmClient and
// `sendInput` flushes the current input to the server; both are injected so this
// module stays decoupled from the FSM/networking core. Listeners attach once.
let inputSetup = false;

export function setupInput(client, sendInput) {
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
  const UP_KEY = "ArrowUp";
  const DOWN_KEY = "ArrowDown";
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
      if (currentTouchDir === -1) client.handle_key_string(UP_KEY, false);
      if (currentTouchDir === 1) client.handle_key_string(DOWN_KEY, false);

      // Press new direction
      if (newDir === -1) client.handle_key_string(UP_KEY, true);
      if (newDir === 1) client.handle_key_string(DOWN_KEY, true);

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
      if (currentTouchDir === -1) client.handle_key_string(UP_KEY, false);
      if (currentTouchDir === 1) client.handle_key_string(DOWN_KEY, false);
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
        // True while a touch or mouse press is held on a control.
        isPointerDown = true;
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

  document.addEventListener("touchend", () => {
    if (isPointerDown) {
      handlePointerEnd();
    }
  });

  document.addEventListener("touchcancel", () => {
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

  document.addEventListener("mouseup", () => {
    if (isPointerDown) {
      handlePointerEnd();
    }
  });
}
