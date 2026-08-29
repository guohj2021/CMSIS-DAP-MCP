// Call Stack panel: real DWARF CFI frames; clicking a frame jumps the
// disassembly (Go To Address) — the frame selection drives other panels.
import { useEffect, useState } from "react";
import { api, UnwindFrame } from "../api/client";
import { useDebugStore } from "../store/debugStore";

export function CallStackPanel() {
  const halted = useDebugStore((s) => s.status.target === "halted");
  const [frames, setFrames] = useState<UnwindFrame[]>([]);
  const [available, setAvailable] = useState<boolean | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const log = useDebugStore((s) => s.log);

  async function refresh() {
    try {
      const r = await api.callstack();
      setAvailable(r.available);
      setFrames(r.frames);
      setSelected(null);
    } catch (e) {
      setAvailable(false);
      log("error", `调用栈获取失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    if (halted) refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [halted]);

  function selectFrame(i: number, pc: number) {
    setSelected(i);
    // Frame selection drives the Disassembly panel (DebugContext §9).
    window.dispatchEvent(new CustomEvent("goto-address", { detail: { address: pc } }));
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center justify-between border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">调用栈</span>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600 disabled:opacity-40" onClick={refresh} disabled={!halted}>
          刷新
        </button>
      </div>
      <div className="flex-1 overflow-auto px-2 py-1">
        {available === false && (
          <div className="rounded border border-dashed border-zinc-700 p-3 text-zinc-500">
            调用栈不可用：固件无 DWARF CFI（.debug_frame）或目标未暂停。不伪造帧。
          </div>
        )}
        {available && frames.length === 0 && (
          <div className="text-zinc-500">调用栈为空（无可用 CFI）。</div>
        )}
        {frames.map((f, i) => (
          <div
            key={`${f.pc}-${i}`}
            className={`flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 hover:bg-zinc-800/60 ${selected === i ? "bg-zinc-800 text-blue-300" : "text-zinc-300"}`}
            onClick={() => selectFrame(i, f.pc)}
          >
            <span className="w-6 text-zinc-600">#{i}</span>
            <span className="w-28 font-mono text-zinc-400">0x{f.pc.toString(16).toUpperCase().padStart(8, "0")}</span>
            <span className="flex-1 truncate">{f.function ?? "?"}</span>
            {f.source && (
              <span className="text-emerald-400">{f.source.file.split(/[\\/]/).pop()}:{f.source.line}</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
