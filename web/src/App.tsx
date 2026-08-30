// App shell: WS bootstrap + toolbar + workspace + fault banner.
import { useEffect, useState } from "react";
import { Toolbar } from "./components/Toolbar";
import { DebugWorkspace } from "./workspaces/DebugWorkspace";
import { FlashWorkspace } from "./workspaces/FlashWorkspace";
import { CommandPalette } from "./components/CommandPalette";
import { connectWs, onWsEvent } from "./ws/client";
import { refreshProbes, refreshRegisters, refreshFault } from "./debug/operations";
import { useDebugStore, SessionStatus } from "./store/debugStore";

export default function App() {
  const target = useDebugStore((s) => s.status.target);
  const error = useDebugStore((s) => s.error);
  const [workspace, setWorkspace] = useState<"debug" | "flash">("debug");
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        setPaletteOpen(true);
      }
    };
    const onWs = (e: Event) => {
      const detail = (e as CustomEvent).detail as { workspace?: string };
      if (detail?.workspace === "flash") setWorkspace("flash");
      if (detail?.workspace === "debug") setWorkspace("debug");
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("open-workspace", onWs);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("open-workspace", onWs);
    };
  }, []);

  useEffect(() => {
    connectWs();
    refreshProbes();

    // Poll status while connected so target state (e.g. breakpoint halt)
    // stays in sync even without a WS event.
    const pollTimer = setInterval(async () => {
      if (!useDebugStore.getState().connected) return;
      try {
        const r = await fetch("/api/status");
        if (r.ok) {
          const j = await r.json();
          const coreState = j.status?.state as string | undefined;
          if (coreState === "halted" || coreState === "running") {
            const target = coreState === "halted" ? "halted" : "running";
            const pc = j.pc ?? j.status?.pc ?? null;
            const reason = j.reason ?? j.status?.halt_reason ?? null;
            const cur = useDebugStore.getState().status;
            // Only push a store update when something actually changed;
            // this keeps status-object churn (and re-renders) minimal.
            const changed =
              cur.target !== target ||
              (cur.pc ?? null) !== (pc ?? null) ||
              (cur.reason ?? null) !== (reason ?? null);
            if (changed) {
              useDebugStore.getState().setStatus({ target, pc, reason });
            }
            // Refresh registers/fault only when the target *enters* halted,
            // not on every poll cycle (avoids needless probe traffic).
            if (target === "halted" && cur.target !== "halted") {
              refreshRegisters();
              refreshFault();
            }
          }
        }
      } catch {
        /* ignore transient errors */
      }
    }, 500);

    // IMPORTANT: WS events only update the store. Registers/fault are
    // refreshed by the bounded status poller and explicit actions — never
    // from inside this handler, otherwise every executor op (which emits a
    // target_state_changed) would trigger another refresh and cascade.
    const offReady = onWsEvent("ready", (data) => {
      const d = data as unknown as Partial<SessionStatus>;
      useDebugStore.getState().setStatus(d);
    });
    const offState = onWsEvent("target_state_changed", (data) => {
      const d = data as unknown as Partial<SessionStatus>;
      useDebugStore.getState().setStatus(d);
    });
    const offLost = onWsEvent("probe_lost", (data) => {
      const d = data as { message?: string };
      useDebugStore.getState().log("error", `探针丢失: ${d?.message ?? ""}`);
      useDebugStore.getState().setStatus({ server: "disconnected", target: "unknown" });
    });
    const offClosed = onWsEvent("ws_closed", () => {
      useDebugStore.getState().setWsConnected(false);
    });

    return () => {
      clearInterval(pollTimer);
      offReady();
      offState();
      offLost();
      offClosed();
    };
  }, []);

  return (
    <div className="flex h-screen flex-col bg-zinc-950 text-zinc-200">
      <Toolbar />
      {target === "fault" && (
        <div className="flex items-center gap-2 border-b border-red-800 bg-red-950 px-3 py-1 text-xs text-red-300">
          ⚠ FAULT：目标处于异常状态，请查看 Fault 面板 / 快照
          {error ? ` (${error})` : ""}
        </div>
      )}
      <div className="flex items-center gap-1 border-b border-zinc-800 bg-zinc-900 px-2 py-0.5 text-xs">
        <button className={`rounded px-2 py-0.5 ${workspace === "debug" ? "bg-blue-600 text-white" : "text-zinc-400 hover:text-zinc-200"}`} onClick={() => setWorkspace("debug")}>
          Debug
        </button>
        <button className={`rounded px-2 py-0.5 ${workspace === "flash" ? "bg-blue-600 text-white" : "text-zinc-400 hover:text-zinc-200"}`} onClick={() => setWorkspace("flash")}>
          Flash
        </button>
      </div>
      <div className="flex-1 overflow-hidden">
        {workspace === "debug" ? <DebugWorkspace /> : <FlashWorkspace />}
      </div>
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} />
    </div>
  );
}

