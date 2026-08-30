// Debug workspace: Keil-style Dockview layout + Window menu + layout configs.
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

type Direction = "left" | "right" | "above" | "below" | "within";

const LAYOUT_KEY = "cmsis-dap-layout-v2";
const CONFIGS_KEY = "cmsis-dap-configs";

interface PanelAdd {
  id: string;
  title: string;
  component: string;
  position?: { referencePanel: string; direction: Direction };
}

interface PanelSpec {
  id: string;
  title: string;
  component: string;
  /** Default placement anchor used when a window is opened from the menu. */
  anchor?: string;
  direction?: Direction;
}

// All windows available to the Window menu, with their default placement.
const PANEL_SPECS: PanelSpec[] = [
  { id: "registers", title: "寄存器", component: "registers", anchor: "disasm", direction: "left" },
  { id: "symbols", title: "符号", component: "symbols", anchor: "registers", direction: "below" },
  { id: "disasm", title: "代码 / 反汇编", component: "disassembly", anchor: "registers", direction: "right" },
  { id: "source", title: "源码", component: "source", anchor: "disasm", direction: "below" },
  { id: "watch", title: "Watch", component: "watch", anchor: "disasm", direction: "right" },
  { id: "memory", title: "内存", component: "memory", anchor: "source", direction: "below" },
  { id: "peripheral", title: "外设", component: "peripheral", anchor: "memory", direction: "within" },
  { id: "breakpoints", title: "断点", component: "breakpoints", anchor: "memory", direction: "within" },
  { id: "callstack", title: "调用栈", component: "callstack", anchor: "memory", direction: "within" },
  { id: "locals", title: "Locals", component: "locals", anchor: "memory", direction: "within" },
  { id: "rtt", title: "RTT", component: "rtt", anchor: "memory", direction: "within" },
  { id: "evr", title: "EVR", component: "evr", anchor: "memory", direction: "within" },
  { id: "swo", title: "SWO", component: "swo", anchor: "memory", direction: "within" },
  { id: "profiler", title: "采样分析器", component: "profiler", anchor: "memory", direction: "within" },
  { id: "console", title: "控制台", component: "console", anchor: "memory", direction: "below" },
];

// Keil µVision-inspired presets. Full puts registers/symbols on the left,
// code in the centre, watch on the right, a tabbed bottom strip and the
// command console at the very bottom.
const PRESETS: Record<Preset, PanelAdd[]> = {
  quick: [
    { id: "registers", title: "寄存器", component: "registers" },
    {
      id: "disasm",
      title: "代码 / 反汇编",
      component: "disassembly",
      position: { referencePanel: "registers", direction: "left" },
    },
    {
      id: "watch",
      title: "Watch",
      component: "watch",
      position: { referencePanel: "registers", direction: "below" },
    },
    {
      id: "console",
      title: "控制台",
      component: "console",
      position: { referencePanel: "watch", direction: "below" },
    },
  ],
  full: [
    { id: "registers", title: "寄存器", component: "registers" },
    {
      id: "symbols",
      title: "符号",
      component: "symbols",
      position: { referencePanel: "registers", direction: "below" },
    },
    {
      id: "disasm",
      title: "代码 / 反汇编",
      component: "disassembly",
      position: { referencePanel: "registers", direction: "right" },
    },
    {
      id: "source",
      title: "源码",
      component: "source",
      position: { referencePanel: "disasm", direction: "below" },
    },
    {
      id: "watch",
      title: "Watch",
      component: "watch",
      position: { referencePanel: "disasm", direction: "right" },
    },
    {
      id: "memory",
      title: "内存",
      component: "memory",
      position: { referencePanel: "source", direction: "below" },
    },
    { id: "peripheral", title: "外设", component: "peripheral", position: { referencePanel: "memory", direction: "within" } },
    { id: "breakpoints", title: "断点", component: "breakpoints", position: { referencePanel: "memory", direction: "within" } },
    { id: "callstack", title: "调用栈", component: "callstack", position: { referencePanel: "memory", direction: "within" } },
    { id: "locals", title: "Locals", component: "locals", position: { referencePanel: "memory", direction: "within" } },
    { id: "rtt", title: "RTT", component: "rtt", position: { referencePanel: "memory", direction: "within" } },
    { id: "evr", title: "EVR", component: "evr", position: { referencePanel: "memory", direction: "within" } },
    { id: "swo", title: "SWO", component: "swo", position: { referencePanel: "memory", direction: "within" } },
    { id: "profiler", title: "采样分析器", component: "profiler", position: { referencePanel: "memory", direction: "within" } },
    {
      id: "console",
      title: "控制台",
      component: "console",
      position: { referencePanel: "memory", direction: "below" },
    },
  ],
};

