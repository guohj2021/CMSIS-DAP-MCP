// Peripheral explorer: upload SVD, tree, read/write/decode registers.
import { useEffect, useState } from "react";
import { api, DecodedRegister, MonitorItem, PeripheralInfo, PeripheralSummary } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { onWsEvent } from "../ws/client";

export function PeripheralPanel() {
  const [peripherals, setPeripherals] = useState<PeripheralSummary[]>([]);
  const [selected, setSelected] = useState<PeripheralInfo | null>(null);
  const [regValue, setRegValue] = useState<Record<string, string>>({});
  const [decoded, setDecoded] = useState<DecodedRegister | null>(null);
  const [monitors, setMonitors] = useState<MonitorItem[]>([]);
  const [monitorValues, setMonitorValues] = useState<Record<number, number | null>>({});
  const log = useDebugStore((s) => s.log);

  useEffect(() => {
    api.monitorList().then((r) => setMonitors(r.items)).catch(() => {});
    const off = onWsEvent("peripheral_value_changed", (data) => {
      const d = data as { items: { id: number; value?: number | null }[] };
      const next: Record<number, number | null> = {};
      for (const it of d.items) next[it.id] = it.value ?? null;
      setMonitorValues((m) => ({ ...m, ...next }));
    });
    return off;
  }, []);

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
      setDecoded(null);
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
    }
  }

  async function read(reg: string) {
    if (!selected) return;
    try {
      const r = await api.peripheralRead(selected.name, reg);
      setRegValue((m) => ({ ...m, [reg]: `0x${r.value.toString(16).toUpperCase().padStart(8, "0")}` }));
      if (r.decoded) setDecoded(r.decoded);
    } catch (e) {
      log("error", e instanceof Error ? e.message : String(e));
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

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <input
          type="file"
          accept=".svd"
          className="w-40 text-[10px] text-zinc-400 file:mr-2 file:rounded file:border-0 file:bg-zinc-700 file:px-2 file:py-0.5 file:text-zinc-200"
          onChange={(e) => e.target.files?.[0] && upload(e.target.files[0])}
        />
        <span className="text-zinc-500">{peripherals.length} 个外设</span>
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
          {selected?.registers.map((r) => (
            <div key={r.name} className="mb-1 rounded bg-zinc-900 p-1">
              <div className="flex items-center gap-2">
                <span className="font-mono text-amber-300">{r.name}</span>
                <span className="text-zinc-600">0x{(selected.base + r.offset).toString(16)}</span>
                <span className="text-zinc-600">{r.access ?? ""}</span>
                <input
                  className="ml-auto w-28 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
                  placeholder="写入值(hex)"
                  value={regValue[r.name] ?? ""}
                  onChange={(e) => setRegValue((m) => ({ ...m, [r.name]: e.target.value }))}
                />
                <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => read(r.name)}>
                  读
                </button>
                <button
                  className="rounded bg-emerald-900 px-2 py-0.5 text-emerald-200 hover:bg-emerald-800"
                  onClick={async () => {
                    try {
                      const pname = selected?.name ?? "";
                      const m = await api.monitorAdd({ peripheral: pname, register: r.name, rate_ms: 500 });
                      setMonitors((prev) => [...prev, { id: m.id, peripheral: pname, register: r.name, rate_ms: 500, safety: m.safety }]);
                      log("info", `已监控 ${pname}.${r.name}`);
                    } catch (e) {
                      log("error", e instanceof Error ? e.message : String(e));
                    }
                  }}

                >
                  监控
                </button>
                <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => write(r.name, regValue[r.name] ?? "")}>
                  写
                </button>
              </div>
              {r.description && <div className="mt-0.5 text-zinc-500">{r.description}</div>}
              {decoded && (
                <div className="mt-1 border-t border-zinc-800 pt-1">
                  {decoded.fields.map((f) => (
                    <div key={f.name} className="flex gap-2">
                      <span className="w-28 text-zinc-400">{f.name}[{f.offset}:{f.offset + f.width - 1}]</span>
                      <span className="text-zinc-200">0x{f.value.toString(16)}</span>
                      {f.text && <span className="text-emerald-400">{f.text}</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
          {selected && selected.registers.length === 0 && (
            <div className="text-zinc-500">无寄存器</div>
          )}
          {monitors.length > 0 && (
            <div className="mt-2 border-t border-zinc-800 pt-1">
              <div className="mb-1 text-zinc-500">监控中（周期刷新）</div>
              {monitors.map((m) => (
                <div key={m.id} className="flex items-center gap-2">
                  <span className="text-zinc-300">{m.peripheral}.{m.register}</span>
                  <span className="font-mono text-emerald-300">
                    {(() => {
                      const v = monitorValues[m.id];
                      return v !== undefined && v !== null ? `0x${v.toString(16)}` : "—";
                    })()}
                  </span>
                  <button
                    className="rounded bg-red-900 px-2 py-0.5 text-red-200 hover:bg-red-800"
                    onClick={async () => {
                      await api.monitorDelete(m.id);
                      setMonitors((prev) => prev.filter((x) => x.id !== m.id));
                    }}
                  >
                    停止
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
