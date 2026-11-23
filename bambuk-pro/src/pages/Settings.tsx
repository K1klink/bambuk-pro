import { Card, Page, Switch } from "../components/ui";
import { useApp } from "../store";

export default function Settings() {
  const { snap, setSettings, setApplied } = useApp();
  if (!snap) return null;
  const s = snap.settings;
  return (
    <Page title="Settings">
      <Card title="Автоматизация">
        <div className="filter-row"><div><h3>Применять профиль при запуске игры</h3><p className="muted">Когда появится процесс ROB RUN.exe, профиль включится сам.</p></div>
          <Switch on={s.autoApplyOnGameStart} onChange={(v) => setSettings({ ...s, autoApplyOnGameStart: v })} label="Автоприменение" /></div>
        <div className="filter-row"><div><h3>Откатывать при выходе из игры</h3><p className="muted">Разрешение, цвет и гамма вернутся, когда игра закроется.</p></div>
          <Switch on={s.autoRevertOnGameExit} onChange={(v) => setSettings({ ...s, autoRevertOnGameExit: v })} label="Автооткат" /></div>
      </Card>
      <Card title="Безопасность и откат" hint="При закрытии Bambuk Pro все изменения откатываются всегда.">
        <ul className="plain">
          <li>Исходные гамма и vibrance сохраняются на диск. Если программу завершили аварийно, при следующем запуске значения вернутся автоматически.</li>
          <li>Цветовая матрица и временное разрешение сбрасываются самой системой при завершении процесса.</li>
          <li>Файлы игры не изменяются, в процесс игры ничего не внедряется.</li>
        </ul>
        <button className="btn danger" onClick={() => setApplied(false)}>Откатить всё сейчас</button>
      </Card>
      <Card title="О программе"><p className="muted">Bambuk Pro 0.1. Требует права администратора (нужны для измерения FPS). Мониторинг кадров использует PresentMon (Intel, лицензия MIT).</p></Card>
    </Page>
  );
}
