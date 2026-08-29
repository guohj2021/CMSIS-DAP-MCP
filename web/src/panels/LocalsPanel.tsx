// Locals panel: honest placeholder (P6.3 deferred: DWARF variable evaluation).
export function LocalsPanel() {
  return (
    <div className="flex h-full flex-col p-2 text-xs">
      <div className="mb-2 text-zinc-300">局部变量</div>
      <div className="rounded border border-dashed border-zinc-700 p-3 text-zinc-500">
        Locals 不可用（需要 DWARF 变量求值，P6.3）。当前不伪造。
      </div>
    </div>
  );
}
