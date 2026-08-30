// Command registry: single source for the Command Palette (P7a).
import * as ops from "./operations";
import { useDebugStore } from "../store/debugStore";

export interface CommandDef {
  id: string;
  label: string;
  keywords: string;
  run: () => void;
  enabled: () => boolean;
}

export const commands: CommandDef[] = [
  { id: "connect", label: "连接探针", keywords: "connect probe", run: () => void ops.connect(), enabled: () => !useDebugStore.getState().connected },
  { id: "disconnect", label: "断开连接", keywords: "disconnect", run: () => void ops.disconnect(), enabled: () => useDebugStore.getState().connected },
  { id: "run", label: "运行 (Run)", keywords: "run go continue", run: () => void ops.run(), enabled: () => useDebugStore.getState().status.target !== "running" && useDebugStore.getState().connected },
  { id: "halt", label: "暂停 (Halt)", keywords: "halt stop break", run: () => void ops.halt(), enabled: () => useDebugStore.getState().status.target === "running" },
  { id: "step", label: "单步 (Step)", keywords: "step", run: () => void ops.step(), enabled: () => useDebugStore.getState().status.target === "halted" },
  { id: "reset", label: "复位 (Reset)", keywords: "reset", run: () => void ops.reset("run"), enabled: () => useDebugStore.getState().connected },
  { id: "reset_halt", label: "复位并暂停", keywords: "reset halt", run: () => void ops.reset("halt"), enabled: () => useDebugStore.getState().connected },
  { id: "refresh_regs", label: "刷新寄存器", keywords: "registers refresh", run: () => void ops.refreshRegisters(), enabled: () => useDebugStore.getState().connected },
  { id: "refresh_fault", label: "检查 Fault", keywords: "fault cfsr", run: () => void ops.refreshFault(), enabled: () => useDebugStore.getState().connected },
  { id: "refresh_bp", label: "刷新断点", keywords: "breakpoint", run: () => void ops.refreshBreakpoints(), enabled: () => useDebugStore.getState().connected },
  { id: "goto", label: "Go to 地址 (Ctrl+L)", keywords: "goto address disasm", run: () => { document.getElementById("goto-addr")?.focus(); }, enabled: () => true },
  { id: "flash", label: "打开 Flash 工作区", keywords: "flash program firmware", run: () => { window.dispatchEvent(new CustomEvent("open-workspace", { detail: { workspace: "flash" } })); }, enabled: () => true },
  { id: "debug", label: "打开 Debug 工作区", keywords: "debug workspace", run: () => { window.dispatchEvent(new CustomEvent("open-workspace", { detail: { workspace: "debug" } })); }, enabled: () => true },
];

export function runCommand(id: string) {
  const c = commands.find((x) => x.id === id);
  if (c && c.enabled()) c.run();
}
