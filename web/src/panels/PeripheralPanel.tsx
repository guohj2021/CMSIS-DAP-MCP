// Peripheral explorer: upload SVD, tree, read/write/decode registers.
//
// Design (v5 user feedback): no extra monitor items/variables - either
// refresh ALL registers of the selected peripheral in place (全部刷新), or
// toggle periodic monitoring per register with the value shown in the SAME
// register row (监控 button toggles on/off). There is no separate "监控中"
// block below.
import { useEffect, useState } from "react";
import { api, DecodedRegister, PeripheralInfo, PeripheralSummary } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { onWsEvent } from "../ws/client";

const RATE_OPTIONS = [100, 200, 500, 1000, 2000, 5000];

export function PeripheralPanel() {
  const [peripherals, setPeripherals] = useState<PeripheralSummary[]>([]);
  const [selected, setSelected] = useState<PeripheralInfo | null>(null);
  const [regValue, setRegValue] = useState<Record<string, string>>({});
  const [decoded, setDecoded] = useState<Record<string, DecodedRegister>>({});
  const [readValues, setReadValues] = useState<Record<string, number>>({});
  const [rateMs, setRateMs] = useState(500);
  // monitored: "peripheral.register" -> monitor id (button state)
  const [monitored, setMonitored] = useState<Record<string, number>>({});
  // monitorRegs: monitor id -> "peripheral.register" (WS value routing)
  const [monitorRegs, setMonitorRegs] = useState<Record<number, string>>({});
  const [refreshing, setRefreshing] = useState(false);
  const log = useDebugStore((s) => s.log);

  useEffect(() => {
    api.monitorList().then((r) => {
      const m: Record<string, number> = {};
      const mr: Record<number, string> = {};
      for (const it of r.items) {
        const key = `${it.peripheral}.${it.register}`;
        m[key] = it.id;
        mr[it.id] = key;
      }
      setMonitored(m);
      setMonitorRegs(mr);
    }).catch(() => {});
    const off = onWsEvent("peripheral_value_changed", (data) => {
      const d = data as { items: { id: number; value?: number | null }[] };
      const next: Record<string, number> = {};
      for (const it of d.items) {
        const key = monitorRegs[it.id];
        if (key && it.value !== null && it.value !== undefined) {
          const reg = key.split(".").slice(1).join(".");
          next[reg] = it.value;
        }
      }
      if (Object.keys(next).length) {
        setReadValues((m) => ({ ...m, ...next }));
      }
    });
    return off;
  }, [monitorRegs]);

  async function upload(file: File) {
    try {
      const r = await api.svdUpload(file);
      log("info", `已加载 SVD ${r.name ?? file.name}: ${r.peripherals} 个外设`);
      const list = await api.peripherals();
      setPeripherals(list.peripherals);
    } catch (e) {
      log("error", `SVD 加载失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function open(name: string) {
    try {
      const r = await api.peripheralGet(name);
      setSelected(r.peripheral);
      setDecoded({});
      setReadValues({});
      setRegValue({});
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function read(reg: string) {
    if (!selected) return;
    try {
      const r = await api.peripheralRead(selected.name, reg);
      setReadValues((m) => ({ ...m, [reg]: r.value }));
      const dv = r.decoded;
      if (dv) setDecoded((m) => ({ ...m, [reg]: dv }));
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  // Read every readable register of the selected peripheral in place.
  async function refreshAll() {
    if (!selected || refreshing) return;
    setRefreshing(true);
    let ok = 0;
    try {
      for (const r of selected.registers) {
        if (r.access === "write-only") continue;
        await read(r.name);
        ok += 1;
      }
      log("info", `已刷新 ${selected.name} 的 ${ok} 个寄存器`);
    } finally {
      setRefreshing(false);
    }
  }

  async function write(reg: string, raw: string) {
    if (!selected) return;
    const v = parseInt(raw, 16);
    if (Number.isNaN(v)) {
      log("error", `无效值 ${raw}`);
      return;
    }
    try {
      await api.peripheralWrite(selected.name, reg, v);
      log("info", `已写入 ${selected.name}.${reg} = 0x${v.toString(16)}`);
      await read(reg);
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  // Toggle periodic in-place refresh for one register (no extra monitor block).
  async function toggleMonitor(reg: string) {
    if (!selected) return;
    const key = `${selected.name}.${reg}`;
    const id = monitored[key];
    try {
      if (id !== undefined) {
        await api.monitorDelete(id);
        const next = { ...monitored };
        delete next[key];
        setMonitored(next);
        const mr = { ...monitorRegs };
        delete mr[id];
        setMonitorRegs(mr);
        log("info", `已停止监控 ${key}`);
      } else {
        const m = await api.monitorAdd({ peripheral: selected.name, register: reg, rate_ms: rateMs });
        setMonitored((prev) => ({ ...prev, [key]: m.id }));
        setMonitorRegs((prev) => ({ ...prev, [m.id]: key }));
        log("info", `已开始监控 ${key}（${rateMs}ms，值就地刷新）`);
        await read(reg);
      }
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  // Changing the header rate also applies to active monitors of this peripheral.
  async function changeRate(v: number) {
    setRateMs(v);
    if (!selected) return;
    const ids = Object.entries(monitored)
      .filter(([k]) => k.startsWith(selected.name + "."))
      .map(([, id]) => id);
    await Promise.all(
      ids.map((id) => api.monitorPatch(id, { rate_ms: v }).catch(() => {}))
    );
    if (ids.length) log("info", `监控刷新周期已改为 ${v}ms`);
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex flex-wrap items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <input
          type="file"
          accept=".svd"
          className="w-40 text-[10px] text-zinc-400 file:mr-2 file:rounded file:border-0 file:bg-zinc-700 file:px-2 file:py-0.5 file:text-zinc-200"
          onChange={(e) => e.target.files?.[0] && upload(e.target.files[0])}
        />
        <span className="text-zinc-500">{peripherals.length} 个外设</span>
        <button
          className="rounded bg-blue-600 px-2 py-0.5 text-white hover:bg-blue-500 disabled:opacity-40"
          onClick={refreshAll}
          disabled={!selected || refreshing}
          title="读取选中外设的所有寄存器（跳过只写寄存器）"
        >
          {refreshing ? "刷新中…" : "全部刷新"}
        </button>
        <span className="ml-auto text-zinc-500">刷新周期</span>
        <select
          className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-400 outline-none"
          value={rateMs}
          onChange={(e) => changeRate(Number(e.target.value))}
          title="监控刷新周期"
        >
          {RATE_OPTIONS.map((r) => (
            <option key={r} value={r}>
              {r >= 1000 ? `${r / 1000}s` : `${r}ms`}
            </option>
          ))}
        </select>
      </div>
      <div className="flex flex-1 overflow-hidden">
        <div className="w-40 overflow-auto border-r border-zinc-800 py-1">
          {peripherals.map((p) => (
            <button
              key={p.name}
              className={`block w-full px-2 py-0.5 text-left hover:bg-zinc-800 ${selected?.name === p.name ? "bg-zinc-800 text-blue-300" : "text-zinc-300"}`}
              onClick={() => open(p.name)}
            >
              {p.name}
            </button>
          ))}
        </div>
        <div className="flex-1 overflow-auto px-2 py-1">
          {selected?.registers.map((r) => {
            const dv = decoded[r.name];
            const rv = readValues[r.name];
            const isMonitored = monitored[`${selected.name}.${r.name}`] !== undefined;
            return (
              <div key={r.name} className="mb-1 rounded bg-zinc-900 p-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-amber-300">{r.name}</span>
                  <span className="text-zinc-600">0x{(selected.base + r.offset).toString(16)}</span>
                  <span className="text-zinc-600">{r.access ?? ""}</span>
                  {rv !== undefined && (
                    <span className="font-mono text-emerald-300">
                      值=0x{rv.toString(16).toUpperCase().padStart(8, "0")}
                      {isMonitored && <span className="ml-1 text-emerald-500">●</span>}
                    </span>
                  )}
                  <input
                    className="ml-auto w-24 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
                    placeholder="写入值(hex)"
                    value={regValue[r.name] ?? ""}
                    onChange={(e) => setRegValue((m) => ({ ...m, [r.name]: e.target.value }))}
                  />
                  <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => read(r.name)}>
                    读
                  </button>
                  <button
                    className={`rounded px-2 py-0.5 ${isMonitored ? "bg-red-900 text-red-200 hover:bg-red-800" : "bg-emerald-900 text-emerald-200 hover:bg-emerald-800"}`}
                    onClick={() => toggleMonitor(r.name)}
                    title="周期刷新该寄存器（值就地显示）"
                  >
                    {isMonitored ? "停止" : "监控"}
                  </button>
                  <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => write(r.name, regValue[r.name] ?? "")}>
                    写
                  </button>
                </div>
                {r.description && <div className="mt-0.5 text-zinc-500">{r.description}</div>}
                {dv && (
                  <div className="mt-1 border-t border-zinc-800 pt-1">
                    {dv.fields.map((f) => (
                      <div key={f.name} className="flex gap-2">
                        <span className="w-28 text-zinc-400">{f.name}[{f.offset}:{f.offset + f.width - 1}]</span>
                        <span className="text-zinc-200">0x{f.value.toString(16)}</span>
                        {f.text && <span className="text-emerald-400">{f.text}</span>}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
          {selected && selected.registers.length === 0 && (
            <div className="text-zinc-500">无寄存器</div>
          )}
          {!selected && peripherals.length === 0 && (
            <div className="rounded border border-dashed border-zinc-700 p-3 text-center text-zinc-500">
              上传 SVD 或选择外设；可"全部刷新"或对单个寄存器点"监控"（值就地刷新）
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
