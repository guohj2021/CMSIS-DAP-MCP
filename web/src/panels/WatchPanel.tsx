// Watch panel: list, add by drag, refresh, delete.
import { useCallback, useEffect, useState } from "react";
import { api, WatchItem } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { onWsEvent } from "../ws/client";

export function WatchPanel() {
  const [items, setItems] = useState<(WatchItem & { value?: number | null })[]>([]);
  const log = useDebugStore((s) => s.log);

  const refresh = useCallback(async () => {
    try {
      const list = await api.watchList();
      const values = await api.watchRefresh();
      const valueMap = new Map(values.items.map((v) => [v.id, v.value]));
      setItems(list.items.map((w) => ({ ...w, value: valueMap.get(w.id) })));
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    refresh();
    const off = onWsEvent("live_watch_value_changed", (data) => {
      const d = data as { items: { id: number; value?: number | null }[] };
      setItems((prev) =>
        prev.map((w) => {
          const hit = d.items.find((x) => x.id === w.id);
          return hit ? { ...w, value: hit.value ?? undefined } : w;
        })
      );
    });
    return off;
  }, [refresh]);

  async function setRate(id: number, rate_ms: number) {
    try {
      await api.watchPatch(id, { rate_ms });
      setItems((prev) => prev.map((w) => (w.id === id ? { ...w, rate_ms } : w)));
    } catch {
      /* ignore */
    }
  }

  async function onDrop(e: React.DragEvent) {
    e.preventDefault();
    const symbolId = e.dataTransfer.getData("application/x-symbol");
    if (!symbolId) return;
    try {
      await api.watchAdd({ kind: "symbol", symbol_id: Number(symbolId) });
      log("info", "已添加到 Watch");
      await refresh();
    } catch (err) {
      log("error", err instanceof Error ? err.message : String(err));
    }
  }

  function label(w: WatchItem): string {
    if (w.target.kind === "symbol") return `符号 #${w.target.symbol_id}`;
    if (w.target.kind === "address") return `0x${(w.target.address ?? 0).toString(16)}`;
    return w.target.name ?? "?";
  }

  return (
    <div className="flex h-full flex-col text-xs" onDragOver={(e) => e.preventDefault()} onDrop={onDrop}>
      <div className="flex items-center justify-between border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">Watch（从符号面板拖入）</span>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={refresh}>
          刷新
        </button>
      </div>
      <div className="flex-1 overflow-auto px-2 py-1">
        {items.length === 0 && (
          <div className="rounded border border-dashed border-zinc-700 p-3 text-center text-zinc-500">
            将符号拖拽到这里建立 Watch
          </div>
        )}
        {items.map((w) => (
          <div key={w.id} className="flex items-center gap-2 rounded px-1 py-0.5 hover:bg-zinc-800/50">
            <span className="flex-1 text-zinc-300">{label(w)}</span>
            <select
              className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-400 outline-none"
              value={w.rate_ms}
              onChange={(e) => setRate(w.id, Number(e.target.value))}
              title="刷新周期 (Live Watch)"
            >
              <option value={50}>50ms</option>
              <option value={100}>100ms</option>
              <option value={200}>200ms</option>
              <option value={500}>500ms</option>
              <option value={1000}>1s</option>
            </select>
            <span className="font-mono text-amber-300">
              {w.value !== undefined && w.value !== null ? `0x${w.value.toString(16)}` : "—"}
            </span>
            <button
              className="rounded bg-red-900 px-2 py-0.5 text-red-200 hover:bg-red-800"
              onClick={async () => {
                await api.watchDelete(w.id);
                await refresh();
              }}
            >
              删除
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
