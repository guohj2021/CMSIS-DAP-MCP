// App shell: WS bootstrap + toolbar + workspace + fault banner.
import { useEffect } from "react";
import { Toolbar } from "./components/Toolbar";
import { DebugWorkspace } from "./workspaces/DebugWorkspace";
import { connectWs, onWsEvent } from "./ws/client";
import { refreshProbes, refreshRegisters, refreshFault } from "./debug/operations";
import { useDebugStore, SessionStatus } from "./store/debugStore";

export default function App() {
  const target = useDebugStore((s) => s.status.target);
  const error = useDebugStore((s) => s.error);

  useEffect(() => {
    connectWs();
    refreshProbes();

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
      <div className="flex-1 overflow-hidden">
        <DebugWorkspace />
      </div>
    </div>
  );
}

