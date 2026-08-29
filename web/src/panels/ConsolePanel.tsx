// Console panel: operation log + fault alerts + expression evaluator.
import { useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";

export function ConsolePanel() {
  const lines = useDebugStore((s) => s.console);
  const clearConsole = useDebugStore((s) => s.clearConsole);
  const connected = useDebugStore((s) => s.connected);
  const [expr, setExpr] = useState("");
  const [result, setResult] = useState("");

  const color = (level: string) =>
    level === "error" ? "text-red-400" : level === "warn" ? "text-amber-300" : "text-zinc-300";

  async function evalExpr() {
    if (!expr.trim()) return;
    try {
      const r = await api.expression(expr);
      setResult(`= ${r.result.text}`);
    } catch (e) {
      setResult(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center justify-between border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">控制台</span>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={clearConsole}>
          清空
        </button>
      </div>
      <div className="flex items-center gap-2 border-b border-zinc-800 px-2 py-1">
        <span className="text-zinc-500">表达式</span>
        <input
          className="flex-1 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
          placeholder="例: *(0x40010810) | $pc | main + 4 | (0x20000000 >> 2)"
          value={expr}
          onChange={(e) => setExpr(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && evalExpr()}
          disabled={!connected}
        />
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600 disabled:opacity-40" onClick={evalExpr} disabled={!connected}>
          求值
        </button>
        <span className="font-mono text-emerald-300">{result}</span>
      </div>
      <div className="flex-1 overflow-auto px-2 py-1 font-mono">
        {lines.map((l, i) => (
          <div key={i} className={color(l.level)}>
            <span className="text-zinc-600">[{l.ts}]</span> {l.text}
          </div>
        ))}
      </div>
    </div>
  );
}
