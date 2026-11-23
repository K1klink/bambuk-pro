import { Card, Page, Reports, Stat, Switch } from "../components/ui";
import { POTATO_LEVELS } from "../api";
import { useApp } from "../store";

export default function Home() {
  const { snap, live, setApplied, applyPreset, setProfile } = useApp();
  if (!snap) return null;
  const f = live?.frame; const hw = live?.hw; const running = snap.game.running;
  const gfx = snap.presets.find((p) => p.id === snap.activeGraphics)?.name ?? "Свой набор";
  const col = snap.presets.find((p) => p.id === snap.activeColor)?.name ?? "Свой набор";
  const colorPresets = snap.presets.filter((p) => p.kind === "color" && p.builtin);
  return (
    <Page title="Главная" sub="Статус игры, активные профили и быстрое переключение."
      aside={<label className="apply"><span>{snap.applied ? "Профиль применён" : "Профиль выключен"}</span><Switch on={snap.applied} onChange={setApplied} label="Применить профиль" /></label>}>
      {snap.engine.recovered && <div className="notice">{snap.engine.recovered}</div>}
      <div className="grid g3">
        <Card title="ROB RUN"><p className="big">{running ? "Запущена" : "Не запущена"}</p>
          <p className="muted">{running ? (snap.game.hasWindow ? `PID ${snap.game.pid}, окно найдено` : `PID ${snap.game.pid}, окно ещё не найдено`) : "Запустите игру — FPS появится автоматически."}</p></Card>
        <Card title="Графика"><p className="big">{gfx}</p><p className="muted">Разрешение рендера: {Math.round((snap.engine.currentMode && snap.engine.nativeMode ? snap.engine.currentMode.w / snap.engine.nativeMode.w : 1) * 100)}%</p></Card>
        <Card title="Цвет"><p className="big">{col}</p><p className="muted">{snap.engine.colorActive ? "Матрица цвета активна" : "Цвет не изменён"}</p></Card>
      </div>
      <div className="grid g5">
        <Stat label="FPS" value={f && f.frames ? f.fps.toFixed(0) : "—"} />
        <Stat label="1% Low" value={f && f.frames ? f.onePercentLow.toFixed(0) : "—"} />
        <Stat label="Frame time" value={f && f.frames ? f.avgFrameMs.toFixed(1) : "—"} unit=" мс" />
        <Stat label="GPU" value={hw?.gpuPercent != null ? String(hw.gpuPercent) : "—"} unit="%" />
        <Stat label="CPU" value={hw?.cpuPercent != null ? hw.cpuPercent.toFixed(0) : "—"} unit="%" />
      </div>
      <div className="grid g2">
        <Card title="Быстрый Potato" hint="Применяется, когда профиль включён.">
          <div className="chips">{POTATO_LEVELS.map((l) => {
            const id = `builtin-gfx-${l.name.toLowerCase().replace(" ", "-")}`;
            return <button key={l.id} className={`chip ${snap.activeGraphics === id ? "on" : ""}`} onClick={() => applyPreset(id)}>{l.name}</button>;
          })}</div>
        </Card>
        <Card title="Быстрый цвет">
          <div className="chips">{colorPresets.map((p) => <button key={p.id} className={`chip ${snap.activeColor === p.id ? "on" : ""}`} onClick={() => applyPreset(p.id)}>{p.name}</button>)}
            <button className="chip" onClick={() => setProfile((p) => ({ ...p, color: { ...p.color, saturation: 0, vibrance: 0, contrast: 0, brightness: 0, gamma: 1, temperature: 0, tint: 0, sharpness: 0 } }))}>Сбросить</button></div>
        </Card>
      </div>
      <Card title="Что применилось в последний раз" hint="Статус «Применено» ставится только после проверки системой."><Reports reports={snap.reports} /></Card>
    </Page>
  );
}
