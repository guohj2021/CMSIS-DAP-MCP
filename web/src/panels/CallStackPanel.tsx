// Call Stack panel: honest placeholder (P6.2 deferred per A5: probe-rs 0.32
// has no unwinder; we never fabricate frames).
import { useDebugStore } from "../store/debugStore";

export function CallStackPanel() {
  const pc = useDebugStore((s) => s.status.pc);
  return (
    <div className="flex h-full flex-col p-2 text-xs">
      <div className="mb-2 text-zinc-300">调用栈</div>
      <div className="rounded border border-dashed border-zinc-700 p-3 text-zinc-500">
        调用栈不可用（需要 unwinder / DWARF CFI，P6.2）。当前不伪造帧。
        {pc !== undefined && pc !== null && (
          <div className="mt-2 font-mono text-zinc-400">PC = 0x{pc.toString(16)}</div>
        )}
      </div>
    </div>
  );
}
