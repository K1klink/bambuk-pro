import { invoke } from "@tauri-apps/api/core";

export interface ColorParams { saturation: number; vibrance: number; contrast: number; brightness: number; gamma: number; temperature: number; tint: number; sharpness: number }
export interface Toggle { enabled: boolean; amount: number }
export interface FilterParams { grayscale: Toggle; sepia: Toggle; invert: Toggle; blueCut: Toggle; highContrast: Toggle; redBoost: Toggle }
export type PotatoLevel = "default" | "potato" | "maxPotato" | "extreme" | "ultraPotato";
export interface PotatoParams { level: PotatoLevel; customScale: number | null }
export interface Profile { potato: PotatoParams; color: ColorParams; filters: FilterParams; overlay: { enabled: boolean } }
export type PresetKind = "graphics" | "color" | "full";
export interface Preset { id: string; name: string; kind: PresetKind; builtin: boolean; profile: Profile }
export interface AppSettings { autoApplyOnGameStart: boolean; autoRevertOnGameExit: boolean }
export interface Report { subsystem: string; status: "applied" | "failed" | "unsupported" | "skipped"; detail: string }
export interface Mode { w: number; h: number; hz: number }
export interface EngineStatus {
  supported: boolean; colorActive: boolean; gammaActive: boolean; vibranceActive: boolean; displayChanged: boolean;
  nativeMode: Mode | null; currentMode: Mode | null; overlay: { running: boolean; frames: number; error: string | null }; recovered: string | null;
}
export interface GameState { running: boolean; pid: number | null; hasWindow: boolean; hwnd: number | null }
export interface Snapshot {
  profile: Profile; presets: Preset[]; settings: AppSettings; activeGraphics: string | null; activeColor: string | null;
  applied: boolean; engine: EngineStatus; game: GameState; reports: Report[]; perfExeFound: boolean;
}
export interface FrameStats { fps: number; onePercentLow: number; avgFrameMs: number; lastFrameMs: number; p99FrameMs: number; frames: number }
export interface Hw { cpuPercent: number | null; gpuPercent: number | null; gpuTempC: number | null; gpuClockMhz: number | null; vramUsedMb: number | null; vramTotalMb: number | null; gpuName: string | null; gpuNote: string | null }
export interface Live {
  game: GameState; frame: FrameStats; perf: { running: boolean; receiving: boolean; note: string | null }; hw: Hw;
  engine: EngineStatus; baseline: FrameStats | null; applied: boolean;
}

export const api = {
  snapshot: () => invoke<Snapshot>("get_snapshot"),
  updateProfile: (profile: Profile) => invoke<Snapshot>("update_profile", { profile }),
  setApplied: (on: boolean) => invoke<Snapshot>("set_applied", { on }),
  applyPreset: (id: string) => invoke<Snapshot>("apply_preset", { id }),
  createPreset: (name: string, kind: PresetKind) => invoke<Snapshot>("create_preset", { name, kind }),
  savePreset: (id: string) => invoke<Snapshot>("save_preset", { id }),
  renamePreset: (id: string, name: string) => invoke<Snapshot>("rename_preset", { id, name }),
  deletePreset: (id: string) => invoke<Snapshot>("delete_preset", { id }),
  exportPreset: (id: string) => invoke<string>("export_preset", { id }),
  importPreset: (json: string) => invoke<Snapshot>("import_preset", { json }),
  setSettings: (settings: AppSettings) => invoke<Snapshot>("set_settings", { settings }),
  live: () => invoke<Live>("live"),
  setBaseline: () => invoke<void>("set_baseline"),
  clearBaseline: () => invoke<void>("clear_baseline"),
};

export const POTATO_LEVELS: { id: PotatoLevel; name: string; scale: number }[] = [
  { id: "default", name: "Default", scale: 1 },
  { id: "potato", name: "Potato", scale: 0.85 },
  { id: "maxPotato", name: "Max Potato", scale: 0.75 },
  { id: "extreme", name: "Extreme", scale: 0.6 },
  { id: "ultraPotato", name: "Ultra Potato", scale: 0.5 },
];
export const levelScale = (p: PotatoParams) => (p.customScale ?? POTATO_LEVELS.find((l) => l.id === p.level)!.scale);
