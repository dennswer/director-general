import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

const BAR_COUNT = 14;

/// The tiny always-on-top window shown while recording. Pulsing bars give the
/// user visual confirmation that input is being captured; the X stops it.
///
/// We don't have direct mic access here (cpal owns it in Rust), so the bars
/// are a synthetic animation rather than a real spectrogram. Good enough for
/// "yes, it's listening" feedback.
export function Overlay() {
  const [bars, setBars] = useState<number[]>(() =>
    Array.from({ length: BAR_COUNT }, () => 0.2),
  );
  const phaseRef = useRef(0);

  useEffect(() => {
    const id = setInterval(() => {
      phaseRef.current += 0.18;
      const phase = phaseRef.current;
      setBars(
        Array.from({ length: BAR_COUNT }, (_, i) => {
          // Smoothly varying pseudo-equalizer values per bar.
          const a = Math.sin(phase + i * 0.6);
          const b = Math.cos(phase * 0.7 + i * 0.3);
          const norm = (a * a + b * b) / 2; // 0..1
          // Boost the centre a bit so it looks like "louder middle frequencies".
          const centre = 1 - Math.abs(i - (BAR_COUNT - 1) / 2) / ((BAR_COUNT - 1) / 2);
          const v = 0.18 + norm * (0.55 + 0.35 * centre);
          return Math.min(1, v);
        }),
      );
    }, 70);
    return () => clearInterval(id);
  }, []);

  function stop() {
    // Same code path as a real hotkey press — toggles the state machine.
    invoke("trigger_hotkey").catch(() => {});
  }

  return (
    <div
      style={{
        height: "100vh",
        width: "100vw",
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "0 10px",
        background: "rgba(15, 17, 23, 0.92)",
        border: "1px solid rgba(232, 69, 69, 0.55)",
        borderRadius: 10,
        boxShadow: "0 6px 18px rgba(0,0,0,0.45)",
        // Make the title bar area itself draggable by the user.
        // (data-tauri-drag-region lets the user reposition the overlay.)
      }}
      data-tauri-drag-region
    >
      <div
        style={{
          flex: 1,
          height: 36,
          display: "flex",
          alignItems: "center",
          gap: 2,
        }}
      >
        {bars.map((v, i) => (
          <div
            key={i}
            style={{
              width: 3,
              height: `${Math.round(v * 100)}%`,
              background: "linear-gradient(180deg, #ff6b6b 0%, #e84545 100%)",
              borderRadius: 2,
              transition: "height 60ms linear",
            }}
          />
        ))}
      </div>
      <button
        onClick={stop}
        title="Stop recording"
        style={{
          flex: "0 0 auto",
          width: 32,
          height: 32,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "rgba(232, 69, 69, 0.15)",
          border: "1px solid rgba(232, 69, 69, 0.6)",
          borderRadius: 8,
          color: "#ff8585",
          cursor: "pointer",
          padding: 0,
        }}
      >
        <X size={18} />
      </button>
    </div>
  );
}
