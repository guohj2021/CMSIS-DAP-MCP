// Memory viewer: address / hex values (u8/u16/u32) / ASCII, 16 bytes per row.
// A fixed byte range is read; the width selector only changes how the bytes
// are grouped and displayed (u16 shows 16-bit cells, u32 shows 32-bit cells),
// so switching width re-groups instantly without re-reading.
import { useState } from "react";
import { api } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { ascii, parseNumber } from "../debug/formatters";

type MemWidth = "u8" | "u16" | "u32";

const ROW_BYTES = 16;

const WIDTH_SIZE: Record<MemWidth, number> = { u8: 1, u16: 2, u32: 4 };
const WIDTH_CLASS: Record<MemWidth, string> = {
  u8: "w-7",
  u16: "w-12",
  u32: "w-20",
};

function widthHex(v: number, width: MemWidth): string {
  const digits = WIDTH_SIZE[width] * 2;
  return v.toString(16).toUpperCase().padStart(digits, "0");
}

// Group a row of bytes into little-endian values of the given width.
function groupRow(bytes: number[], width: MemWidth): number[] {
  const w = WIDTH_SIZE[width];
  const out: number[] = [];
  for (let i = 0; i < bytes.length; i += w) {
    let v = 0;
    for (let j = 0; j < w; j++) v |= (bytes[i + j] ?? 0) << (8 * j);
    out.push(v >>> 0); // keep unsigned so 32-bit values display correctly
  }
  return out;
}

export function MemoryPanel() {
  const [address, setAddress] = useState("0x20000000");
  const [width, setWidth] = useState<MemWidth>("u32");
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
      // Always fetch a fixed byte range; width only affects display grouping.
      const r = await api.memoryRead(base, "u8", ROW_BYTES * 8);
      const out: { address: number; bytes: number[]; ascii: string }[] = [];
      for (let i = 0; i < r.bytes.length; i += ROW_BYTES) {
        const chunk = r.bytes.slice(i, i + ROW_BYTES);
        out.push({ address: base + i, bytes: chunk, ascii: ascii(chunk) });
      }
      setRows(out);
      setAddress(`0x${base.toString(16)}`);
      setStatus(`${r.bytes.length} 字节 @ 0x${base.toString(16)}（${width}）`);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : String(e));
    }
  }

  async function writeCell(rowIndex: number, cellIndex: number, newValue: number) {
    const row = rows[rowIndex];
    if (!row) return;
    const w = WIDTH_SIZE[width];
    const bytes = [...row.bytes];
    for (let j = 0; j < w; j++) bytes[cellIndex * w + j] = (newValue >>> (8 * j)) & 0xff;
    try {
      const values: number[] = [];
      for (let i = 0; i < bytes.length; i += w) {
        let v = 0;
        for (let j = 0; j < w; j++) v |= (bytes[i + j] ?? 0) << (8 * j);
        values.push(v >>> 0);
      }
      await api.memoryWrite(row.address, width, values);
      log("info", `已写入 0x${row.address.toString(16)}（${width}）`);
      setRows((prev) => prev.map((r, ri) => (ri === rowIndex ? { ...r, bytes } : r)));
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
          onChange={(e) => setWidth(e.target.value as MemWidth)}
          title="数据宽度（影响显示分组与写入）"
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
        {rows.map((row, i) => {
          const values = groupRow(row.bytes, width);
          return (
            <div key={row.address} className="flex items-center gap-2 whitespace-pre hover:bg-zinc-800/50">
              <span className="w-28 text-zinc-500">{`0x${row.address.toString(16).toUpperCase().padStart(8, "0")}`}</span>
              <span className="flex-1">
                {values.map((v, j) => (
                  <input
                    key={`${width}-${j}`}
                    className={`${WIDTH_CLASS[width]} bg-transparent text-center text-zinc-200 outline-none focus:bg-zinc-700`}
                    defaultValue={widthHex(v, width)}
                    onBlur={(e) => {
                      const nv = parseNumber(e.target.value);
                      if (nv !== null && nv !== v) writeCell(i, j, nv);
                    }}
                  />
                ))}
              </span>
              <span className="w-16 text-zinc-400">{row.ascii}</span>
            </div>
          );
        })}
        {rows.length === 0 && <div className="mt-2 text-zinc-600">输入地址后点击读取</div>}
      </div>
    </div>
  );
}
