// Source viewer: shows the source file for the current PC / selected frame
// with the current line highlighted (P7b). Reads source from the host when
// the DWARF path is accessible.
import { useEffect, useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";

interface SourceData {
  file: string;
  total: number;
  start: number;
  current: number;
  lines: string[];
}

export function SourceViewerPanel() {
  const pc = useDebugStore((s) => s.status.pc);
  const [src, setSrc] = useState<SourceData | null>(null);

  async function load(addr?: number) {
    const target = addr ?? pc;
    if (target === undefined || target === null) {
      setSrc(null);
      return;
    }
    try {
      const desc = await api.addressDescribe(target);
      const sl = (desc as { source_location?: { file: string; line: number } }).source_location;
      if (!sl) {
        setSrc(null);
        return;
      }
      const r = await api.source(sl.file, sl.line);
      setSrc(r);
    } catch {
      setSrc(null);
    }
  }

  useEffect(() => {
    if (pc !== undefined && pc !== null) load(pc);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pc]);

  // Follow frame selection / go-to from call stack & disassembly.
  useEffect(() => {
    const onGoto = (e: Event) => {
      const detail = (e as CustomEvent).detail as { address?: number };
      if (detail?.address) load(detail.address);
    };
    window.addEventListener("goto-address", onGoto);
    return () => window.removeEventListener("goto-address", onGoto);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="border-b border-zinc-700 px-2 py-1 text-zinc-400">
        {src ? `${src.file.split(/[\\/]/).pop()}:${src.current}` : "源码（DWARF 定位）"}
      </div>
      <div className="flex-1 overflow-auto px-2 py-1 font-mono">
        {src ? (
          src.lines.map((l, i) => {
            const lineNo = src.start + i;
            const isCurrent = lineNo === src.current;
            return (
              <div key={lineNo} className={`flex ${isCurrent ? "bg-amber-500/20" : ""}`}>
                <span className="w-10 select-none pr-2 text-right text-zinc-600">{lineNo}</span>
                <span className={`flex-1 whitespace-pre ${isCurrent ? "text-amber-200" : "text-zinc-300"}`}>{l || " "}</span>
              </div>
            );
          })
        ) : (
          <div className="mt-2 text-zinc-600">
            {pc !== undefined && pc !== null
              ? "无源码信息（固件无 DWARF 或源码不在本机）"
              : "未连接"}
          </div>
        )}
      </div>
    </div>
  );
}
