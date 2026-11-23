import { createContext, ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { api, Live, Profile, Snapshot, PresetKind, AppSettings } from "./api";

export interface Toast { id: number; kind: "ok" | "err" | "info"; text: string }
interface Ctx {
  snap: Snapshot | null; live: Live | null; history: number[]; toasts: Toast[];
  setProfile: (f: (p: Profile) => Profile) => void;
  toast: (text: string, kind?: Toast["kind"]) => void;
  run: <T>(p: Promise<T>, okText?: string) => Promise<T | undefined>;
  setApplied: (on: boolean) => Promise<void>;
  applyPreset: (id: string) => Promise<void>;
  createPreset: (name: string, kind: PresetKind) => Promise<void>;
  savePreset: (id: string) => Promise<void>;
  renamePreset: (id: string, name: string) => Promise<void>;
  deletePreset: (id: string) => Promise<void>;
  importPreset: (json: string) => Promise<void>;
  setSettings: (s: AppSettings) => Promise<void>;
}
const C = createContext<Ctx>(null as unknown as Ctx);
export const useApp = () => useContext(C);

const errText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e));

export function Provider({ children }: { children: ReactNode }) {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [live, setLive] = useState<Live | null>(null);
  const [history, setHistory] = useState<number[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const pending = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const latest = useRef<Profile | null>(null);
  const nextId = useRef(1);

  const toast = useCallback((text: string, kind: Toast["kind"] = "info") => {
    const id = nextId.current++;
    setToasts((t) => [...t.slice(-3), { id, kind, text }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 4500);
  }, []);

  const run = useCallback(async <T,>(p: Promise<T>, okText?: string) => {
    try { const r = await p; if (okText) toast(okText, "ok"); return r; }
    catch (e) { toast(errText(e), "err"); return undefined; }
  }, [toast]);

  const refresh = useCallback(async () => { try { const s = await api.snapshot(); if (!pending.current) setSnap(s); else setSnap((o) => (o ? { ...s, profile: o.profile } : s)); } catch { /* окно закрывается */ } }, []);

  useEffect(() => {
    refresh();
    const a = window.setInterval(refresh, 2500);
    const b = window.setInterval(async () => {
      try {
        const l = await api.live(); setLive(l);
        setHistory((h) => [...h.slice(-119), l.frame.frames > 0 ? l.frame.lastFrameMs : 0]);
      } catch { /* ignore */ }
    }, 700);
    return () => { clearInterval(a); clearInterval(b); };
  }, [refresh]);

  const setProfile = useCallback((f: (p: Profile) => Profile) => {
    setSnap((s) => {
      if (!s) return s;
      const np = f(s.profile); latest.current = np; return { ...s, profile: np };
    });
    pending.current = true;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      chain.current = chain.current.then(async () => {
        const p = latest.current; if (!p) return;
        try { const s = await api.updateProfile(p); pending.current = false; setSnap(s); const bad = s.reports.filter((r) => r.status === "failed"); if (bad.length && s.applied) toast(bad.map((b) => `${b.subsystem}: ${b.detail}`).join(" · "), "err"); }
        catch (e) { pending.current = false; toast(errText(e), "err"); }
      });
    }, 160);
  }, [toast]);

  const wrap = useCallback(async (p: Promise<Snapshot>, ok?: string) => { const s = await run(p, ok); if (s) setSnap(s); return s; }, [run]);

  const value = useMemo<Ctx>(() => ({
    snap, live, history, toasts, setProfile, toast, run,
    setApplied: async (on) => {
      const s = await wrap(api.setApplied(on));
      if (!s) return;
      const bad = s.reports.filter((r) => r.status === "failed");
      if (on) toast(bad.length ? `Не всё применилось: ${bad.map((b) => b.subsystem).join(", ")}` : "Профиль применён", bad.length ? "err" : "ok");
      else toast("Все изменения откатены", "ok");
    },
    applyPreset: async (id) => { await wrap(api.applyPreset(id)); },
    createPreset: async (n, k) => { await wrap(api.createPreset(n, k), "Пресет сохранён"); },
    savePreset: async (id) => { await wrap(api.savePreset(id), "Пресет обновлён"); },
    renamePreset: async (id, n) => { await wrap(api.renamePreset(id, n), "Пресет переименован"); },
    deletePreset: async (id) => { await wrap(api.deletePreset(id), "Пресет удалён"); },
    importPreset: async (j) => { await wrap(api.importPreset(j), "Пресет импортирован"); },
    setSettings: async (s) => { await wrap(api.setSettings(s)); },
  }), [snap, live, history, toasts, setProfile, toast, run, wrap]);

  return <C.Provider value={value}>{children}</C.Provider>;
}
