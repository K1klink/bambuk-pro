import { useRef, useState } from "react";
import { Card, Page } from "../components/ui";
import { api, Preset, PresetKind } from "../api";
import { useApp } from "../store";

const KIND: Record<PresetKind, string> = { graphics: "Графика", color: "Цвет", full: "Полный профиль" };

export default function Presets() {
  const { snap, applyPreset, createPreset, savePreset, renamePreset, deletePreset, importPreset, run, toast } = useApp();
  const [name, setName] = useState(""); const [kind, setKind] = useState<PresetKind>("full");
  const [edit, setEdit] = useState<{ id: string; v: string } | null>(null);
  const [paste, setPaste] = useState("");
  const file = useRef<HTMLInputElement>(null);
  if (!snap) return null;
  const mine = snap.presets.filter((p) => !p.builtin);
  const builtin = snap.presets.filter((p) => p.builtin);

  const exportIt = async (p: Preset) => {
    const json = await run(api.exportPreset(p.id)); if (!json) return;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([json], { type: "application/json" }));
    a.download = `${p.name.replace(/[^\p{L}\p{N}_-]+/gu, "_")}.bambuk.json`; a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000); toast("Пресет экспортирован", "ok");
  };
  const onFile = async (f?: File) => { if (!f) return; await importPreset(await f.text()); if (file.current) file.current.value = ""; };
  const active = (p: Preset) => p.id === snap.activeGraphics || p.id === snap.activeColor;

  const Row = ({ p }: { p: Preset }) => (
    <li className={active(p) ? "on" : ""}>
      <div className="pr-main">
        {edit?.id === p.id
          ? <input className="text" autoFocus value={edit.v} onChange={(e) => setEdit({ id: p.id, v: e.target.value })} onKeyDown={(e) => { if (e.key === "Enter") { renamePreset(p.id, edit.v); setEdit(null); } if (e.key === "Escape") setEdit(null); }} />
          : <b>{p.name}</b>}
        <span className="tag">{KIND[p.kind]}</span>{active(p) && <span className="tag on">Активен</span>}
      </div>
      <div className="pr-act">
        <button className="btn sm" onClick={() => applyPreset(p.id)}>Выбрать</button>
        {!p.builtin && <>
          <button className="btn sm ghost" onClick={() => savePreset(p.id)} title="Записать текущие настройки в этот пресет">Обновить</button>
          <button className="btn sm ghost" onClick={() => setEdit(edit?.id === p.id ? null : { id: p.id, v: p.name })}>{edit?.id === p.id ? "Отмена" : "Переименовать"}</button>
        </>}
        <button className="btn sm ghost" onClick={() => exportIt(p)}>Экспорт</button>
        {!p.builtin && <button className="btn sm danger" onClick={() => { if (confirm(`Удалить пресет «${p.name}»?`)) deletePreset(p.id); }}>Удалить</button>}
      </div>
    </li>
  );

  return (
    <Page title="Presets" sub="Графика и цвет сохраняются отдельно или вместе в один профиль. Настройки хранятся между запусками.">
      <Card title="Сохранить текущие настройки">
        <div className="row">
          <input className="text" placeholder="Название пресета" value={name} maxLength={48} onChange={(e) => setName(e.target.value)} />
          <select className="text" value={kind} onChange={(e) => setKind(e.target.value as PresetKind)}>
            <option value="full">Полный профиль</option><option value="graphics">Только графика</option><option value="color">Только цвет и фильтры</option>
          </select>
          <button className="btn" disabled={!name.trim()} onClick={async () => { await createPreset(name, kind); setName(""); }}>Сохранить</button>
        </div>
      </Card>
      <Card title="Мои пресеты">{mine.length ? <ul className="plist">{mine.map((p) => <Row key={p.id} p={p} />)}</ul> : <p className="muted">Своих пресетов пока нет.</p>}</Card>
      <Card title="Встроенные"><ul className="plist">{builtin.map((p) => <Row key={p.id} p={p} />)}</ul></Card>
      <Card title="Импорт" hint="Файл .bambuk.json или текст из буфера.">
        <div className="row"><input ref={file} type="file" accept=".json,application/json" hidden onChange={(e) => onFile(e.target.files?.[0])} />
          <button className="btn" onClick={() => file.current?.click()}>Выбрать файл</button></div>
        <textarea className="text area" placeholder="Или вставьте JSON пресета сюда" value={paste} onChange={(e) => setPaste(e.target.value)} />
        <button className="btn" disabled={!paste.trim()} onClick={async () => { await importPreset(paste); setPaste(""); }}>Импортировать из текста</button>
      </Card>
    </Page>
  );
}
