// Command Palette: Ctrl+Shift+P, fuzzy filter over the command registry.
import { useEffect, useMemo, useState } from "react";
import { commands, runCommand } from "../debug/commands";

export function CommandPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);

  const filtered = useMemo(() => {
    const query = q.trim().toLowerCase();
    const list = commands.filter((c) => c.enabled());
    if (!query) return list;
    return list.filter((c) => (c.label + " " + c.keywords).toLowerCase().includes(query));
  }, [q]);

  useEffect(() => {
    setSel(0);
  }, [q]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowDown") { e.preventDefault(); setSel((s) => Math.min(s + 1, filtered.length - 1)); }
      if (e.key === "ArrowUp") { e.preventDefault(); setSel((s) => Math.max(s - 1, 0)); }
      if (e.key === "Enter" && filtered[sel]) {
        runCommand(filtered[sel].id);
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, filtered, sel, onClose]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-24" onClick={onClose}>
      <div className="w-130 max-w-lg rounded-lg border border-zinc-700 bg-zinc-900 shadow-xl" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          className="w-full rounded-t-lg border-b border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 outline-none"
          placeholder="输入命令… (Ctrl+Shift+P)"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <div className="max-h-80 overflow-auto py-1">
          {filtered.length === 0 && <div className="px-3 py-2 text-sm text-zinc-500">无匹配命令</div>}
          {filtered.map((c, i) => (
            <div
              key={c.id}
              className={`cursor-pointer px-3 py-1.5 text-sm ${i === sel ? "bg-blue-600 text-white" : "text-zinc-300"}`}
              onMouseEnter={() => setSel(i)}
              onClick={() => { runCommand(c.id); onClose(); }}
            >
              {c.label}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
