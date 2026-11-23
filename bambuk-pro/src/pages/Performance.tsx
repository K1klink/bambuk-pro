import { Card, Page, Stat } from "../components/ui";
import { api, FrameStats } from "../api";
import { useApp } from "../store";

function Spark({ data }: { data: number[] }) {
  const w = 560, h = 120; const max = Math.max(33, ...data) * 1.1;
  const pts = data.map((v, i) => `${(i / Math.max(1, data.length - 1)) * w},${h - (v / max) * h}`).join(" ");
  const y16 = h - (16.67 / max) * h, y33 = h - (33.3 / max) * h;
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="spark" role="img" aria-label="График времени кадра">
      <line x1="0" x2={w} y1={y16} y2={y16} className="guide" /><text x="4" y={y16 - 3}>16.7 мс (60 FPS)</text>
      {y33 > 10 && <><line x1="0" x2={w} y1={y33} y2={y33} className="guide" /><text x="4" y={y33 - 3}>33.3 мс (30 FPS)</text></>}
      <polyline points={pts} fill="none" />
    </svg>
  );
}

const delta = (a: number, b: number, higherBetter: boolean) => {
  if (!a) return { t: "—", good: undefined as boolean | undefined };
  const d = ((b - a) / a) * 100; const good = higherBetter ? d >= 0 : d <= 0;
  return { t: `${d >= 0 ? "+" : ""}${d.toFixed(1)}%`, good };
};

export default function Performance() {
  const { live, history, run, toast } = useApp();
  const f: FrameStats | undefined = live?.frame; const hw = live?.hw; const base = live?.baseline;
  const has = !!f && f.frames > 0;
  const rows: [string, keyof FrameStats, boolean, number][] = [["FPS", "fps", true, 0], ["1% Low", "onePercentLow", true, 0], ["Средний frame time, мс", "avgFrameMs", false, 2], ["99-й перцентиль, мс", "p99FrameMs", false, 2]];
  return (
    <Page title="Performance" sub="Реальные кадры из событий Windows (ETW) через PresentMon. Без инъекции в игру, без выдуманных цифр.">
      {live?.perf.note && <div className="notice">{live.perf.note}</div>}
      {!live?.game.running && <div className="notice soft">Запустите ROB RUN — мониторинг стартует автоматически.</div>}
      <div className="grid g5">
        <Stat label="FPS" value={has ? f!.fps.toFixed(0) : "—"} />
        <Stat label="1% Low" value={has ? f!.onePercentLow.toFixed(0) : "—"} />
        <Stat label="Frame time" value={has ? f!.lastFrameMs.toFixed(1) : "—"} unit=" мс" />
        <Stat label="GPU" value={hw?.gpuPercent != null ? String(hw.gpuPercent) : "—"} unit="%" tone={hw?.gpuPercent != null && hw.gpuPercent > 97 ? "warn" : undefined} />
        <Stat label="CPU" value={hw?.cpuPercent != null ? hw.cpuPercent.toFixed(0) : "—"} unit="%" />
      </div>
      <Card title="Время кадра" hint="Последние ~80 секунд. Пики вверх — рывки."><Spark data={history} /></Card>
      <div className="grid g2">
        <Card title="Видеокарта" hint={hw?.gpuName ?? "NVML недоступен"}>
          <p className="kv"><span>Температура GPU</span><b>{hw?.gpuTempC != null ? `${hw.gpuTempC} °C` : "—"}</b></p>
          <p className="kv"><span>Частота ядра</span><b>{hw?.gpuClockMhz != null ? `${hw.gpuClockMhz} МГц` : "—"}</b></p>
          <p className="kv"><span>Видеопамять</span><b>{hw?.vramUsedMb != null ? `${hw.vramUsedMb} / ${hw.vramTotalMb} МБ` : "—"}</b></p>
          {hw?.gpuNote && <p className="muted">{hw.gpuNote}</p>}
          <p className="muted">Температура процессора не показывается: для неё нужен сторонний драйвер ядра, а он может конфликтовать с античитом.</p>
        </Card>
        <Card title="До и после" hint="Запомните показатели, смените пресет, подождите 15–20 секунд и сравните.">
          <div className="row">
            <button className="btn" onClick={() => run(api.setBaseline(), "Показатели «до» сохранены")}>Запомнить как «до»</button>
            <button className="btn ghost" disabled={!base} onClick={() => api.clearBaseline().then(() => toast("Сравнение сброшено"))}>Сбросить</button>
          </div>
          {base && has ? (
            <table className="cmp"><thead><tr><th /><th>До</th><th>Сейчас</th><th>Δ</th></tr></thead><tbody>
              {rows.map(([name, key, hb, d]) => { const a = base[key] as number, b = f![key] as number; const x = delta(a, b, hb);
                return <tr key={key}><td>{name}</td><td>{a.toFixed(d)}</td><td>{b.toFixed(d)}</td><td className={x.good === undefined ? "" : x.good ? "good" : "bad"}>{x.t}</td></tr>; })}
            </tbody></table>
          ) : <p className="muted">{base ? "Нет текущих кадров." : "Сравнение появится после сохранения «до»."}</p>}
        </Card>
      </div>
    </Page>
  );
}

