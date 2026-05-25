// Take screenshots of every tab using Playwright + a mock for window.__TAURI_INTERNALS__.
// Requires Vite dev server (http://localhost:1420) to be running.

import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

const outDir = join(process.cwd(), "screenshots");
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 900, height: 640 } });
const page = await ctx.newPage();

// Mock the Tauri internals so invoke() returns fake data and listen() is a no-op.
// This lets us screenshot the React UI in a plain browser, no Tauri runtime needed.
await page.addInitScript(() => {
  const fakeHistory = [
    {
      id: "demo-1",
      timestamp: new Date().toISOString(),
      text: "Hello, this is a test of voice transcription using Whisper. The quick brown fox jumps over the lazy dog.",
      audio_path: "C:\\Users\\Dennswer\\AppData\\Roaming\\com.voiceeee.app\\tmp\\rec_20260408-200042.wav",
      model: "ggml-large-v3-turbo",
      duration_ms: 1710,
    },
    {
      id: "demo-2",
      timestamp: new Date(Date.now() - 5 * 60_000).toISOString(),
      text: "Bună, te rog scrie un email către echipa de marketing despre lansarea de luna viitoare.",
      audio_path: "C:\\Users\\Dennswer\\AppData\\Roaming\\com.voiceeee.app\\tmp\\rec_20260408-195530.wav",
      model: "ggml-large-v3-turbo",
      duration_ms: 2412,
    },
    {
      id: "demo-3",
      timestamp: new Date(Date.now() - 60 * 60_000).toISOString(),
      text: "Adaugă în calendar întâlnirea cu clientul vineri la ora trei.",
      audio_path: "C:\\Users\\Dennswer\\AppData\\Roaming\\com.voiceeee.app\\tmp\\rec_20260408-190021.wav",
      model: "ggml-large-v3-turbo",
      duration_ms: 1985,
    },
  ];

  // Tauri 2 invoke() goes through window.__TAURI_INTERNALS__.invoke
  // and event listen() goes through __TAURI_INTERNALS__.transformCallback
  (window).__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      switch (cmd) {
        case "plugin:event|listen":
          // listen() returns a numeric handler id; we never fire events
          return Promise.resolve(0);
        case "plugin:event|unlisten":
          return Promise.resolve();
        case "ping":
          return "pong";
        case "get_mode":
          return "idle";
        case "get_settings":
          return {
            hotkey: "Backquote",
            model_path:
              "E:\\Proiecte Claudele GG\\voiceeee\\models\\ggml-large-v3-turbo.bin",
            language: null,
            auto_paste: true,
            max_history: 200,
          };
        case "update_settings":
          return undefined;
        case "get_history":
          return fakeHistory;
        case "delete_history_entry":
        case "clear_history":
        case "copy_history_entry":
          return undefined;
        default:
          console.warn("[mock] unknown command:", cmd, args);
          return undefined;
      }
    },
    transformCallback: (cb) => {
      // Generate a stable id; we never call the callback.
      return Math.floor(Math.random() * 1e9);
    },
    metadata: { currentWindow: { label: "main" } },
  };
});

await page.goto("http://localhost:1420", { waitUntil: "networkidle" });

const tabs = ["Status", "Settings", "History"];
for (const tab of tabs) {
  await page.getByRole("button", { name: tab, exact: true }).click();
  await page.waitForTimeout(200);
  const file = join(outDir, `phase6-${tab.toLowerCase()}.png`);
  await page.screenshot({ path: file, fullPage: false });
  console.log("saved", file);
}

await browser.close();
