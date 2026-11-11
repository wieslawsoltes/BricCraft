// Original procedural sound: a short plastic snap, generated after a user gesture.
export function installAudio() {
    let context;
    globalThis.briccraftSound = kind => {
        try {
            context ??= new (window.AudioContext || window.webkitAudioContext)();
            if (context.state === 'suspended') void context.resume();
            const now = context.currentTime;
            const oscillator = context.createOscillator();
            const gain = context.createGain();
            oscillator.type = 'triangle';
            oscillator.frequency.setValueAtTime(kind === 'remove' ? 380 : kind === 'build' ? 880 : 680, now);
            oscillator.frequency.exponentialRampToValueAtTime(140, now + 0.055);
            gain.gain.setValueAtTime(0.035, now);
            gain.gain.exponentialRampToValueAtTime(0.001, now + 0.08);
            oscillator.connect(gain); gain.connect(context.destination);
            oscillator.start(now); oscillator.stop(now + 0.09);
            oscillator.onended = () => { oscillator.disconnect(); gain.disconnect(); };
        } catch { /* Sound is optional; construction remains available without an audio device. */ }
    };
}
