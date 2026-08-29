// Locals panel: real DWARF locals/arguments with expandable struct/array.
import { useEffect, useState } from "react";
import { api, LocalValue } from "../api/client";
import { useDebugStore } from "../store/debugStore";

function LocalRow({ v, depth }: { v: LocalValue; depth: number }) {
  const [open, setOpen] = useState(false);
  const hasChildren = v.children.length > 0;
  return (
    <div>
      <div
        className="flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 hover:bg-zinc-800/50"
        style={{ paddingLeft: 4 + depth * 12 }}
        onClick={() => hasChildren && setOpen(!open)}
      >
        {hasChildren ? <span className="w-3 text-zinc-500">{open ? "▾" : "▸"}</span> : <span className="w-3" />}
        <span className="text-zinc-200">{v.name}</span>
        <span className="text-zinc-500">{v.type_name}</span>
        <span className="ml-auto font-mono text-emerald-300">{v.value}</span>
        {v.address !== undefined && v.address !== null && (
          <span className="font-mono text-zinc-600">0x{v.address.toString(16)}</span>
        )}
      </div>
      {open &&
        v.children.map((c, i) => <LocalRow key={i} v={c} depth={depth + 1} />)}
    </div>
  );
}

export function LocalsPanel() {
  const halted = useDebugStore((s) => s.status.target === "halted");
  const [locals, setLocals] = useState<LocalValue[]>([]);
  const [available, setAvailable] = useState<boolean | null>(null);
  const [frame, setFrame] = useState<{ pc: number } | null>(null);
  const log = useDebugStore((s) => s.log);

  async function refresh() {
    try {
      const r = await api.locals();
      setAvailable(r.available);
      setLocals(r.locals);
      setFrame(null);
    } catch (e) {
      setAvailable(false);
      log("error", `Locals 获取失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function loadFrame(pc: number, registers: Record<string, number>) {
    try {
      const r = await api.localsAt(pc, registers);
      setAvailable(r.available);
      setLocals(r.locals);
      setFrame({ pc });
    } catch (e) {
      log("error", `帧 Locals 失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  // Frame selection from the Call Stack panel evaluates locals in that frame.
  useEffect(() => {
    const onFrame = (e: Event) => {
      const d = (e as CustomEvent).detail as { pc?: number; registers?: Record<string, number> };
      if (d?.pc && d.registers) loadFrame(d.pc, d.registers);
    };
    window.addEventListener("frame-selected", onFrame);
    return () => window.removeEventListener("frame-selected", onFrame);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (halted) refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [halted]);

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center justify-between border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">
          局部变量 / 参数{frame ? `（帧 0x${frame.pc.toString(16)}）` : ""}
        </span>
        <div className="flex items-center gap-2">
          {frame && (
            <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={refresh}>
              当前帧
            </button>
          )}
          <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600 disabled:opacity-40" onClick={refresh} disabled={!halted}>
            刷新
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-auto px-1 py-1">
        {available === false && (
          <div className="rounded border border-dashed border-zinc-700 p-3 text-zinc-500">
            Locals 不可用：固件无 DWARF 变量信息或目标未暂停。不伪造。
          </div>
        )}
        {available && locals.length === 0 && <div className="text-zinc-500">当前函数无可见局部变量。</div>}
        {locals.map((v, i) => (
          <LocalRow key={i} v={v} depth={0} />
        ))}
      </div>
    </div>
  );
}