const PRESET_LABELS: Record<Preset, string> = {
  quick: "Quick Debug",
  full: "Full Debug（Keil 风格）",
};

function listConfigs(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(CONFIGS_KEY) || "{}");
  } catch {
    return {};
  }
}

export function DebugWorkspace() {
  const apiRef = useRef<DockviewReadyEvent["api"] | null>(null);
  // builtRef makes onReady idempotent (StrictMode-safe): the layout is built
  // exactly once per mount instead of being cleared/rebuilt repeatedly.
  const builtRef = useRef(false);
  const saveSubRef = useRef<{ dispose: () => void } | null>(null);
  const [preset, setPreset] = useState<Preset>("full");
  const [visible, setVisible] = useState<Set<string>>(new Set());
  const [openMenu, setOpenMenu] = useState<"layout" | "window" | "config" | null>(null);
  const [configs, setConfigs] = useState<Record<string, string>>(listConfigs);
  const [saving, setSaving] = useState(false);
  const [configName, setConfigName] = useState("");
  const menuBarRef = useRef<HTMLDivElement | null>(null);

  const syncVisible = useCallback((api: DockviewReadyEvent["api"]) => {
    setVisible(new Set(api.panels.map((p) => p.id)));
  }, []);

  const addPanel = useCallback((api: DockviewReadyEvent["api"], spec: PanelSpec) => {
    if (api.getPanel(spec.id)) return;
    const anchor = spec.anchor ? api.getPanel(spec.anchor) : undefined;
    if (anchor && spec.direction) {
      api.addPanel({
        id: spec.id,
        component: spec.component,
        title: spec.title,
        position: { referencePanel: spec.anchor!, direction: spec.direction },
      });
    } else if (api.panels.length > 0) {
      // Fallback: append as a tab in the first group.
      api.addPanel({
        id: spec.id,
        component: spec.component,
        title: spec.title,
        position: { referencePanel: api.panels[0].id, direction: "within" },
      });
    } else {
      api.addPanel({ id: spec.id, component: spec.component, title: spec.title });
    }
  }, []);

  const applyPreset = useCallback((p: Preset, api: DockviewReadyEvent["api"]) => {
    api.clear();
    for (const panel of PRESETS[p]) {
      api.addPanel(panel);
    }
  }, []);

  // Build the layout exactly once per mount: restore the saved layout when
  // present (fall back to the preset otherwise). The save + visibility
  // subscription is registered here too, so it is always attached.
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
          syncVisible(api);
        });
      }
      if (builtRef.current) return;
      builtRef.current = true;
      try {
        const saved = localStorage.getItem(LAYOUT_KEY);
        if (saved) {
          api.fromJSON(JSON.parse(saved));
          syncVisible(api);
          return;
        }
      } catch {
        /* fall back to the preset */
      }
      applyPreset(preset, api);
      syncVisible(api);
    },
    [applyPreset, preset, syncVisible]
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

  // Close the open menu when clicking anywhere outside the menu bar (avoids
  // a full-screen backdrop that would swallow clicks on sibling menu buttons).
  useEffect(() => {
    if (!openMenu) return;
    const onDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (menuBarRef.current && !menuBarRef.current.contains(target)) {
        closeMenu();
      }
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openMenu]);

  function closeMenu() {
    setOpenMenu(null);
    setSaving(false);
    setConfigName("");
  }

  function switchPreset(p: Preset) {
    setPreset(p);
    closeMenu();
    const api = apiRef.current;
    if (api) applyPreset(p, api);
  }

  function togglePanel(id: string) {
    const api = apiRef.current;
    if (!api) return;
    const spec = PANEL_SPECS.find((s) => s.id === id);
    if (!spec) return;
    const panel = api.getPanel(id);
    if (panel) {
      panel.api.close();
    } else {
      addPanel(api, spec);
    }
  }

  function saveConfig() {
    const name = configName.trim();
    const api = apiRef.current;
    if (!name || !api) return;
    const all = listConfigs();
    all[name] = JSON.stringify(api.toJSON());
    localStorage.setItem(CONFIGS_KEY, JSON.stringify(all));
    setConfigs(all);
    closeMenu();
  }

  function loadConfig(name: string) {
    const api = apiRef.current;
    if (!api) return;
    const json = listConfigs()[name];
    if (!json) return;
    try {
      api.fromJSON(JSON.parse(json));
      closeMenu();
    } catch {
      /* ignore */
    }
  }

  function deleteConfig(name: string) {
    const all = listConfigs();
    delete all[name];
    localStorage.setItem(CONFIGS_KEY, JSON.stringify(all));
    setConfigs(all);
  }

  function resetDefault() {
    const api = apiRef.current;
    if (!api) return;
    applyPreset(preset, api);
    closeMenu();
  }

  return (
    <div className="flex h-full flex-col">
      <div ref={menuBarRef} className="flex items-center gap-1 border-b border-zinc-800 bg-zinc-900 px-2 py-0.5 text-xs">
        <Dropdown label="布局" open={openMenu === "layout"} onToggle={() => setOpenMenu(openMenu === "layout" ? null : "layout")}>
          {(Object.keys(PRESETS) as Preset[]).map((p) => (
            <MenuItem key={p} onClick={() => switchPreset(p)} active={preset === p}>
              {PRESET_LABELS[p]}
            </MenuItem>
          ))}
          <div className="my-1 border-t border-zinc-700" />
          <MenuItem onClick={resetDefault}>恢复默认布局</MenuItem>
        </Dropdown>

        <Dropdown label="窗口" open={openMenu === "window"} onToggle={() => setOpenMenu(openMenu === "window" ? null : "window")}>
          {PANEL_SPECS.map((s) => (
            <MenuItem key={s.id} onClick={() => togglePanel(s.id)} checked={visible.has(s.id)}>
              {s.title}
            </MenuItem>
          ))}
        </Dropdown>

        <Dropdown label="配置" open={openMenu === "config"} onToggle={() => setOpenMenu(openMenu === "config" ? null : "config")}>
          <MenuItem onClick={() => setSaving(true)}>保存当前布局…</MenuItem>
          {saving && (
            <div className="flex items-center gap-1 px-2 py-1">
              <input
                autoFocus
                className="w-32 rounded border border-zinc-700 bg-zinc-800 px-2 py-0.5 text-zinc-100 outline-none"
                placeholder="配置名称"
                value={configName}
                onChange={(e) => setConfigName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && saveConfig()}
              />
              <button className="rounded bg-blue-600 px-2 py-0.5 text-white hover:bg-blue-500" onClick={saveConfig}>
                保存
              </button>
            </div>
          )}
          <div className="my-1 border-t border-zinc-700" />
          {Object.keys(configs).length === 0 && (
            <div className="px-3 py-1 text-zinc-500">暂无已保存配置</div>
          )}
          {Object.keys(configs).map((name) => (
            <div key={name} className="group flex items-center px-1">
              <button
                className="flex-1 rounded px-2 py-1 text-left text-zinc-300 hover:bg-zinc-800"
                onClick={() => loadConfig(name)}
                title={`加载配置: ${name}`}
              >
                {name}
              </button>
              <button
                className="rounded px-1.5 py-1 text-zinc-500 hover:text-red-400"
                title="删除配置"
                onClick={() => deleteConfig(name)}
              >
                ×
              </button>
            </div>
          ))}
          <div className="my-1 border-t border-zinc-700" />
          <MenuItem onClick={resetDefault}>恢复默认布局</MenuItem>
        </Dropdown>

        <span className="ml-auto text-zinc-500">{PRESET_LABELS[preset]}</span>
      </div>
      <div className="flex-1">
        <DockviewReact components={components} onReady={onReady} />
      </div>
    </div>
  );
}

