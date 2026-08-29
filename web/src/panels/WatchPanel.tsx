// Watch panel (V0.1 placeholder; full implementation in P4/P6).
export function WatchPanel() {
  return (
    <div className="flex h-full flex-col p-2 text-xs text-zinc-400">
      <div className="mb-2 text-zinc-300">Watch</div>
      <div className="rounded border border-dashed border-zinc-700 p-3 text-center">
        P4: 从符号列表拖拽变量到此处建立 Watch
      </div>
    </div>
  );
}
