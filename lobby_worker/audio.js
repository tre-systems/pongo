// Sound effects using the Web Audio API.
const AudioContext = window.AudioContext || window.webkitAudioContext;
let audioCtx = null;

export function initAudio() {
  if (!audioCtx) {
    audioCtx = new AudioContext();
  }
}

export function playSound(type) {
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
