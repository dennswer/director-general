import { useEffect, useState } from "react";
import {
  humanTimestamp,
  ipc,
  onTranscriptionComplete,
  type HistoryEntry,
} from "../lib/ipc";
import { Copy, Trash2, AppWindow, Check } from "lucide-react";

export function HistoryList() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [copiedId, setCopiedId] = useState<string | null>(null);

  async function refresh() {
    const data = await ipc.getHistory();
    // Most recent first
    setEntries([...data].reverse());
  }

  useEffect(() => {
    refresh();
    const unlisten = onTranscriptionComplete(() => refresh());
    return () => {
      unlisten.then((un) => un());
    };
  }, []);

  function toggleExpand(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setExpanded(next);
  }

  async function handleCopy(id: string) {
    await ipc.copyHistoryEntry(id);
    setCopiedId(id);
    setTimeout(() => setCopiedId((c) => (c === id ? null : c)), 1500);
  }

  async function handleDelete(id: string) {
    await ipc.deleteHistoryEntry(id);
    refresh();
  }

  async function handleClearAll() {
    if (!confirm("Delete all transcriptions from history?")) return;
    await ipc.clearHistory();
    refresh();
  }

  return (
    <div className="max-w-3xl">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-semibold">History</h2>
        {entries.length > 0 && (
          <button
            onClick={handleClearAll}
            className="text-xs text-[var(--color-muted)] hover:text-red-400 transition-colors"
          >
            Clear all
          </button>
        )}
      </div>

      {entries.length === 0 ? (
        <p className="text-[var(--color-muted)] text-sm">
          No transcriptions yet. Press the hotkey to record your first one.
        </p>
      ) : (
        <ul className="space-y-2">
          {entries.map((e) => {
            const isExpanded = expanded.has(e.id);
            const time = new Date(e.timestamp);
            const friendly = humanTimestamp(e.timestamp);
            return (
              <li
                key={e.id}
                className="bg-[var(--color-panel)] border border-[var(--color-border)] rounded-lg overflow-hidden"
              >
                <div
                  className="p-3 flex items-center gap-3 cursor-pointer hover:bg-white/5"
                  onClick={() => toggleExpand(e.id)}
                >
                  <div className="text-xs text-[var(--color-muted)] font-mono whitespace-nowrap min-w-[7rem]">
                    {friendly}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm truncate">
                      {e.text || <span className="italic text-[var(--color-muted)]">(empty)</span>}
                    </div>
                    {e.window_title && (
                      <div className="text-[10px] text-[var(--color-muted)] mt-0.5 flex items-center gap-1 truncate">
                        <AppWindow size={11} />
                        <span className="truncate">{e.window_title}</span>
                      </div>
                    )}
                  </div>
                  <button
                    onClick={(ev) => {
                      ev.stopPropagation();
                      handleCopy(e.id);
                    }}
                    className="text-xs px-2 py-1 rounded bg-[var(--color-bg)] border border-[var(--color-border)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)] transition-colors flex items-center gap-1"
                    title="Copy text"
                  >
                    {copiedId === e.id ? (
                      <>
                        <Check size={12} /> Copied
                      </>
                    ) : (
                      <>
                        <Copy size={12} /> Copy
                      </>
                    )}
                  </button>
                  <button
                    onClick={(ev) => {
                      ev.stopPropagation();
                      handleDelete(e.id);
                    }}
                    className="text-xs px-2 py-1 rounded bg-[var(--color-bg)] border border-[var(--color-border)] hover:border-red-500 hover:text-red-400 transition-colors flex items-center gap-1"
                    title="Delete entry"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
                {isExpanded && (
                  <div className="px-3 pb-3 pt-1 border-t border-[var(--color-border)]">
                    <div className="text-sm whitespace-pre-wrap">
                      {e.text || (
                        <span className="italic text-[var(--color-muted)]">
                          (empty transcription)
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-[var(--color-muted)] mt-2 font-mono">
                      {e.model} · {e.duration_ms}ms · {time.toLocaleString()}
                    </div>
                    {e.window_title && (
                      <div className="text-xs text-[var(--color-muted)] mt-1 flex items-center gap-1">
                        <AppWindow size={12} />
                        <span>{e.window_title}</span>
                      </div>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
