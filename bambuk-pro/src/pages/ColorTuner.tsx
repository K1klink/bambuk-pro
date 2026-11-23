import { Card, Page, Scene, Slider } from "../components/ui";
import type { ColorParams } from "../api";
import { fullMatrix, svgValues } from "../colorMath";
import { useApp } from "../store";

const DEF: ColorParams = { saturation: 0, vibrance: 0, contrast: 0, brightness: 0, gamma: 1, temperature: 0, tint: 0, sharpness: 0 };

export default function ColorTuner() {
  const { snap, setProfile, applyPreset } = useApp();
  if (!snap) return null;
  const c = snap.profile.color;
  const set = (k: keyof ColorParams) => (v: number) => setProfile((p) => ({ ...p, color: { ...p.color, [k]: v } }));
  const m = fullMatrix(c, snap.profile.filters);
  const presets = snap.presets.filter((p) => p.kind === "color" && p.builtin);
  const rep = snap.reports.filter((r) => ["color", "gamma", "vibrance", "overlay"].includes(r.subsystem));
  return (
    <Page title="ColorTuner" sub="Цвет применяется ко всему экрану через Windows, пока профиль включён."
      aside={<button className="btn" onClick={() => setProfile((p) => ({ ...p, color: DEF }))}>Сбросить всё</button>}>
      <svg width="0" height="0" style={{ position: "absolute" }} aria-hidden>
        <filter id="bp-preview" colorInterpolationFilters="sRGB">
          <feColorMatrix type="matrix" values={svgValues(m)} />
          <feComponentTransfer><feFuncR type="gamma" exponent={1 / c.gamma} /><feFuncG type="gamma" exponent={1 / c.gamma} /><feFuncB type="gamma" exponent={1 / c.gamma} /></feComponentTransfer>
        </filter>
      </svg>
      <div className="chips top">{presets.map((p) => <button key={p.id} className={`chip ${snap.activeColor === p.id ? "on" : ""}`} onClick={() => applyPreset(p.id)}>{p.name}</button>)}
        {!snap.activeColor && <span className="chip ghost">Custom</span>}</div>
      <div className="grid g2 wide-left">
        <Card>
          <Slider label="Saturation (насыщенность)" value={c.saturation} min={-100} max={100} neutral={0} onChange={set("saturation")} />
          <Slider label="Vibrance" value={c.vibrance} min={-100} max={100} neutral={0} onChange={set("vibrance")} note="Через драйвер NVIDIA (Digital Vibrance). Если NVAPI недоступен — через шейдер overlay." />
          <Slider label="Contrast (контраст)" value={c.contrast} min={-100} max={100} neutral={0} onChange={set("contrast")} />
          <Slider label="Brightness (яркость)" value={c.brightness} min={-100} max={100} neutral={0} onChange={set("brightness")} />
          <Slider label="Gamma" value={c.gamma} min={0.6} max={1.8} step={0.01} digits={2} neutral={1} onChange={set("gamma")} note="Рампа гаммы основного монитора; драйвер может отклонить крайние значения." />
          <Slider label="Temperature (температура)" value={c.temperature} min={-100} max={100} neutral={0} onChange={set("temperature")} note="Влево холоднее, вправо теплее." />
          <Slider label="Tint (оттенок)" value={c.tint} min={-100} max={100} neutral={0} onChange={set("tint")} note="Влево зеленее, вправо пурпурнее." />
          <Slider label="Sharpness (резкость)" value={c.sharpness} min={0} max={100} neutral={0} onChange={set("sharpness")} note="Работает через overlay: он захватывает окно игры и рисует его с шейдером CAS. Запускается автоматически, когда игра в фокусе." />
        </Card>
        <div className="stack">
          <Card title="Превью" hint="Считается той же формулой, что и применение. Vibrance и резкость в превью не показаны."><Scene filterId="bp-preview" height={240} /></Card>
          {rep.length > 0 && <Card title="Статус">{rep.map((r, i) => <p key={i} className={`line ${r.status}`}><b>{r.subsystem}</b> {r.detail}</p>)}</Card>}
        </div>
      </div>
    </Page>
  );
}
