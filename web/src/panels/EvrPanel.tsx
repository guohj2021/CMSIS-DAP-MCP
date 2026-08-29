// EVR viewer: start/stop, event stream.
import { useEffect, useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { onWsEvent } from "../ws/client";

interface EvrRow {
  ts: string;
  component: number;
  message: number;
  val1: number;
  val2: number;
  irq: boolean;
}

export function EvrPanel() {
  const connected = useDebugStore((s) => s.connected);
  const log = useDebugStore((s) => s.log);
  const [running, setRunning] = useState(false);
  const [infoAddr, setInfoAddr] = useState("0x00000000");
  const [rows, setRows] = useState<EvrRow[]>([]);

  useEffect(() => {
    const off = onWsEvent("evr_event", (data) => {
      const d = data as { component: number; message: number; val1: number; val2: number; irq: boolean };
      setRows((prev) => [
        ...prev.slice(-499),
        { ts: new Date().toLocaleTimeString(), component: d.component, message: d.message, val1: d.val1, val2: d.val2, irq: d.irq },
      ]);
    });
    return off;
  }, []);

  async function start() {
    const addr = parseInt(infoAddr, 16);
    if (Number.isNaN(addr)) return;
    try {
      await api.evrStart(addr);
      setRunning(true);
      log("info", `EVR 已启动 (info@0x${addr.toString(16)})`);
    } catch (e) {
      log("error", `EVR 启动失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  async function stop() {
    try {
      await api.evrStop();
      setRunning(false);
      log("info", "EVR 已停止");
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">EVR</span>
        <input className="w-24 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none" value={infoAddr} onChange={(e) => setInfoAddr(e.target.value)} />
        {!running ? (
          <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600 disabled:opacity-40" onClick={start} disabled={!connected}>
            启动
          </button>
        ) : (
          <button className="rounded bg-red-900 px-2 py-0.5 text-red-200 hover:bg-red-800" onClick={stop}>
            停止
          </button>
        )}
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => setRows([])}>
          清空
        </button>
      </div>
      <div className="flex-1 overflow-auto px-2 py-1 font-mono">
        <div className="flex gap-2 text-zinc-500">
          <span className="w-20">时间</span>
          <span className="w-16">组件</span>
          <span className="w-16">事件</span>
          <span className="w-20">val1</span>
          <span className="w-20">val2</span>
        </div>
        {rows.map((r, i) => (
          <div key={i} className="flex gap-2 text-zinc-300">
            <span className="w-20 text-zinc-600">{r.ts}</span>
            <span className="w-16">{r.component}</span>
            <span className="w-16">{r.message}</span>
            <span className="w-20">{r.val1}</span>
            <span className="w-20">{r.val2}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
