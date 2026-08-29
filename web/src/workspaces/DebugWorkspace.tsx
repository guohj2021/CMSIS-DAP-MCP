// Debug workspace: Dockview layout presets (Quick / Full), same state.
import { useEffect, useRef, useState } from "react";
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

export function DebugWorkspace() {
  const apiRef = useRef<DockviewReadyEvent["api"] | null>(null);
  const [preset, setPreset] = useState<Preset>("quick");

  function applyPreset(p: Preset, api: DockviewReadyEvent["api"]) {
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
  }

  useEffect(() => {
    if (apiRef.current) applyPreset(preset, apiRef.current);
  }, [preset]);

  // Layout persistence: save on change, restore on load (P7a).
  useEffect(() => {
    const api = apiRef.current;
    if (!api) return;
    const save = () => {
      try {
        const json = api.toJSON();
        localStorage.setItem("cmsis-dap-layout", JSON.stringify(json));
      } catch {
        /* ignore */
      }
    };
    const sub = api.onDidLayoutChange(() => save());
    const restore = localStorage.getItem("cmsis-dap-layout");
    if (restore) {
      try {
        api.fromJSON(JSON.parse(restore));
        return;
      } catch {
        /* fall back to preset */
      }
    }
    return () => sub.dispose();
  }, []);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-zinc-800 bg-zinc-900 px-2 py-1 text-xs">
        <span className="text-zinc-400">工作区</span>
        <button
          className={`rounded px-2 py-0.5 ${preset === "quick" ? "bg-blue-600 text-white" : "bg-zinc-700 text-zinc-300"}`}
          onClick={() => setPreset("quick")}
        >
          Quick Debug
        </button>
        <button
          className={`rounded px-2 py-0.5 ${preset === "full" ? "bg-blue-600 text-white" : "bg-zinc-700 text-zinc-300"}`}
          onClick={() => setPreset("full")}
        >
          Full Debug
        </button>
      </div>
      <div className="flex-1">
        <DockviewReact
          components={components}
          onReady={(event) => {
            apiRef.current = event.api;
            applyPreset(preset, event.api);
          }}
        />
      </div>
    </div>
  );
}


