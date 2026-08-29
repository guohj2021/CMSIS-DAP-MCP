// Profiler panel: sampling PC histogram (V2 - Performance Analyzer basics).
import { useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";

export function ProfilerPanel() {
  const connected = useDebugStore((s) => s.connected);
  const log = useDebugStore((s) => s.log);
  const [samples, setSamples] = useState(200);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{ total: number; samples: { pc: number; count: number }[] } | null>(null);
  const [symbols, setSymbols] = useState<Record<number, string>>({});

  async function run() {
    setBusy(true);
    try {
      const r = await api.profileRun(samples, 5);
      setResult(r);
      // Resolve top PCs to symbols.
      const map: Record<number, string> = {};
      for (const s of r.samples.slice(0, 12)) {
        const addr = s.pc;
        const syms = await api.symbols({ kind: "function", limit: 2000 });
        const hit = syms.items.find((x) => x.address <= addr && addr < x.address + Math.max(x.size, 4));
        map[addr] = hit ? `${hit.name}+0x${(addr - hit.address).toString(16)}` : `0x${addr.toString(16)}`;
      }
      setSymbols(map);
      log("info", `采样完成: ${r.total} 个样本`);
    } catch (e) {
      log("error", `采样失败: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(false);
    }
  }

  const max = Math.max(1, ...(result?.samples.map((s) => s.count) ?? []));

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">采样分析器</span>
        <input
          type="number"
          className="w-20 rounded bg-zinc-800 px-2 py-0.5 text-zinc-200 outline-none"
          value={samples}
          min={10}
          max={5000}
          onChange={(e) => setSamples(Number(e.target.value))}
        />
        <span className="text-zinc-500">样本</span>
        <button className="rounded bg-blue-600 px-2 py-0.5 text-white hover:bg-blue-500 disabled:opacity-40" onClick={run} disabled={!connected || busy}>
          {busy ? "采样中…" : "开始"}
        </button>
      </div>
      <div className="flex-1 overflow-auto px-2 py-1">
        {result?.samples.slice(0, 20).map((s, i) => (
          <div key={s.pc} className="flex items-center gap-2 py-0.5">
            <span className="w-8 text-zinc-600">{i + 1}</span>
            <span className="w-44 truncate text-zinc-200">{symbols[s.pc] ?? `0x${s.pc.toString(16)}`}</span>
            <div className="h-3 flex-1 rounded bg-zinc-800">
              <div className="h-3 rounded bg-blue-500" style={{ width: `${(s.count / max) * 100}%` }} />
            </div>
            <span className="w-16 text-right text-zinc-400">{s.count} ({((s.count / result.total) * 100).toFixed(1)}%)</span>
          </div>
        ))}
        {result && result.total === 0 && <div className="text-zinc-500">未采集到样本。</div>}
        {!result && <div className="mt-2 text-zinc-600">运行目标后点击"开始"进行 PC 采样分析。</div>}
      </div>
    </div>
  );
}
