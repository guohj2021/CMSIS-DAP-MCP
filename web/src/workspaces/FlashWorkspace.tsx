// Flash workspace: drop BIN/HEX, analyze, erase/program/verify, progress.
import { useEffect, useState } from "react";
import { api, FirmwareUploadResult } from "../api/client";
import { useDebugStore } from "../store/debugStore";
import { onWsEvent } from "../ws/client";

export function FlashWorkspace() {
  const connected = useDebugStore((s) => s.connected);
  const log = useDebugStore((s) => s.log);
  const [file, setFile] = useState<FirmwareUploadResult | null>(null);
  const [binAddress, setBinAddress] = useState("0x08000000");
  const [mode, setMode] = useState("reset_run");
  const [verify, setVerify] = useState(true);
  const [progress, setProgress] = useState<{ phase: string; done: number; total: number } | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);

  useEffect(() => {
    const off = onWsEvent("flash_progress", (data) => {
      const d = data as { phase: string; bytes_done: number; bytes_total: number };
      setProgress({ phase: d.phase, done: d.bytes_done, total: d.bytes_total });
    });
    const offDone = onWsEvent("flash_complete", () => {
      setBusy(false);
      setProgress(null);
    });
    return () => {
      off();
      offDone();
    };
  }, []);

  async function upload(f: File) {
    try {
      const address = f.name.toLowerCase().endsWith(".bin") ? parseInt(binAddress, 16) : undefined;
      const r = await api.firmwareUpload(f, Number.isNaN(address as number) ? undefined : address);
      setFile(r);
      setConfirm(false);
      log("info", `固件已分析: ${r.format}, ${r.total_size} 字节, 范围 ${r.address_range?.map((a) => "0x" + a.toString(16)).join("..")}`);
    } catch (e) {
      log("error", `上传失败: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function erase() {
    if (!file || !file.address_range) return;
    try {
      setBusy(true);
      await api.flashErase(file.address_range[0], file.address_range[1] - file.address_range[0]);
      log("info", "擦除完成");
    } catch (e) {
      log("error", `擦除失败: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  async function program() {
    if (!file) return;
    if (!confirm) {
      setConfirm(true);
      return;
    }
    try {
      setBusy(true);
      const r = await api.flashProgram(file.file_id, { verify, mode });
      log("info", `烧录完成: ${r.bytes} 字节, verify=${r.verify}, mode=${r.mode}`);
      setConfirm(false);
    } catch (e) {
      log("error", `烧录失败: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  const pct = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;

  return (
    <div className="flex h-full flex-col overflow-auto p-4 text-xs">
      <h2 className="mb-2 text-sm font-semibold text-zinc-200">固件编程（Firmware Programming）</h2>

      <div className="rounded border border-dashed border-zinc-700 p-4 text-center text-zinc-500" onDragOver={(e) => e.preventDefault()} onDrop={(e) => {
        e.preventDefault();
        if (e.dataTransfer.files[0]) upload(e.dataTransfer.files[0]);
      }}>
        拖入 BIN / HEX 文件，或
        <input type="file" accept=".bin,.hex" className="ml-2 text-[10px] text-zinc-400 file:rounded file:border-0 file:bg-zinc-700 file:px-2 file:py-0.5 file:text-zinc-200" onChange={(e) => e.target.files?.[0] && upload(e.target.files[0])} />
      </div>

      {file && (
        <div className="mt-3 grid grid-cols-2 gap-2">
          <div className="text-zinc-400">文件类型: <span className="text-zinc-200">{file.format.toUpperCase()}</span></div>
          <div className="text-zinc-400">大小: <span className="text-zinc-200">{file.total_size} 字节</span></div>
          <div className="text-zinc-400">范围: <span className="font-mono text-zinc-200">{file.address_range ? file.address_range.map((a) => "0x" + a.toString(16)).join(" .. ") : "—"}</span></div>
          <div className="text-zinc-400">段数: <span className="text-zinc-200">{file.segments.length}</span></div>
        </div>
      )}

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {file?.format === "bin" && (
          <>
            <span className="text-zinc-400">BIN 地址</span>
            <input className="w-28 rounded bg-zinc-800 px-2 py-0.5 font-mono text-zinc-200 outline-none" value={binAddress} onChange={(e) => setBinAddress(e.target.value)} />
          </>
        )}
        <select className="rounded bg-zinc-800 px-1 py-0.5 text-zinc-200 outline-none" value={mode} onChange={(e) => setMode(e.target.value)}>
          <option value="stay_halted">保持暂停</option>
          <option value="reset">复位</option>
          <option value="reset_run">复位并运行</option>
        </select>
        <label className="flex items-center gap-1 text-zinc-400">
          <input type="checkbox" checked={verify} onChange={(e) => setVerify(e.target.checked)} /> 校验
        </label>
        <button className="rounded bg-zinc-700 px-3 py-1 text-zinc-200 hover:bg-zinc-600 disabled:opacity-40" onClick={erase} disabled={!file || busy || !connected}>
          擦除
        </button>
        <button className={`rounded px-3 py-1 text-white disabled:opacity-40 ${confirm ? "bg-red-700 hover:bg-red-600" : "bg-blue-600 hover:bg-blue-500"}`} onClick={program} disabled={!file || busy || !connected}>
          {confirm ? "再次点击确认烧录" : busy ? "烧录中…" : "烧录"}
        </button>
      </div>

      {progress && (
        <div className="mt-3">
          <div className="mb-1 flex justify-between text-zinc-400">
            <span>{progress.phase}</span>
            <span>{progress.done} / {progress.total} 字节 ({pct}%)</span>
          </div>
          <div className="h-2 rounded bg-zinc-800">
            <div className="h-2 rounded bg-blue-500 transition-all" style={{ width: `${pct}%` }} />
          </div>
        </div>
      )}

      {!connected && <div className="mt-3 text-red-400">未连接目标，请先连接。</div>}
    </div>
  );
}
