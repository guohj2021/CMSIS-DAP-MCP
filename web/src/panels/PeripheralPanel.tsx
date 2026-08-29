// Peripheral explorer (V0.1 placeholder; full implementation in P4/P5).
export function PeripheralPanel() {
  return (
    <div className="flex h-full flex-col p-2 text-xs text-zinc-400">
      <div className="mb-2 text-zinc-300">外设</div>
      <div className="rounded border border-dashed border-zinc-700 p-3 text-center">
        P4: 上传 SVD 后浏览外设寄存器与位域
      </div>
    </div>
  );
}