function Dropdown({
  label,
  open,
  onToggle,
  children,
}: {
  label: string;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="relative">
      <button
        className={`rounded px-2 py-0.5 ${open ? "bg-zinc-700 text-zinc-100" : "text-zinc-400 hover:text-zinc-200"}`}
        onClick={onToggle}
      >
        {label} ▾
      </button>
      {open && (
        <div className="absolute left-0 top-full z-50 mt-0.5 max-h-96 min-w-48 overflow-auto rounded border border-zinc-700 bg-zinc-900 py-1 shadow-xl">
          {children}
        </div>
      )}
    </div>
  );
}

function MenuItem({
  children,
  onClick,
  active,
  checked,
  className,
}: {
  children: React.ReactNode;
  onClick?: () => void;
  active?: boolean;
  checked?: boolean;
  className?: string;
}) {
  return (
    <button
      className={`flex w-full items-center gap-2 px-3 py-1 text-left hover:bg-zinc-800 ${
        active ? "text-blue-300" : "text-zinc-300"
      } ${className ?? ""}`}
      onClick={onClick}
    >
      {checked !== undefined && (
        <span className={`w-4 text-center ${checked ? "text-emerald-400" : "text-zinc-600"}`}>
          {checked ? "✓" : ""}
        </span>
      )}
      <span className="flex-1">{children}</span>
    </button>
  );
}
