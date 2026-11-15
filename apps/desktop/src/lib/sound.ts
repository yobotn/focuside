// A short, quiet chime when a session finishes or waits (Settings > Sound). Made with Web
// Audio, so the app carries no sound file. A failure plays a lower note.
let ctx: AudioContext | null = null;

export function playChime(low = false) {
  try {
    ctx ??= new AudioContext();
    const c = ctx;
    if (c.state === "suspended") c.resume().catch(() => {});
    const now = c.currentTime;
    const notes = low ? [440, 330] : [660, 880];
    notes.forEach((freq, i) => {
      const osc = c.createOscillator();
      const gain = c.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      const start = now + i * 0.12;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.08, start + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.25);
      osc.connect(gain).connect(c.destination);
      osc.start(start);
      osc.stop(start + 0.3);
    });
  } catch {
    // Sound is optional.
  }
}
