// Memory viewer: address / hex bytes / ASCII, 16 bytes per row, u8/u16/u32,
// jump + read + write + refresh.
import { useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { ascii, hex8, parseNumber } from "../debug/formatters";

const ROW = 16;

export function MemoryPanel() {
  const [address, setAddress] = useState("0x20000000");
  const [width, setWidth] = useState<"u8" | "u16" | "u32">("u32");
  const [rows, setRows] = useState<{ address: number; bytes: number[]; ascii: string }[]>([]);
  const [status, setStatus] = useState("");

  const log = useDebugStore((s) => s.log);

  async function read(addr?: number) {
    const base = addr ?? parseNumber(address);
    if (base === null) {
      setStatus("无效地址");
      return;
    }
    try {
      const r = await api.memoryRead(base, width, ROW * 8);
      const out: { address: number; bytes: number[]; ascii: string }[] = [];
      for (let i = 0; i < r.bytes.length; i += ROW) {
        const chunk = r.bytes.slice(i, i + ROW);
        out.push({ address: base + i, bytes: chunk, ascii: ascii(chunk) });
      }
      setRows(out);
      setAddress(`0x${base.toString(16)}`);
      setStatus(`${r.bytes.length} 字节 @ 0x${base.toString(16)}`);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : String(e));
    }
  }

  async function writeCell(rowIndex: number, bytes: number[]) {
    const row = rows[rowIndex];
    if (!row) return;
    try {
      // Rebuild the row as u32 values INCLUDING the edited byte (the old code
      // used the pre-edit row, so single-cell writes never took effect).
      const values: number[] = [];
      for (let i = 0; i < bytes.length; i += 4) {
        values.push(
          (bytes[i] ?? 0) |
            ((bytes[i + 1] ?? 0) << 8) |
            ((bytes[i + 2] ?? 0) << 16) |
            ((bytes[i + 3] ?? 0) << 24)
        );
      }
      await api.memoryWrite(row.address, "u32", values);
      log("info", `已写入 0x${row.address.toString(16)}`);
      await read(row.address);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="flex h-full flex-col text-xs">
      <div className="flex items-center gap-2 border-b border-zinc-700 px-2 py-1">
        <span className="text-zinc-400">地址</span>
        <input
          className="w-32 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none"
          value={address}
          onChange={(e) => setAddress(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && read()}
        />
        <select
          className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-200 outline-none"
          value={width}
          onChange={(e) => setWidth(e.target.value as "u8" | "u16" | "u32")}
        >
          <option value="u8">u8</option>
          <option value="u16">u16</option>
          <option value="u32">u32</option>
        </select>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => read()}>
          读取
        </button>
        <button className="rounded bg-zinc-700 px-2 py-0.5 hover:bg-zinc-600" onClick={() => read(parseNumber(address) ?? undefined)}>
          刷新
        </button>
        <span className="text-zinc-500">{status}</span>
      </div>
      <div className="overflow-auto px-2 py-1 font-mono">
        {rows.map((row, i) => (
          <div key={row.address} className="flex items-center gap-2 whitespace-pre hover:bg-zinc-800/50">
            <span className="w-28 text-zinc-500">{`0x${row.address.toString(16).toUpperCase().padStart(8, "0")}`}</span>
            <span className="flex-1">
              {row.bytes.map((b, j) => (
                <input
                  key={j}
                  className="w-7 bg-transparent text-center text-zinc-200 outline-none focus:bg-zinc-700"
                  defaultValue={hex8(b)}
                  onBlur={(e) => {
                    const v = parseNumber(e.target.value);
                    if (v !== null && v !== b) {
                      const modified = [...row.bytes];
                      modified[j] = v & 0xff;
                      setRows((prev) => prev.map((r, ri) => (ri === i ? { ...r, bytes: modified } : r)));
                      writeCell(i, modified);
                    }
                  }}
                />
              ))}
            </span>
            <span className="w-16 text-zinc-400">{row.ascii}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
