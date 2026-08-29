// SWO / ITM viewer: start/stop, poll SWO data, decode ITM port 0 as text.
import { useEffect, useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";

export function SwoPanel() {
  const connected = useDebugStore((s) => s.connected);
  const log = useDebugStore((s) => s.log);
  const [running, setRunning] = useState(false);
  const [text, setText] = useState("");
  const [decoder] = useState(() => new TextDecoder("utf-8"));

  useEffect(() => {
    if (!running) return;
    const timer = setInterval(async () => {
      try {
        const r = await api.swoRead();
        if (r.data && r.data.length) {
          const bytes = Uint8Array.from(r.data);
          setText((t) => (t + decoder.decode(bytes)).slice(-8000));
        }
      } catch {
        /* ignore */
      }
    }, 100);
    return () => clearInterval(timer);
  }, [running, decoder]);

  async function start() {
    try {
      await api.swoStart(2_000_000, 8_000_000);
      setRunning(true);
      log("info", "SWO 已启动 (2MHz)");
    } catch (e) {
      log("error", `SWO 启动失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  async function stop() {
    try {
      await api.swoStop();
      setRunning(false);
      log("info", "SWO 已停止");
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">SWO / ITM</span>
        {!running ? (
          <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600 disabled:opacity-40" onClick={start} disabled={!connected}>
            启动
          </button>
        ) : (
          <button className="rounded bg-red-900 px-2 py-0.5 text-red-200 hover:bg-red-800" onClick={stop}>
            停止
          </button>
        )}
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => setText("")}>
          清空
        </button>
        {!connected && <span className="text-red-400">未连接</span>}
      </div>
      <pre className="flex-1 overflow-auto whitespace-pre-wrap px-2 py-1 font-mono text-emerald-300">{text || "（等待 SWO 数据）"}</pre>
    </div>
  );
}
