// Disassembly / code panel: fetch Thumb instructions, PC highlight, Go To.
import { useEffect, useState } from "react";
import { api, DisasmInsn } from "../api/client";
import * as ops from "../debug/operations";
import { useDebugStore } from "../store/debugStore";
import { parseNumber } from "../debug/formatters";

export function DisassemblyPanel() {
  const pc = useDebugStore((s) => s.status.pc);
  const connected = useDebugStore((s) => s.connected);
  const [addr, setAddr] = useState("0x08000000");
  const [insns, setInsns] = useState<DisasmInsn[]>([]);
  const [note, setNote] = useState("");
  const log = useDebugStore((s) => s.log);

  async function load(target?: number) {
    const base = target ?? parseNumber(addr);
    if (base === null) return;
    setAddr(`0x${base.toString(16)}`);
    try {
      const r = await api.disassembly(base, 24);
      setInsns(r.instructions);
    } catch (e) {
      setNote(e instanceof Error ? e.message : String(e));
      log("error", `反汇编失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  useEffect(() => {
    if (connected) load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connected]);

  // Ctrl+L focuses the Go-To input.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "l") {
        e.preventDefault();
        document.getElementById("goto-addr")?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Frame selection from the Call Stack panel jumps here (DebugContext §9).
  useEffect(() => {
    const onGoto = (e: Event) => {
      const detail = (e as CustomEvent).detail as { address?: number };
      if (detail?.address) load(detail.address);
    };
    window.addEventListener("goto-address", onGoto);
    return () => window.removeEventListener("goto-address", onGoto);
  }, []);

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">Go to 地址 (Ctrl+L)</span>
        <input
          id="goto-addr"
          className="w-32 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
          value={addr}
          onChange={(e) => setAddr(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && load()}
        />
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => load()}>
          反汇编
        </button>
        {pc !== undefined && pc !== null && (
          <button
            className="rounded bg-amber-500/20 px-2 py-0.5 text-amber-300 hover:bg-amber-500/30"
            onClick={() => load(pc)}
          >
            PC = 0x{pc.toString(16)}
          </button>
        )}
      </div>
      <div className="flex-1 overflow-auto px-2 py-1 font-mono">
        {insns.map((i) => {
          const isPc = pc !== undefined && pc !== null && i.address === pc;
          return (
            <div key={i.address} className={`flex gap-2 whitespace-pre ${isPc ? "bg-amber-500/20 text-amber-200" : "text-zinc-300"}`}>
              <span className="w-28 text-zinc-500">0x{i.address.toString(16).toUpperCase().padStart(8, "0")}</span>
              <span className="w-20 text-zinc-600">{i.bytes}</span>
              <span className="w-20 text-blue-300">{i.mnemonic}</span>
              <span className="flex-1">{i.op_str}</span>
              {i.source && <span className="text-emerald-400">{i.source.file.split(/[\\/]/).pop()}:{i.source.line}</span>}
              {i.symbol && <span className="text-emerald-400">{i.symbol}</span>}
              <button
                className="rounded bg-zinc-700 px-1 py-0.5 text-[10px] text-zinc-300 hover:bg-zinc-600"
                title="运行到此处"
                onClick={() => ops.runTo(i.address)}
              >
                运行到
              </button>
            </div>
          );
        })}
        {insns.length === 0 && <div className="mt-2 text-zinc-600">{note || "输入地址开始反汇编"}</div>}
      </div>
    </div>
  );
}
