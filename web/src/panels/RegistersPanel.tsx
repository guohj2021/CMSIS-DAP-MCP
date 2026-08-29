// CPU Registers panel: HEX/DEC/BIN, PC highlight, per-register value.
import { useMemo, useState } from "react";
import { useDebugStore } from "../store/debugStore";
import { formatValue, NumFormat } from "../debug/formatters";

const PC_RE = /^(pc|r15)$/i;

export function RegistersPanel() {
  const registers = useDebugStore((s) => s.registers);
  const [fmt, setFmt] = useState<NumFormat>("hex");
  const [filter, setFilter] = useState("");

  const rows = useMemo(() => {
    const f = filter.trim().toLowerCase();
    return registers.filter((r) => r.name.toLowerCase().includes(f));
  }, [registers, filter]);

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <input
          className="w-40 rounded bg-zinc-800 px-2 py-0.5 text-zinc-200 outline-none"
          placeholder="过滤寄存器…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <select
          className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-200 outline-none"
          value={fmt}
          onChange={(e) => setFmt(e.target.value as NumFormat)}
        >
          <option value="hex">HEX</option>
          <option value="dec">DEC</option>
          <option value="bin">BIN</option>
        </select>
        <span className="text-zinc-500">{rows.length} 个寄存器</span>
      </div>
      <div className="grid grid-cols-2 gap-x-3 overflow-y-auto px-2 py-1">
        {rows.map((r) => (
          <div
            key={r.name}
            className={`flex justify-between rounded px-1 py-0.5 ${
              PC_RE.test(r.name) ? "bg-amber-500/20 font-bold text-amber-300" : "text-zinc-300"
            }`}
          >
            <span className="text-zinc-400">{r.name}</span>
            <span className="font-mono">{formatValue(r.value, fmt)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
