import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useApp } from "./store";
import Home from "./pages/Home";
import Potato from "./pages/Potato";
import ColorTuner from "./pages/ColorTuner";
import Filters from "./pages/Filters";
import Performance from "./pages/Performance";
import Presets from "./pages/Presets";
import Settings from "./pages/Settings";

const TABS = [
  { id: "home", name: "Главная", el: <Home /> },
  { id: "potato", name: "Potato Mode", el: <Potato /> },
  { id: "color", name: "ColorTuner", el: <ColorTuner /> },
  { id: "filters", name: "Filters", el: <Filters /> },
  { id: "perf", name: "Performance", el: <Performance /> },
  { id: "presets", name: "Presets", el: <Presets /> },
  { id: "settings", name: "Settings", el: <Settings /> },
] as const;

const win = () => { try { return getCurrentWindow(); } catch { return null; } };

export default function App() {
  const [tab, setTab] = useState<(typeof TABS)[number]["id"]>("home");
  const { snap, live, toasts } = useApp();
  const game = live?.game ?? snap?.game;
  return (
    <div className="shell">
      <div className="titlebar" data-tauri-drag-region>
        <span className="brand" data-tauri-drag-region>Bambuk <b>Pro</b></span>
        <div className="wc">
          <button onClick={() => win()?.minimize()} aria-label="Свернуть">—</button>
          <button onClick={() => win()?.toggleMaximize()} aria-label="Развернуть">▢</button>
          <button className="x" onClick={() => win()?.close()} aria-label="Закрыть">✕</button>
        </div>
      </div>
      <nav className="side" aria-label="Разделы">
        {TABS.map((t) => (
          <button key={t.id} className={tab === t.id ? "active" : ""} onClick={() => setTab(t.id)}>{t.name}</button>
        ))}
        <div className="side-foot">
          <span className={`dot ${game?.running ? "on" : ""}`} />
          <span>{game?.running ? "ROB RUN запущена" : "ROB RUN не запущена"}</span>
        </div>
      </nav>
      <main className="main">
        {!snap ? <p className="muted pad">Загрузка…</p> : TABS.find((t) => t.id === tab)!.el}
      </main>
      <div className="toasts" aria-live="polite">{toasts.map((t) => <div key={t.id} className={`toast ${t.kind}`}>{t.text}</div>)}</div>
    </div>
  );
}
