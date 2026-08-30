// Debug workspace: Dockview layout presets (Quick / Full), same state.
import { useCallback, useEffect, useRef, useState } from "react";
import { DockviewReact, DockviewReadyEvent, IDockviewPanelProps } from "dockview-react";
import { RegistersPanel } from "../panels/RegistersPanel";
import { MemoryPanel } from "../panels/MemoryPanel";
import { ConsolePanel } from "../panels/ConsolePanel";
import { SymbolsPanel } from "../panels/SymbolsPanel";
import { WatchPanel } from "../panels/WatchPanel";
import { PeripheralPanel } from "../panels/PeripheralPanel";
import { BreakpointsPanel } from "../panels/BreakpointsPanel";
import { RttPanel } from "../panels/RttPanel";
import { EvrPanel } from "../panels/EvrPanel";
import { CallStackPanel } from "../panels/CallStackPanel";
import { LocalsPanel } from "../panels/LocalsPanel";
import { SwoPanel } from "../panels/SwoPanel";
import { SourceViewerPanel } from "../panels/SourceViewerPanel";
import { ProfilerPanel } from "../panels/ProfilerPanel";
import { DisassemblyPanel } from "../panels/DisassemblyPanel";

const components: Record<string, React.FC<IDockviewPanelProps<{ title?: string }>>> = {
  registers: () => <RegistersPanel />,
  memory: () => <MemoryPanel />,
  console: () => <ConsolePanel />,
  symbols: () => <SymbolsPanel />,
  watch: () => <WatchPanel />,
  peripheral: () => <PeripheralPanel />,
  breakpoints: () => <BreakpointsPanel />,
  rtt: () => <RttPanel />,
  evr: () => <EvrPanel />,
  callstack: () => <CallStackPanel />,
  locals: () => <LocalsPanel />,
  swo: () => <SwoPanel />,
  source: () => <SourceViewerPanel />,
  profiler: () => <ProfilerPanel />,
  disassembly: () => <DisassemblyPanel />,
};

type Preset = "quick" | "full";

const LAYOUT_KEY = "cmsis-dap-layout";

export function DebugWorkspace() {
  const apiRef = useRef<DockviewReadyEvent["api"] | null>(null);
  // builtRef makes onReady idempotent (StrictMode-safe): the layout is built
  // exactly once per mount instead of being cleared/rebuilt 2-3 times.
  const builtRef = useRef(false);
  const saveSubRef = useRef<{ dispose: () => void } | null>(null);
  const [preset, setPreset] = useState<Preset>("quick");

  const applyPreset = useCallback((p: Preset, api: DockviewReadyEvent["api"]) => {
    api.clear();
    if (p === "quick") {
      api.addPanel({ id: "registers", component: "registers", title: "寄存器" });
      api.addPanel({
        id: "disasm",
        component: "disassembly",
        title: "代码 / 反汇编",
        position: { referencePanel: "registers", direction: "left" },
      });
      api.addPanel({
        id: "watch",
        component: "watch",
        title: "Watch",
        position: { referencePanel: "registers", direction: "below" },
      });
      api.addPanel({
        id: "console",
        component: "console",
        title: "控制台",
        position: { referencePanel: "watch", direction: "below" },
      });
    } else {
      api.addPanel({ id: "symbols", component: "symbols", title: "符号" });
      api.addPanel({
        id: "disasm",
        component: "disassembly",
        title: "代码 / 反汇编",
        position: { referencePanel: "symbols", direction: "right" },
      });
      api.addPanel({
        id: "source",
        component: "source",
        title: "源码",
        position: { referencePanel: "disasm", direction: "below" },
      });
      api.addPanel({
        id: "registers",
        component: "registers",
        title: "寄存器",
        position: { referencePanel: "disasm", direction: "right" },
      });
      api.addPanel({
        id: "watch",
        component: "watch",
        title: "Watch",
        position: { referencePanel: "disasm", direction: "below" },
      });
      api.addPanel({
        id: "memory",
        component: "memory",
        title: "内存",
        position: { referencePanel: "watch", direction: "below" },
      });
      api.addPanel({
        id: "peripheral",
        component: "peripheral",
        title: "外设",
        position: { referencePanel: "memory", direction: "right" },
      });
      api.addPanel({
        id: "breakpoints",
        component: "breakpoints",
        title: "断点",
        position: { referencePanel: "peripheral", direction: "right" },
      });
      api.addPanel({
        id: "rtt",
        component: "rtt",
        title: "RTT",
        position: { referencePanel: "breakpoints", direction: "right" },
      });
      api.addPanel({
        id: "evr",
        component: "evr",
        title: "EVR",
        position: { referencePanel: "rtt", direction: "right" },
      });
      api.addPanel({
        id: "callstack",
        component: "callstack",
        title: "调用栈",
        position: { referencePanel: "evr", direction: "right" },
      });
      api.addPanel({
        id: "locals",
        component: "locals",
        title: "Locals",
        position: { referencePanel: "callstack", direction: "right" },
      });
      api.addPanel({
        id: "swo",
        component: "swo",
        title: "SWO",
        position: { referencePanel: "locals", direction: "right" },
      });
      api.addPanel({
        id: "profiler",
        component: "profiler",
        title: "Profiler",
        position: { referencePanel: "swo", direction: "right" },
      });
      api.addPanel({
        id: "console",
        component: "console",
        title: "控制台",
        position: { referencePanel: "peripheral", direction: "below" },
      });
    }
  }, []);

  // Build the layout exactly once per mount: restore the saved layout when
  // present (fall back to the preset otherwise). The save subscription is
  // registered here too, so it is always attached - the old code returned
  // early on restore and leaked/dropped the subscription.
  const onReady = useCallback(
    (event: DockviewReadyEvent) => {
      const api = event.api;
      apiRef.current = api;
      if (!saveSubRef.current) {
        saveSubRef.current = api.onDidLayoutChange(() => {
          try {
            const json = api.toJSON();
            localStorage.setItem(LAYOUT_KEY, JSON.stringify(json));
          } catch {
            /* ignore */
          }
        });
      }
      if (builtRef.current) return;
      builtRef.current = true;
      try {
        const saved = localStorage.getItem(LAYOUT_KEY);
        if (saved) {
          api.fromJSON(JSON.parse(saved));
          return;
        }
      } catch {
        /* fall back to the preset */
      }
      applyPreset(preset, api);
    },
    [applyPreset, preset]
  );

  // Clean up on unmount so a remount (Debug/Flash switch, StrictMode) rebuilds
  // from scratch instead of ending up with a half-initialised layout.
  useEffect(() => {
    return () => {
      saveSubRef.current?.dispose();
      saveSubRef.current = null;
      builtRef.current = false;
      apiRef.current = null;
    };
  }, []);

  function switchPreset(p: Preset) {
    setPreset(p);
    const api = apiRef.current;
    if (api) applyPreset(p, api);
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-2 py-1 text-xs">
        <span className="text-zinc-400">工作区</span>
        <button
          className={`rounded px-2 py-0.5 ${preset === "quick" ? "bg-blue-600 text-white" : "bg-zinc-700 text-zinc-300"}`}
          onClick={() => switchPreset("quick")}
        >
          Quick Debug
        </button>
        <button
          className={`rounded px-2 py-0.5 ${preset === "full" ? "bg-blue-600 text-white" : "bg-zinc-700 text-zinc-300"}`}
          onClick={() => switchPreset("full")}
        >
          Full Debug
        </button>
      </div>
      <div className="flex-1">
        <DockviewReact
          components={components}
          onReady={onReady}
        />
      </div>
    </div>
  );
}
