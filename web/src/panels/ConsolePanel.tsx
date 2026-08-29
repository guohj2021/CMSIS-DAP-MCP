// Console panel: operation log + fault alerts.
import { useDebugStore } from "../store/debugStore";

export function ConsolePanel() {
  const lines = useDebugStore((s) => s.console);
  const clearConsole = useDebugStore((s) => s.clearConsole);

  const color = (level: string) =>
    level === "error" ? "text-red-400" : level === "warn" ? "text-amber-300" : "text-zinc-300";

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center justify-between border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">控制台</span>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={clearConsole}>
          清空
        </button>
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
