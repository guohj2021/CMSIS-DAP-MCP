// Symbol explorer (V0.1 placeholder; full implementation in P4).
export function SymbolsPanel() {
  return (
    <div className="flex h-full flex-col p-2 text-xs text-zinc-400">
      <div className="mb-2 text-zinc-300">符号</div>
      <div className="rounded border border-dashed border-zinc-700 p-3 text-center">
        P4: 上传 ELF/AXF 后显示函数 / 变量（可拖拽到 Watch）
      </div>
    </div>
  );
}
