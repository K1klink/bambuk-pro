import { Card, Page, Scene, Slider, Switch } from "../components/ui";
import type { FilterParams } from "../api";
import { fullMatrix, svgValues } from "../colorMath";
import { useApp } from "../store";

const LIST: { key: keyof FilterParams; name: string; desc: string }[] = [
  { key: "grayscale", name: "Чёрно-белый", desc: "Убирает цвет, оставляя яркость." },
  { key: "sepia", name: "Сепия", desc: "Тёплый коричневатый тон." },
  { key: "invert", name: "Инверсия", desc: "Меняет светлое и тёмное местами." },
  { key: "blueCut", name: "Без синего", desc: "Срезает синий канал, снижает нагрузку на глаза." },
  { key: "highContrast", name: "Высокий контраст", desc: "Усиливает контраст и немного насыщенность." },
  { key: "redBoost", name: "Усиление красного", desc: "Делает красные элементы заметнее." },
];

export default function Filters() {
  const { snap, setProfile } = useApp();
  if (!snap) return null;
  const f = snap.profile.filters; const ov = snap.engine.overlay;
  const m = fullMatrix(snap.profile.color, f);
  const setF = (k: keyof FilterParams, patch: Partial<FilterParams[typeof k]>) => setProfile((p) => ({ ...p, filters: { ...p.filters, [k]: { ...p.filters[k], ...patch } } }));
  const sharp = snap.profile.color.sharpness > 0;
  return (
    <Page title="Filters" sub="Фильтры можно включать вместе. Все они работают через ту же цветовую матрицу Windows, что и ColorTuner.">
      <svg width="0" height="0" style={{ position: "absolute" }} aria-hidden>
        <filter id="bp-filters" colorInterpolationFilters="sRGB"><feColorMatrix type="matrix" values={svgValues(m)} /></filter>
      </svg>
      <div className="grid g2 wide-left">
        <div className="stack">
          {LIST.map((it) => (
            <Card key={it.key} className="filter">
              <div className="filter-row">
                <div><h3>{it.name}</h3><p className="muted">{it.desc}</p></div>
                <Switch on={f[it.key].enabled} onChange={(v) => setF(it.key, { enabled: v })} label={it.name} />
              </div>
              {f[it.key].enabled && <Slider label="Сила" value={f[it.key].amount} min={0} max={100} neutral={100} unit="%" onChange={(v) => setF(it.key, { amount: v })} />}
            </Card>
          ))}
        </div>
        <div className="stack">
          <Card title="Превью"><Scene filterId="bp-filters" height={220} /></Card>
          <Card title="Overlay-движок" hint="Захватывает окно игры и рисует его поверх с шейдером: резкость (CAS) и, при необходимости, vibrance.">
            <div className="filter-row"><div><h3>Включить overlay</h3><p className="muted">{sharp ? "Включён автоматически: резкость в ColorTuner больше нуля." : "Нужен только для резкости и vibrance без NVIDIA."}</p></div>
              <Switch on={snap.profile.overlay.enabled || sharp} disabled={sharp} onChange={(v) => setProfile((p) => ({ ...p, overlay: { enabled: v } }))} label="Overlay" /></div>
            <p className="kv"><span>Состояние</span><b>{ov.running ? "Работает" : "Остановлен"}</b></p>
            <p className="kv"><span>Выведено кадров</span><b>{ov.frames}</b></p>
            {ov.error && <p className="line failed">Ошибка: {ov.error}</p>}
            <p className="muted">Показывается только пока окно ROB RUN активно. Добавляет задержку примерно в один кадр и немного нагружает GPU.</p>
          </Card>
        </div>
      </div>
    </Page>
  );
}
