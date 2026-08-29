// Debug actions: the single entry point between panels and the API/WS layer
// (frozen v5 §10: Panel -> Debug Action -> Store -> API/WS).

import { api, RegisterValue } from "../api/client";
import { useDebugStore } from "../store/debugStore";

function fmtErr(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export async function refreshProbes() {
  try {
    const r = await api.probes();
    useDebugStore.getState().setProbes(r.probes);
    return r.probes;
  } catch (e) {
    useDebugStore.getState().log("error", `枚举探针失败: ${fmtErr(e)}`);
    return [];
  }
}

export async function connect(probeId?: string) {
  const st = useDebugStore.getState();
  st.setStatus({ server: "busy" });
  try {
    const r = await api.connect(probeId ? { probe_id: probeId } : {});
    useDebugStore.getState().setTarget(r.target);
    useDebugStore.getState().setStatus({ server: "ready", target: "unknown" });
    useDebugStore.getState().log("info", `已连接: ${r.target.core_type} (${r.target.cores.length} core)`);
    return r.target;
  } catch (e) {
    useDebugStore.getState().setStatus({ server: "disconnected", target: "unknown" });
    useDebugStore.getState().log("error", `连接失败: ${fmtErr(e)}`);
    throw e;
  }
}

export async function disconnect() {
  try {
    await api.disconnect();
    useDebugStore.getState().setStatus({ server: "disconnected", target: "unknown" });
    useDebugStore.getState().setTarget(undefined);
    useDebugStore.getState().setRegisters([]);
    useDebugStore.getState().log("info", "已断开连接");
  } catch (e) {
    useDebugStore.getState().log("error", `断开失败: ${fmtErr(e)}`);
  }
}

export async function run() {
  try {
    await api.run();
    useDebugStore.getState().setStatus({ target: "running" });
  } catch (e) {
    useDebugStore.getState().log("error", `运行失败: ${fmtErr(e)}`);
  }
}

export async function halt() {
  try {
    await api.halt();
    useDebugStore.getState().setStatus({ target: "halted" });
    await refreshRegisters();
  } catch (e) {
    useDebugStore.getState().log("error", `暂停失败: ${fmtErr(e)}`);
  }
}

export async function step() {
  try {
    await api.step();
    await refreshRegisters();
  } catch (e) {
    useDebugStore.getState().log("error", `单步失败: ${fmtErr(e)}`);
  }
}

export async function reset(mode: "run" | "halt") {
  try {
    await api.reset(mode);
    useDebugStore.getState().setStatus({ target: mode === "run" ? "running" : "halted" });
    useDebugStore.getState().log("info", `已复位 (${mode})`);
  } catch (e) {
    useDebugStore.getState().log("error", `复位失败: ${fmtErr(e)}`);
  }
}

export async function refreshRegisters(): Promise<RegisterValue[]> {
  try {
    const r = await api.registers();
    useDebugStore.getState().setRegisters(r.registers);
    return r.registers;
  } catch (e) {
    useDebugStore.getState().log("debug", `寄存器读取失败: ${fmtErr(e)}`);
    return [];
  }
}

export async function refreshFault() {
  try {
    const r = await api.fault();
    const nonZero = r.fault.filter((f) => f.value !== 0);
    if (nonZero.length > 0) {
      useDebugStore.getState().setStatus({ target: "fault" });
      useDebugStore.getState().log(
        "error",
        `Fault: ${nonZero.map((f) => `${f.name}=0x${f.value.toString(16)}`).join(", ")}`
      );
    }
    return r.fault;
  } catch {
    return [];
  }
}

export async function refreshBreakpoints() {
  try {
    const r = await api.breakpoints();
    useDebugStore.getState().setBreakpoints(r.breakpoints);
  } catch {
    /* ignore */
  }
}

export async function snapshot() {
  try {
    const r = await api.snapshot() as {
      state?: string;
      pc?: number | null;
      registers?: { name: string; value: number }[];
      fault?: { name: string; value: number }[];
    };
    const regs = new Map((r.registers ?? []).map((x) => [x.name, x.value]));
    const get = (n: string) => {
      const v = regs.get(n);
      return v !== undefined ? `0x${v.toString(16)}` : "?";
    };
    const fault = (r.fault ?? []).filter((f) => f.value !== 0)
      .map((f) => `${f.name}=0x${f.value.toString(16)}`)
      .join(", ");
    useDebugStore.getState().log(
      "info",
      `快照: state=${r.state ?? "?"} pc=${get("pc")} sp=${get("sp")} lr=${get("lr")} xpsr=${get("xpsr")}${fault ? ` fault=[${fault}]` : ""}`
    );
  } catch (e) {
    useDebugStore.getState().log("error", `快照失败: ${e instanceof Error ? e.message : String(e)}`);
  }
}

export function logError(message: string) {
  useDebugStore.getState().log("error", message);
}

export function isRunning(): boolean {
  return useDebugStore.getState().status.target === "running";
}

