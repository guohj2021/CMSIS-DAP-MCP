// Symbol explorer: upload ELF/AXF, virtualized list, drag to watch/breakpoint.
import { useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, SymbolItem } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { hex8 } from "../debug/formatters";

export function SymbolsPanel() {
  const [items, setItems] = useState<SymbolItem[]>([]);
  const [total, setTotal] = useState(0);
  const [kind, setKind] = useState("");
  const [pattern, setPattern] = useState("");
  const [loaded, setLoaded] = useState(false);
  const log = useDebugStore((s) => s.log);
  const parentRef = useRef<HTMLDivElement>(null);

  async function upload(file: File) {
    try {
      const r = await api.elfUpload(file);
      setLoaded(true);
      log("info", `已加载 ${r.name ?? file.name}: ${r.symbols} 个符号 (${r.functions} 函数, ${r.variables} 变量)`);
      await refresh();
    } catch (e) {
      log("error", `ELF 加载失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function refresh(k = kind, p = pattern) {
    try {
      const r = await api.symbols({ kind: k || undefined, pattern: p || undefined, limit: 500 });
      setTotal(r.total);
      setItems(r.items);
    } catch (e) {
      log("error", `符号查询失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 22,
    overscan: 20,
  });

  const rows = useMemo(() => virtualizer.getVirtualItems(), [virtualizer]);

  async function onDrop(e: React.DragEvent) {
    e.preventDefault();
    if (e.dataTransfer.files.length > 0) {
      await upload(e.dataTransfer.files[0]);
    }
  }

  return (
    <div className="flex h-full flex-col text-xs" onDragOver={(e) => e.preventDefault()} onDrop={onDrop}>
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <input
          type="file"
          accept=".elf,.axf"
          className="w-40 text-[10px] text-zinc-400 file:mr-2 file:rounded file:border-0 file:bg-zinc-700 file:px-2 file:py-0.5 file:text-zinc-200"
          onChange={(e) => e.target.files?.[0] && upload(e.target.files[0])}
        />
        <select className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-200 outline-none" value={kind} onChange={(e) => { setKind(e.target.value); refresh(e.target.value, pattern); }}>
          <option value="">全部</option>
          <option value="function">函数</option>
          <option value="variable">变量</option>
        </select>
        <input
          className="w-28 rounded bg-zinc-800 px-2 py-0.5 text-zinc-200 outline-none"
          placeholder="搜索…"
          value={pattern}
          onChange={(e) => { setPattern(e.target.value); refresh(kind, e.target.value); }}
        />
        <span className="text-zinc-500">{total} 符号</span>
      </div>
      {!loaded ? (
        <div className="flex-1 rounded border border-dashed border-zinc-700 p-3 text-center text-zinc-500">
          上传 ELF/AXF 或拖入文件；拖拽符号到 Watch 面板
        </div>
      ) : (
        <div ref={parentRef} className="flex-1 overflow-auto">
          <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
            {rows.map((row) => {
              const s = items[row.index];
              return (
                <div
                  key={s.id}
                  style={{ position: "absolute", top: 0, left: 0, width: "100%", height: row.size, transform: `translateY(${row.start}px)` }}
                  className="flex items-center gap-2 px-2 hover:bg-zinc-800/60"
                  draggable
                  onDragStart={(e) => e.dataTransfer.setData("application/x-symbol", String(s.id))}
                >
                  <span className={`w-2 ${s.kind === "function" ? "text-blue-400" : "text-emerald-400"}`}>{s.kind === "function" ? "ƒ" : "v"}</span>
                  <span className="w-28 font-mono text-zinc-400">0x{hex8(s.address).padStart(8, "0")}</span>
                  <span className="flex-1 truncate text-zinc-200">{s.name}</span>
                  <span className="text-zinc-600">{s.size}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
