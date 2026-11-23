import { Card, Page, Reports, Scene, Slider } from "../components/ui";
import { levelScale, POTATO_LEVELS } from "../api";
import { useApp } from "../store";

export default function Potato() {
  const { snap, setProfile, applyPreset } = useApp();
  if (!snap) return null;
  const pot = snap.profile.potato; const scale = levelScale(pot);
  const cur = snap.engine.currentMode; const nat = snap.engine.nativeMode ?? cur;
  const rep = snap.reports.filter((r) => r.subsystem === "potato");
  return (
    <Page title="Potato Mode" sub="Снижение нагрузки на GPU через разрешение рендера. Чем ниже уровень, тем выше FPS и тем грубее картинка.">
      <div className="grid g2 wide-left">
        <div className="stack">
          <Card title="Уровень">
            <div className="levels">
              {POTATO_LEVELS.map((l) => {
                const id = `builtin-gfx-${l.name.toLowerCase().replace(" ", "-")}`;
                const on = pot.customScale == null && pot.level === l.id;
                return (
                  <button key={l.id} className={`level ${on ? "on" : ""}`} onClick={() => applyPreset(id)}>
                    <span className="lv-name">{l.name}</span>
                    <span className="lv-scale">{Math.round(l.scale * 100)}%</span>
                    <span className="lv-bar"><i style={{ width: `${l.scale * 100}%` }} /></span>
                  </button>
                );
              })}
            </div>
            <Slider label="Свой масштаб" value={Math.round(scale * 100)} min={40} max={100} step={1} neutral={100} unit="%"
              onChange={(v) => setProfile((p) => ({ ...p, potato: { ...p.potato, customScale: v >= 100 ? null : v / 100, level: v >= 100 ? "default" : p.potato.level } }))}
              note="Числовое значение подбирается к ближайшему видеорежиму с тем же соотношением сторон." />
          </Card>
          <Card title="Как это работает">
            <ul className="plain">
              <li>Bambuk Pro временно меняет разрешение рабочего стола. Игра в режиме «Полноэкранный в окне» рисует кадр в этом разрешении, поэтому нагрузка на GPU падает почти пропорционально числу пикселей.</li>
              <li>Монитор или видеокарта растягивают картинку до экрана, из-за этого она мягче. Резкость в ColorTuner частично это компенсирует.</li>
              <li>Затрагивается весь рабочий стол: окна и значки сместятся. При закрытии Bambuk Pro и при выходе из игры исходное разрешение возвращается.</li>
              <li>Текстуры, тени и детализацию внутри игры эти уровни не меняют: без правки файлов и инъекций такой доступ невозможен.</li>
            </ul>
          </Card>
        </div>
        <div className="stack">
          <Card title="Предпросмотр" hint="Так выглядит рендер при выбранном масштабе (иллюстрация, не кадр игры)."><Scene scale={scale} height={230} /></Card>
          <Card title="Состояние">
            <p className="kv"><span>Текущее разрешение</span><b>{cur ? `${cur.w}×${cur.h} @ ${cur.hz} Гц` : "—"}</b></p>
            <p className="kv"><span>Исходное</span><b>{nat ? `${nat.w}×${nat.h}` : "—"}</b></p>
            <p className="kv"><span>Цель</span><b>{nat ? `≈ ${Math.round(nat.w * scale)}×${Math.round(nat.h * scale)}` : "—"}</b></p>
            {!snap.applied && <p className="muted">Профиль выключен — включите переключатель на Главной, чтобы изменения вступили в силу.</p>}
            {rep.length > 0 && <Reports reports={rep} />}
          </Card>
        </div>
      </div>
    </Page>
  );
}
