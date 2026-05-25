import { useState, useRef } from "react";
import { keyboardEventToShortcut, prettyHotkey } from "../lib/ipc";

interface Props {
  value: string;
  onChange: (next: string) => void;
}

export function HotkeyPicker({ value, onChange }: Props) {
  const [capturing, setCapturing] = useState(false);
  const inputRef = useRef<HTMLDivElement>(null);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      setCapturing(false);
      return;
    }

    const shortcut = keyboardEventToShortcut(e.nativeEvent);
    if (shortcut) {
      onChange(shortcut);
      setCapturing(false);
    }
  }

  return (
    <div
      ref={inputRef}
      tabIndex={0}
      onClick={() => setCapturing(true)}
      onBlur={() => setCapturing(false)}
      onKeyDown={handleKeyDown}
      className={
        "px-4 py-2 rounded-md border cursor-text font-mono text-sm select-none transition-colors " +
        (capturing
          ? "bg-[var(--color-accent)]/10 border-[var(--color-accent)] text-[var(--color-accent)]"
          : "bg-[var(--color-panel)] border-[var(--color-border)] hover:border-[var(--color-muted)]")
      }
    >
      {capturing ? (
        <span>Press a key combo... (Esc to cancel)</span>
      ) : (
        <span>{prettyHotkey(value) || "Click to set hotkey"}</span>
      )}
    </div>
  );
}
