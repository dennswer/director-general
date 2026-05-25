import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { StatusIndicator } from "./components/StatusIndicator";
import { SettingsView } from "./components/Settings";
import { HistoryList } from "./components/HistoryList";
import { onModeChanged, ipc, type Mode } from "./lib/ipc";
import { Activity, Settings as SettingsIcon, History, Briefcase } from "lucide-react";

type Tab = "status" | "settings" | "history";

function App() {
  const [tab, setTab] = useState<Tab>("status");
  const [mode, setMode] = useState<Mode>("idle");

  useEffect(() => {
    ipc.getMode().then(setMode);
    const unlistenMode = onModeChanged(setMode);
    // Tray menu emits "nav" events with the target tab name
    const unlistenNav = listen<string>("nav", (e) => {
      const target = e.payload as Tab;
      if (target === "status" || target === "settings" || target === "history") {
        setTab(target);
      }
    });
    return () => {
      unlistenMode.then((un) => un());
      unlistenNav.then((un) => un());
    };
  }, []);

  return (
    <div className="flex h-screen w-screen bg-[var(--color-bg)] text-[var(--color-text)]">
      <aside className="w-56 border-r border-[var(--color-border)] p-4 flex flex-col gap-1">
        <div className="text-base font-semibold mb-6 px-2 flex items-center gap-2">
          <Briefcase size={18} className="text-[var(--color-accent)]" />
          <span>Director General</span>
          <ModeDot mode={mode} />
        </div>
        <NavButton
          active={tab === "status"}
          onClick={() => setTab("status")}
          icon={<Activity size={16} />}
        >
          Status
        </NavButton>
        <NavButton
          active={tab === "settings"}
          onClick={() => setTab("settings")}
          icon={<SettingsIcon size={16} />}
        >
          Settings
        </NavButton>
        <NavButton
          active={tab === "history"}
          onClick={() => setTab("history")}
          icon={<History size={16} />}
        >
          History
        </NavButton>
        <div className="mt-auto text-[10px] text-[var(--color-muted)] px-2 leading-relaxed">
          local · whisper<br />by Bossnet
        </div>
      </aside>

      <main className="flex-1 p-8 overflow-auto">
        {tab === "status" && <StatusIndicator />}
        {tab === "settings" && <SettingsView />}
        {tab === "history" && <HistoryList />}
      </main>
    </div>
  );
}

function ModeDot({ mode }: { mode: Mode }) {
  const color =
    mode === "recording"
      ? "bg-[var(--color-recording)]"
      : mode === "transcribing"
      ? "bg-[var(--color-accent)]"
      : "bg-[var(--color-muted)]";
  const animate = mode === "recording" ? " animate-pulse" : "";
  return <span className={`ml-auto w-2 h-2 rounded-full ${color}${animate}`} />;
}

function NavButton({
  active,
  onClick,
  children,
  icon,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  icon?: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={
        "text-left px-3 py-2 rounded-md transition-colors flex items-center gap-2 " +
        (active
          ? "bg-[var(--color-panel)] text-[var(--color-text)]"
          : "text-[var(--color-muted)] hover:bg-[var(--color-panel)]/60")
      }
    >
      {icon}
      <span>{children}</span>
    </button>
  );
}

export default App;
