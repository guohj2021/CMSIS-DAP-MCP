// Zustand debug store (frozen v5 §33 slices). V0.1: connection/target/cpu/
// registers/memory/breakpoints/watch/peripherals/console.

import { create } from "zustand";
import { ProbeInfo, RegisterValue, TargetInfo } from "../api/client";

export type ServerState = "ready" | "busy" | "disconnected";
export type TargetState = "running" | "halted" | "resetting" | "fault" | "unknown";

export interface SessionStatus {
  server: ServerState;
  target: TargetState;
  operation: "none" | "debug" | "flash" | "rtt" | "evr";
  reason?: string | null;
  pc?: number | null;
}

export interface MemoryView {
  id: number;
  address: number;
  width: "u8" | "u16" | "u32";
  rows: { address: number; bytes: number[]; ascii: string }[];
}

export interface ConsoleLine {
  ts: string;
  level: "info" | "warn" | "error" | "debug";
  text: string;
}

interface DebugStore {
  connected: boolean;
  connecting: boolean;
  status: SessionStatus;
  probes: ProbeInfo[];
  target?: TargetInfo;
  registers: RegisterValue[];
  memoryViews: MemoryView[];
  breakpoints: number[];
  console: ConsoleLine[];
  wsConnected: boolean;
  error?: string;

  setStatus: (s: Partial<SessionStatus>) => void;
  setWsConnected: (v: boolean) => void;
  setProbes: (p: ProbeInfo[]) => void;
  setTarget: (t?: TargetInfo) => void;
  setRegisters: (r: RegisterValue[]) => void;
  addMemoryView: (v: MemoryView) => void;
  setBreakpoints: (b: number[]) => void;
  log: (level: ConsoleLine["level"], text: string) => void;
  setError: (e?: string) => void;
  clearConsole: () => void;
}

export const useDebugStore = create<DebugStore>((set) => ({
  connected: false,
  connecting: false,
  status: { server: "disconnected", target: "unknown", operation: "none" },
  probes: [],
  registers: [],
  memoryViews: [],
  breakpoints: [],
  console: [],
  wsConnected: false,

  setStatus: (s) =>
    set((st) => {
      const status = { ...st.status, ...s };
      return {
        status,
        // `connected` only follows explicit server transitions. Partial
        // patches (500ms status polling, run/halt/step/reset) carry no
        // `server`, so they must never flip the connection state - otherwise
        // the poll loop kills itself and buttons/panels flicker every cycle.
        connected:
          s.server !== undefined ? s.server !== "disconnected" : st.connected,
      };
    }),
  setWsConnected: (v) => set({ wsConnected: v }),
  setProbes: (p) => set({ probes: p }),
  setTarget: (t) => set({ target: t }),
  setRegisters: (r) => set({ registers: r }),
  addMemoryView: (v) => set((st) => ({ memoryViews: [...st.memoryViews, v] })),
  setBreakpoints: (b) => set({ breakpoints: b }),
  log: (level, text) =>
    set((st) => ({
      console: [...st.console.slice(-499), { ts: new Date().toLocaleTimeString(), level, text }],
    })),
  setError: (e) => set({ error: e }),
  clearConsole: () => set({ console: [] }),
}));
