// App shell: WS bootstrap + toolbar + workspace + fault banner.
import { useEffect, useState } from "react";
import { Toolbar } from "./components/Toolbar";
import { DebugWorkspace } from "./workspaces/DebugWorkspace";
import { FlashWorkspace } from "./workspaces/FlashWorkspace";
import { connectWs, onWsEvent } from "./ws/client";
import { refreshProbes, refreshRegisters, refreshFault } from "./debug/operations";
import { useDebugStore, SessionStatus } from "./store/debugStore";

export default function App() {
  const target = useDebugStore((s) => s.status.target);
  const error = useDebugStore((s) => s.error);
  const [workspace, setWorkspace] = useState<"debug" | "flash">("debug");

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
            useDebugStore.getState().setStatus({
              target: coreState === "halted" ? "halted" : "running",
              pc: j.pc ?? j.status?.pc ?? null,
              reason: j.reason ?? j.status?.halt_reason ?? null,
            });
          }
          if (coreState === "halted") {
            refreshRegisters();
            refreshFault();
          }
        }
      } catch {
        /* ignore transient errors */
      }
    }, 500);

    const offReady = onWsEvent("ready", (data) => {
      const d = data as unknown as Partial<SessionStatus>;
      useDebugStore.getState().setStatus(d);
      if (d.server === "ready") {
        refreshRegisters();
        refreshFault();
      }
    });
    const offState = onWsEvent("target_state_changed", (data) => {
      const d = data as unknown as Partial<SessionStatus>;
      useDebugStore.getState().setStatus(d);
      if (d.server === "ready") {
        refreshRegisters();
        refreshFault();
      }
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
    </div>
  );
}

