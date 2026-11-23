// Зеркало crates/bambuk-core/src/colormath.rs — превью в интерфейсе считается той же формулой.
import type { ColorParams, FilterParams, Toggle } from "./api";

export type M5 = number[][];
const I = (): M5 => [[1,0,0,0,0],[0,1,0,0,0],[0,0,1,0,0],[0,0,0,1,0],[0,0,0,0,1]];
const L = [0.2126, 0.7152, 0.0722];

export function mul(a: M5, b: M5): M5 {
  const r = Array.from({ length: 5 }, () => Array(5).fill(0));
  for (let i = 0; i < 5; i++) for (let j = 0; j < 5; j++) for (let k = 0; k < 5; k++) r[i][j] += a[i][k] * b[k][j];
  return r;
}
const lerp = (a: M5, b: M5, t: number): M5 => a.map((row, i) => row.map((v, j) => v + (b[i][j] - v) * t));
export const saturation = (f: number): M5 => { const m = I(); for (let i = 0; i < 3; i++) for (let j = 0; j < 3; j++) m[i][j] = (1 - f) * L[i] + (i === j ? f : 0); return m; };
export const contrast = (c: number): M5 => { const m = I(); for (let i = 0; i < 3; i++) { m[i][i] = c; m[4][i] = 0.5 * (1 - c); } return m; };
export const brightness = (a: number): M5 => { const m = I(); for (let i = 0; i < 3; i++) m[4][i] = a; return m; };
export const gains = (r: number, g: number, b: number): M5 => { const m = I(); m[0][0] = Math.max(0, r); m[1][1] = Math.max(0, g); m[2][2] = Math.max(0, b); return m; };
const amt = (t: Toggle) => Math.min(1, Math.max(0, t.amount / 100));

export function colorMatrix(p: ColorParams): M5 {
  let m = I();
  m = mul(m, saturation(Math.max(0, 1 + p.saturation / 100)));
  m = mul(m, contrast(Math.max(0, 1 + p.contrast / 100)));
  m = mul(m, brightness((p.brightness / 100) * 0.3));
  const k = p.temperature / 100; m = mul(m, gains(1 + 0.25 * k, 1, 1 - 0.35 * k));
  const t = p.tint / 100; m = mul(m, gains(1 + 0.05 * t, 1 - 0.2 * t, 1 + 0.05 * t));
  return m;
}
export function filterMatrix(f: FilterParams): M5 {
  let m = I();
  if (f.grayscale.enabled) m = mul(m, lerp(I(), saturation(0), amt(f.grayscale)));
  if (f.sepia.enabled) {
    const s: M5 = [[0.393,0.349,0.272,0,0],[0.769,0.686,0.534,0,0],[0.189,0.168,0.131,0,0],[0,0,0,1,0],[0,0,0,0,1]];
    m = mul(m, lerp(I(), s, amt(f.sepia)));
  }
  if (f.invert.enabled) { const v = I(); for (let i = 0; i < 3; i++) { v[i][i] = -1; v[4][i] = 1; } m = mul(m, lerp(I(), v, amt(f.invert))); }
  if (f.blueCut.enabled) { const a = amt(f.blueCut); m = mul(m, gains(1, 1 - 0.12 * a, 1 - 0.5 * a)); }
  if (f.highContrast.enabled) { const a = amt(f.highContrast); m = mul(m, contrast(1 + 0.8 * a)); m = mul(m, saturation(1 + 0.2 * a)); }
  if (f.redBoost.enabled) { const a = amt(f.redBoost); m = mul(m, gains(1 + 0.35 * a, 1, 1)); }
  return m;
}
export const fullMatrix = (c: ColorParams, f: FilterParams): M5 => mul(colorMatrix(c), filterMatrix(f));

/** Значения для SVG feColorMatrix (4x5, построчно по выходным каналам). */
export function svgValues(m: M5): string {
  const row = (j: number) => `${m[0][j]} ${m[1][j]} ${m[2][j]} 0 ${m[4][j]}`;
  return `${row(0)} ${row(1)} ${row(2)} 0 0 0 1 0`;
}
