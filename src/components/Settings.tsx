import { useEffect, useState } from "react";
import { ipc, type Settings as SettingsT } from "../lib/ipc";
import { HotkeyPicker } from "./HotkeyPicker";
import {
  enable as enableAutostart,
  disable as disableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import {
  Mic,
  Languages,
  Cpu,
  Keyboard,
  ClipboardPaste,
  PowerCircle,
  Eye,
  BookText,
} from "lucide-react";

export function SettingsView() {
  const [settings, setSettings] = useState<SettingsT | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [error, setError] = useState<string>("");
  const [autostart, setAutostart] = useState<boolean>(false);

  useEffect(() => {
    ipc.getSettings().then(setSettings);
    ipc.listInputDevices().then(setDevices).catch(() => {});
    isAutostartEnabled().then(setAutostart).catch(() => {});
  }, []);

  async function toggleAutostart(next: boolean) {
    try {
      if (next) await enableAutostart();
      else await disableAutostart();
      setAutostart(next);
    } catch (e) {
      setError(String(e));
    }
  }

  if (!settings) {
    return (
      <div className="text-[var(--color-muted)]">Loading settings...</div>
    );
  }

  async function save(next: SettingsT) {
    setSaving(true);
    setError("");
    try {
      await ipc.updateSettings(next);
      setSettings(next);
      setSavedAt(Date.now());
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function update<K extends keyof SettingsT>(key: K, value: SettingsT[K]) {
    if (!settings) return;
    save({ ...settings, [key]: value });
  }

  const modelName = settings.model_path.split(/[\\/]/).pop() || settings.model_path;
  const isCapsLock = settings.hotkey === "CapsLock";

  return (
    <div className="max-w-2xl">
      <h2 className="text-2xl font-semibold mb-2">Settings</h2>
      <p className="text-[var(--color-muted)] text-sm mb-8">
        Changes save automatically.
      </p>

      <div className="space-y-8">
        <Field
          icon={<Keyboard size={16} />}
          label="Hotkey"
          hint={
            isCapsLock
              ? "Caps Lock is intercepted via a low-level hook — it will start/stop recording without toggling the caps state."
              : "Press the key combo you want to use to start/stop recording."
          }
        >
          <HotkeyPicker
            value={settings.hotkey}
            onChange={(v) => update("hotkey", v)}
          />
        </Field>

        <Field
          icon={<Mic size={16} />}
          label="Microphone"
          hint="The input device used to capture audio. 'Default' follows the Windows default."
        >
          <select
            value={settings.input_device ?? ""}
            onChange={(e) =>
              update(
                "input_device",
                e.target.value === "" ? null : e.target.value,
              )
            }
            className="px-3 py-2 rounded-md bg-[var(--color-panel)] border border-[var(--color-border)] text-sm w-full"
          >
            <option value="">Default (system input)</option>
            {devices.map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        </Field>

        <Field
          icon={<Languages size={16} />}
          label="Language"
          hint="Auto-detect handles mixed Romanian + English well. Use a forced locale only if auto-detect picks the wrong one."
        >
          <input
            type="text"
            list="dg-langs"
            value={settings.language ?? ""}
            placeholder="auto"
            onChange={(e) =>
              update(
                "language",
                e.target.value.trim() === "" ? null : e.target.value.trim(),
              )
            }
            className="px-3 py-2 rounded-md bg-[var(--color-panel)] border border-[var(--color-border)] text-sm w-full font-mono"
          />
          <datalist id="dg-langs">
            <option value="ro" />
            <option value="en" />
            <option value="de" />
            <option value="fr" />
            <option value="it" />
            <option value="es" />
            <option value="ru" />
          </datalist>
        </Field>

        <Field
          icon={<Cpu size={16} />}
          label="Whisper model"
          hint="Turbo is faster, large-v3 is slightly more accurate."
        >
          <div className="flex flex-col gap-2">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="radio"
                name="model"
                checked={modelName.includes("turbo")}
                onChange={() =>
                  update(
                    "model_path",
                    settings.model_path.replace(
                      /ggml-large-v3(\.bin)?$/,
                      "ggml-large-v3-turbo.bin",
                    ),
                  )
                }
              />
              <span className="text-sm">large-v3-turbo (1.5 GB, ~5x faster)</span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="radio"
                name="model"
                checked={!modelName.includes("turbo")}
                onChange={() =>
                  update(
                    "model_path",
                    settings.model_path.replace(
                      /ggml-large-v3-turbo\.bin$/,
                      "ggml-large-v3.bin",
                    ),
                  )
                }
              />
              <span className="text-sm">large-v3 (3 GB, max quality)</span>
            </label>
          </div>
        </Field>

        <Field
          icon={<BookText size={16} />}
          label="Common words"
          hint="Comma- or newline-separated jargon, brand names, technical terms. Fed to Whisper as initial_prompt so it transcribes them consistently (e.g. 'API, WooCommerce, Bossnet')."
        >
          <textarea
            value={settings.common_words}
            onChange={(e) => update("common_words", e.target.value)}
            placeholder="API, WordPress, WooCommerce, Bossnet, Tauri"
            rows={3}
            className="px-3 py-2 rounded-md bg-[var(--color-panel)] border border-[var(--color-border)] text-sm w-full font-mono leading-relaxed"
          />
        </Field>

        <Field
          icon={<ClipboardPaste size={16} />}
          label="Auto-paste"
          hint="After transcription, simulate Ctrl+V to paste into the focused window."
        >
          <label className="inline-flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.auto_paste}
              onChange={(e) => update("auto_paste", e.target.checked)}
              className="w-4 h-4"
            />
            <span className="text-sm">
              {settings.auto_paste
                ? "On — text gets pasted into the active input"
                : "Off — text is only put on the clipboard"}
            </span>
          </label>
        </Field>

        <Field
          icon={<Eye size={16} />}
          label="Show recording overlay"
          hint="A small always-on-top equalizer with a stop button appears while recording."
        >
          <label className="inline-flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.show_overlay}
              onChange={(e) => update("show_overlay", e.target.checked)}
              className="w-4 h-4"
            />
            <span className="text-sm">
              {settings.show_overlay ? "Visible" : "Hidden"}
            </span>
          </label>
        </Field>

        <Field
          icon={<PowerCircle size={16} />}
          label="Launch on Windows startup"
          hint="The app will start minimized to the tray when you log in."
        >
          <label className="inline-flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={autostart}
              onChange={(e) => toggleAutostart(e.target.checked)}
              className="w-4 h-4"
            />
            <span className="text-sm">
              {autostart ? "Enabled" : "Disabled"}
            </span>
          </label>
        </Field>
      </div>

      <div className="mt-8 text-xs text-[var(--color-muted)] h-4">
        {saving && "Saving..."}
        {!saving && savedAt && `Saved ${new Date(savedAt).toLocaleTimeString()}`}
        {error && <span className="text-red-400">Error: {error}</span>}
      </div>
    </div>
  );
}

function Field({
  icon,
  label,
  hint,
  children,
}: {
  icon?: React.ReactNode;
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <div className="text-sm font-medium mb-1 flex items-center gap-2">
        {icon && <span className="text-[var(--color-muted)]">{icon}</span>}
        <span>{label}</span>
      </div>
      {hint && (
        <div className="text-xs text-[var(--color-muted)] mb-2">{hint}</div>
      )}
      {children}
    </div>
  );
}
