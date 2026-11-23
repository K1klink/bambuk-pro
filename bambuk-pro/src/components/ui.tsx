import { ReactNode, useEffect, useRef } from "react";
import type { Report } from "../api";

export function Card({ title, hint, children, className = "" }: { title?: string; hint?: string; children: ReactNode; className?: string }) {
  return (
    <section className={`card ${className}`}>
      {(title || hint) && <header className="card-h">{title && <h3>{title}</h3>}{hint && <p>{hint}</p>}</header>}
      {children}
    </section>
  );
}

export function Page({ title, sub, children, aside }: { title: string; sub?: string; children: ReactNode; aside?: ReactNode }) {
  return (
    <div className="page">
      <div className="page-h"><div><h1>{title}</h1>{sub && <p>{sub}</p>}</div>{aside}</div>
      {children}
    </div>
  );
}

export function Slider({ label, value, min, max, step = 1, neutral, onChange, unit = "", digits = 0, note }: {
  label: string; value: number; min: number; max: number; step?: number; neutral: number; onChange: (v: number) => void; unit?: string; digits?: number; note?: string;
}) {
  const pct = ((value - min) / (max - min)) * 100;
  const zero = ((neutral - min) / (max - min)) * 100;
  const lo = Math.min(pct, zero), hi = Math.max(pct, zero);
  const changed = Math.abs(value - neutral) > 1e-6;
  return (
    <div className="slider">
      <div className="slider-top">
        <label>{label}</label>
        <div className="slider-val">
          <input type="number" value={Number(value.toFixed(digits))} min={min} max={max} step={step}
            onChange={(e) => { const v = parseFloat(e.target.value); if (!Number.isNaN(v)) onChange(Math.min(max, Math.max(min, v))); }} />
          <span>{unit}</span>
          <button className="reset" disabled={!changed} onClick={() => onChange(neutral)} title="Сбросить" aria-label={`Сбросить ${label}`}>↺</button>
        </div>
      </div>
      <input className="range" type="range" min={min} max={max} step={step} value={value}
        style={{ ["--lo" as string]: `${lo}%`, ["--hi" as string]: `${hi}%` }}
        onChange={(e) => onChange(parseFloat(e.target.value))} aria-label={label} />
      {note && <p className="slider-note">{note}</p>}
    </div>
  );
}

export function Switch({ on, onChange, label, disabled }: { on: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean }) {
  return <button role="switch" aria-checked={on} aria-label={label} disabled={disabled} className={`switch ${on ? "on" : ""}`} onClick={() => onChange(!on)}><i /></button>;
}

const NAMES: Record<string, string> = { potato: "Разрешение", color: "Цвет", gamma: "Гамма", vibrance: "Vibrance", overlay: "Overlay (резкость)", engine: "Движок" };
const LABEL: Record<Report["status"], string> = { applied: "Применено", failed: "Ошибка", unsupported: "Недоступно", skipped: "Пропущено" };

export function Reports({ reports }: { reports: Report[] }) {
  if (!reports.length) return <p className="muted">Пока ничего не применялось.</p>;
  return (
    <ul className="reports">
      {reports.map((r, i) => (
        <li key={i} className={r.status}>
          <span className="badge">{LABEL[r.status]}</span>
          <b>{NAMES[r.subsystem] ?? r.subsystem}</b>
          <span>{r.detail}</span>
        </li>
      ))}
    </ul>
  );
}

export function Stat({ label, value, unit, tone }: { label: string; value: string; unit?: string; tone?: "good" | "warn" | "bad" }) {
  return <div className={`stat ${tone ?? ""}`}><span>{label}</span><b>{value}<small>{unit}</small></b></div>;
}

/** Процедурная сцена для превью. mode "pixel": рисуем в малом разрешении и растягиваем без сглаживания. */
export function Scene({ scale = 1, filterId, height = 220 }: { scale?: number; filterId?: string; height?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current; if (!c) return;
    const w = Math.max(32, Math.round(480 * scale)), h = Math.max(18, Math.round(270 * scale));
    c.width = w; c.height = h;
    const g = c.getContext("2d")!; g.imageSmoothingEnabled = false;
    const sky = g.createLinearGradient(0, 0, 0, h * 0.62);
    sky.addColorStop(0, "#3b5bdb"); sky.addColorStop(0.55, "#e8590c"); sky.addColorStop(1, "#ffd43b");
    g.fillStyle = sky; g.fillRect(0, 0, w, h);
    g.fillStyle = "#fff3bf"; g.beginPath(); g.arc(w * 0.72, h * 0.34, h * 0.11, 0, 7); g.fill();
    g.fillStyle = "#2b8a3e"; g.beginPath(); g.moveTo(0, h * 0.7); for (let x = 0; x <= w; x += w / 24) g.lineTo(x, h * (0.58 + 0.08 * Math.sin(x / w * 9))); g.lineTo(w, h); g.lineTo(0, h); g.fill();
    g.fillStyle = "#862e9c"; g.fillRect(w * 0.12, h * 0.56, w * 0.14, h * 0.2);
    g.fillStyle = "#e03131"; g.fillRect(w * 0.3, h * 0.64, w * 0.1, h * 0.12);
    g.fillStyle = "#1c7ed6"; g.fillRect(w * 0.45, h * 0.6, w * 0.08, h * 0.16);
    g.strokeStyle = "#fff"; g.lineWidth = Math.max(1, h / 130);
    for (let i = 0; i < 6; i++) { g.beginPath(); g.moveTo(w * (0.1 + i * 0.15), h * 0.82); g.lineTo(w * (0.18 + i * 0.15), h * 0.82); g.stroke(); }
    g.fillStyle = "#e9ecef"; g.font = `${Math.round(h * 0.09)}px Bahnschrift, sans-serif`; g.fillText("ROB RUN", w * 0.06, h * 0.12);
  }, [scale]);
  return <canvas ref={ref} className="scene" style={{ height, filter: filterId ? `url(#${filterId})` : undefined }} />;
}
