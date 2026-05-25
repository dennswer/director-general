import { useEffect, useState } from "react";
import {
  ipc,
  onModeChanged,
  onTranscriptionComplete,
  onTranscriptionError,
  prettyHotkey,
  type Mode,
  type Settings,
} from "../lib/ipc";
import { Mic, Loader2, MicOff } from "lucide-react";

export function StatusIndicator() {
  const [mode, setMode] = useState<Mode>("idle");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [lastText, setLastText] = useState<string>("");
  const [lastError, setLastError] = useState<string>("");

  useEffect(() => {
    ipc.getMode().then(setMode);
    ipc.getSettings().then(setSettings);

    const unlistens: Array<Promise<() => void>> = [
      onModeChanged((m) => setMode(m)),
      onTranscriptionComplete((text) => {
        setLastText(text);
        setLastError("");
      }),
      onTranscriptionError((msg) => setLastError(msg)),
    ];

    return () => {
      unlistens.forEach((p) => p.then((un) => un()));
    };
  }, []);

  return (
    <div className="flex flex-col items-center justify-center h-full gap-6">
      <ModeOrb mode={mode} />

      <div className="text-center">
        {mode === "idle" && settings && (
          <p className="text-[var(--color-muted)] text-sm">
            Press{" "}
            <kbd className="px-2 py-1 bg-[var(--color-panel)] border border-[var(--color-border)] rounded font-mono text-xs">
              {prettyHotkey(settings.hotkey)}
            </kbd>{" "}
            to start recording
          </p>
        )}
        {mode === "recording" && (
          <p className="text-[var(--color-recording)] text-sm font-medium">
            Recording... press hotkey again to stop
          </p>
        )}
        {mode === "transcribing" && (
          <p className="text-[var(--color-accent)] text-sm font-medium">
            Transcribing with Whisper...
          </p>
        )}
      </div>

      {lastText && (
        <div className="max-w-xl w-full mt-4 p-4 bg-[var(--color-panel)] border border-[var(--color-border)] rounded-lg">
          <div className="text-xs text-[var(--color-muted)] mb-1">
            Last transcription
          </div>
          <div className="text-sm whitespace-pre-wrap">{lastText}</div>
        </div>
      )}

      {lastError && (
        <div className="max-w-xl w-full mt-4 p-4 bg-red-950/40 border border-red-900/60 rounded-lg">
          <div className="text-xs text-red-400 mb-1">Error</div>
          <div className="text-sm font-mono text-red-200 break-all">
            {lastError}
          </div>
        </div>
      )}
    </div>
  );
}

function ModeOrb({ mode }: { mode: Mode }) {
  const base =
    "w-32 h-32 rounded-full border flex items-center justify-center transition-all duration-200";
  if (mode === "recording") {
    return (
      <div
        className={
          base +
          " bg-[var(--color-recording)]/15 border-[var(--color-recording)] animate-pulse"
        }
      >
        <Mic className="text-[var(--color-recording)]" size={42} />
      </div>
    );
  }
  if (mode === "transcribing") {
    return (
      <div
        className={
          base +
          " bg-[var(--color-accent)]/10 border-[var(--color-accent)]"
        }
      >
        <Loader2
          className="text-[var(--color-accent)] animate-spin"
          size={42}
        />
      </div>
    );
  }
  return (
    <div
      className={
        base + " bg-[var(--color-panel)] border-[var(--color-border)]"
      }
    >
      <MicOff className="text-[var(--color-muted)]" size={36} />
    </div>
  );
}
