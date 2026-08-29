// CPU Registers panel: HEX/DEC/BIN, PC highlight, per-register value.
import { useMemo, useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { formatValue, parseNumber, NumFormat } from "../debug/formatters";

const PC_RE = /^(pc|r15)$/i;

export function RegistersPanel() {
  const registers = useDebugStore((s) => s.registers);
  const [fmt, setFmt] = useState<NumFormat>("hex");
  const [filter, setFilter] = useState("");

  const rows = useMemo(() => {
    const f = filter.trim().toLowerCase();
    return registers.filter((r) => r.name.toLowerCase().includes(f));
  }, [registers, filter]);

  const log = useDebugStore((s) => s.log);

  async function writeRegister(name: string, input: string) {
    const v = parseNumber(input);
    if (v === null || v < 0 || v > 0xffffffff) {
      log("error", `无效寄存器值: ${input}`);
      return;
    }
    try {
      await api.registerWrite(name, v);
      log("info", `寄存器 ${name} = 0x${v.toString(16)}`);
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

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
            <input
              className="w-24 bg-transparent text-right font-mono text-zinc-200 outline-none focus:bg-zinc-700"
              defaultValue={formatValue(r.value, fmt)}
              key={`${r.name}-${fmt}-${r.value}`}
              onBlur={(e) => writeRegister(r.name, e.target.value)}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
