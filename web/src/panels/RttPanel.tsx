// RTT viewer: start/stop, channel output with timestamps.
import { useEffect, useRef, useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { onWsEvent } from "../ws/client";

interface RttLine {
  ts: string;
  channel: number;
  text: string;
}

export function RttPanel() {
  const connected = useDebugStore((s) => s.connected);
  const log = useDebugStore((s) => s.log);
  const [running, setRunning] = useState(false);
  const [lines, setLines] = useState<RttLine[]>([]);
  const [decoder] = useState(() => new TextDecoder("utf-8"));
  const buf = useRef(new Uint8Array(0));

  useEffect(() => {
    const off = onWsEvent("rtt_output", (data) => {
      const d = data as { channel: number; name?: string | null; data: string };
      const bin = Uint8Array.from(atob(d.data), (c) => c.charCodeAt(0));
      buf.current = new Uint8Array([...buf.current, ...bin]);
      // Split on newlines for line-oriented display.
      let idx = 0;
      let nl = buf.current.indexOf(10);
      const chunks: string[] = [];
      while (nl !== -1) {
        chunks.push(decoder.decode(buf.current.slice(idx, nl)).replace(/\r$/, ""));
        idx = nl + 1;
        nl = buf.current.indexOf(10, idx);
      }
      buf.current = buf.current.slice(idx);
      if (chunks.length) {
        const now = new Date().toLocaleTimeString();
        setLines((prev) => [
          ...prev.slice(-499),
          ...chunks.map((text) => ({ ts: now, channel: d.channel, text })),
        ]);
      }
    });
    return off;
  }, [decoder]);

  async function start() {
    try {
      await api.rttStart();
      setRunning(true);
      log("info", "RTT 已启动");
    } catch (e) {
      log("error", `RTT 启动失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  async function stop() {
    try {
      await api.rttStop();
      setRunning(false);
      log("info", "RTT 已停止");
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">RTT</span>
        {!running ? (
          <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600 disabled:opacity-40" onClick={start} disabled={!connected}>
            启动
          </button>
        ) : (
          <button className="rounded bg-red-900 px-2 py-0.5 text-red-200 hover:bg-red-800" onClick={stop}>
            停止
          </button>
        )}
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => setLines([])}>
          清空
        </button>
        {!connected && <span className="text-red-400">未连接</span>}
      </div>
      <div className="flex-1 overflow-auto px-2 py-1 font-mono">
        {lines.map((l, i) => (
          <div key={i} className="text-zinc-300">
            <span className="text-zinc-600">[{l.ts}]</span> <span className="text-blue-400">ch{l.channel}:</span> {l.text}
          </div>
        ))}
      </div>
    </div>
  );
}
