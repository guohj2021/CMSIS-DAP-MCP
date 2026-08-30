// Breakpoints panel: HW/SW breakpoints + limits + watchpoints (V0.1).
import { useEffect, useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { parseNumber } from "../debug/formatters";

export function BreakpointsPanel() {
  const connected = useDebugStore((s) => s.connected);
  const breakpoints = useDebugStore((s) => s.breakpoints);
  const setBreakpoints = useDebugStore((s) => s.setBreakpoints);
  const log = useDebugStore((s) => s.log);
  const [addr, setAddr] = useState("0x08000000");
  const [kind, setKind] = useState<"hw" | "sw_flash">("hw");
  const [limits, setLimits] = useState<{ used?: number; total?: number | null }>({});
  const [watchpoints, setWatchpoints] = useState<{ address: number; access: string }[]>([]);
  const [wpAddr, setWpAddr] = useState("0x20000000");

  async function refresh() {
    try {
      const r = await api.breakpoints();
      setBreakpoints(r.breakpoints);
    } catch {
      /* ignore */
    }
    try {
      const l = await api.breakpointsLimits();
      setLimits(l);
    } catch {
      /* ignore */
    }
    try {
      const w = await api.watchpoints();
      setWatchpoints(w.watchpoints ?? []);
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    if (connected) refresh();
  }, [connected]);

  async function addBp() {
    const v = parseNumber(addr);
    if (v === null) return;
    try {
      await api.breakpointSet(v, kind === "sw_flash" ? "sw_flash" : undefined);
      log("info", `已设置${kind === "sw_flash" ? " Flash 软件" : "硬件"}断点 0x${v.toString(16)}`);
      await refresh();
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function addWp() {
    const v = parseNumber(wpAddr);
    if (v === null) return;
    try {
      await api.watchpointSet(v);
      log("info", `已设置观察点 0x${v.toString(16)}`);
      await refresh();
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  const limitsText =
    limits.total === null || limits.total === undefined
      ? "HW 断点: Unknown"
      : `HW 断点: ${limits.used ?? 0} / ${limits.total}`;

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">断点</span>
        <input
          className="w-28 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
          value={addr}
          onChange={(e) => setAddr(e.target.value)}
        />
        <select
          className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-200 outline-none"
          value={kind}
          onChange={(e) => setKind(e.target.value as "hw" | "sw_flash")}
        >
          <option value="hw">HW</option>
          <option value="sw_flash">SW(Flash)</option>
        </select>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={addBp} disabled={!connected}>
          添加
        </button>
        <span className="text-zinc-500">{limitsText}</span>
      </div>
      <div className="flex-1 overflow-auto px-2 py-1">
        {breakpoints.map((b) => (
          <div key={b} className="flex items-center justify-between rounded px-1 py-0.5 hover:bg-zinc-800/50">
            <span className="font-mono text-amber-300">0x{b.toString(16).toUpperCase().padStart(8, "0")}</span>
            <button
              className="rounded bg-red-900 px-2 py-0.5 text-red-200 hover:bg-red-800"
              onClick={async () => {
                try {
                  await api.breakpointClear();
                  await refresh();
                } catch (e) {
                  log("error", e instanceof Error ? e.message : String(e));
                }
              }}
            >
              清除
            </button>
          </div>
        ))}
      </div>
      <div className="flex items-center gap-2 border-t border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">观察点</span>
        <input
          className="w-28 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
          value={wpAddr}
          onChange={(e) => setWpAddr(e.target.value)}
        />
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={addWp} disabled={!connected}>
          添加
        </button>
        <span className="text-zinc-500">{watchpoints.length} 个</span>
      </div>
    </div>
  );
}
