// Toolbar: Connect | Run | Halt | Step | Reset(Reset & Halt) | 状态
import { useState } from "react";
import { useDebugStore } from "../store/debugStore";
import * as ops from "../debug/operations";

export function Toolbar() {
  const status = useDebugStore((s) => s.status);
  const probes = useDebugStore((s) => s.probes);
  const [probeId, setProbeId] = useState("");
  const [connecting, setConnecting] = useState(false);

  const connected = status.server === "ready";
  const running = status.target === "running";
  const halted = status.target === "halted";
  const busy = status.server === "busy" || status.operation === "flash";

  const btn =
    "rounded px-3 py-1 text-xs font-medium disabled:cursor-not-allowed disabled:opacity-40";

  async function doConnect() {
    setConnecting(true);
    try {
      await ops.connect(probeId || undefined);
    } finally {
      setConnecting(false);
    }
  }

  return (
    <div className="flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-3 py-2">
      <span className="mr-2 font-semibold text-zinc-200">CMSIS-DAP Web Debug</span>

      {!connected ? (
        <>
          <select
            className="w-52 rounded bg-zinc-800 px-2 py-1 text-xs text-zinc-200 outline-none"
            value={probeId}
            onChange={(e) => setProbeId(e.target.value)}
          >
            <option value="">默认探针</option>
            {probes.map((p) => (
              <option key={p.id} value={p.id}>
                {p.product} ({p.serial ?? p.id})
              </option>
            ))}
          </select>
          <button
            className={`${btn} bg-blue-600 hover:bg-blue-500 text-white`}
            onClick={doConnect}
            disabled={connecting || busy}
          >
            {connecting ? "连接中…" : "连接"}
          </button>
        </>
      ) : (
        <button className={`${btn} bg-red-700 hover:bg-red-600 text-white`} onClick={ops.disconnect}>
          断开
        </button>
      )}

      <span className="mx-2 h-5 w-px bg-zinc-700" />

      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={ops.run} disabled={!connected || running || busy}>
        运行
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={ops.halt} disabled={!connected || !running || busy}>
        暂停
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={ops.step} disabled={!connected || !halted || busy}>
        单步
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={ops.stepOver} disabled={!connected || !halted || busy}>
        跳过
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={ops.stepOut} disabled={!connected || !halted || busy}>
        跳出
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={() => ops.reset("run")} disabled={!connected || busy}>
        复位
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={() => ops.reset("halt")} disabled={!connected || busy}>
        复位并暂停
      </button>
      <button className={`${btn} bg-zinc-700 hover:bg-zinc-600 text-zinc-200`} onClick={ops.snapshot} disabled={!connected || busy}>
        快照
      </button>

      <span className="ml-auto flex items-center gap-2 text-xs">
        <StatusDot state={status.server} />
        <span className="text-zinc-400">
          {status.server === "disconnected"
            ? "未连接"
            : status.operation === "flash"
              ? "FLASHING…"
              : status.target === "running"
                ? "运行中"
                : status.target === "halted"
                  ? "已暂停"
                  : status.target === "fault"
                    ? "FAULT"
                    : "已连接"}
        </span>
      </span>
    </div>
  );
}

function StatusDot({ state }: { state: string }) {
  const color =
    state === "ready"
      ? "bg-green-500"
      : state === "busy"
        ? "bg-amber-400"
        : "bg-red-500";
  return <span className={`inline-block h-2 w-2 rounded-full ${color}`} />;
}
