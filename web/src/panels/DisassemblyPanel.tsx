// Disassembly / Code panel: V0.1 shows PC + nearest symbol + Go to address.
// Full instruction rendering lands in P5.
import { useState } from "react";
import { useDebugStore } from "../store/debugStore";
import { parseNumber } from "../debug/formatters";

export function DisassemblyPanel() {
  const pc = useDebugStore((s) => s.status.pc);
  const [addr, setAddr] = useState("");
  const [note, setNote] = useState("");

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">Go to 地址</span>
        <input
          className="w-32 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
          placeholder="0x08003428"
          value={addr}
          onChange={(e) => setAddr(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              const v = parseNumber(addr);
              setNote(v !== null ? `定位到 0x${v.toString(16)}` : "无效地址");
            }
          }}
        />
        <button
          className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600"
          onClick={() => {
            const v = parseNumber(addr);
            setNote(v !== null ? `定位到 0x${v.toString(16)}` : "无效地址");
          }}
        >
          定位
        </button>
        <span className="text-zinc-500">
          {pc !== undefined && pc !== null ? `PC = 0x${pc.toString(16)}` : "未连接"}
        </span>
      </div>
      <div className="flex-1 overflow-auto p-2 font-mono">
        {pc !== undefined && pc !== null ? (
          <div className="rounded bg-amber-500/10 px-2 py-1 text-amber-300">
            &gt;&gt; 0x{pc.toString(16).toUpperCase().padStart(8, "0")}  PC（反汇编 P5）
          </div>
        ) : null}
        <div className="mt-2 text-zinc-600">{note}</div>
      </div>
    </div>
  );
}
