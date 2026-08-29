export interface ApiError {
  code: string;
  message: string;
}

export class ApiErrorImpl extends Error {
  code: string;
  status: number;
  constructor(status: number, err: ApiError) {
    super(err.message);
    this.name = "ApiError";
    this.code = err.code;
    this.status = status;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method,
    headers: body !== undefined ? { "Content-Type": "application/json" } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    let err: ApiError = { code: "http_" + res.status, message: res.statusText };
    try {
      const data = await res.json();
      if (data?.error) err = data.error;
    } catch {
      /* ignore */
    }
    throw new ApiErrorImpl(res.status, err);
  }
  return (await res.json()) as T;
}

export const api = {
  health: () => request<unknown>("GET", "/health"),
  probes: () => request<{ probes: ProbeInfo[] }>("GET", "/probes"),
  connect: (body?: Record<string, unknown>) => request<{ target: TargetInfo }>("POST", "/connect", body ?? {}),
  disconnect: () => request<{ disconnected: boolean }>("POST", "/disconnect"),
  status: () => request<Record<string, unknown>>("GET", "/status"),
  run: () => request<{ running: boolean }>("POST", "/debug/run"),
  halt: () => request<{ halted: boolean }>("POST", "/debug/halt"),
  step: () => request<{ stepped: boolean }>("POST", "/debug/step"),
  reset: (mode: "run" | "halt") => request<{ reset: boolean; mode: string }>("POST", "/debug/reset", { mode }),
  registers: () => request<{ registers: RegisterValue[] }>("GET", "/registers"),
  registerWrite: (name: string, value: number) =>
    request<{ written: boolean }>("POST", "/registers", { name, value }),
  memoryRead: (address: number, width: string, count: number) =>
    request<{ address: number; bytes: number[] }>("POST", "/memory/read", { address, width, count }),
  memoryWrite: (address: number, width: string, values: number[]) =>
    request<{ written: boolean }>("POST", "/memory/write", { address, width, values }),
  breakpoints: () => request<{ breakpoints: number[] }>("GET", "/breakpoints"),
  breakpointSet: (address: number, kind?: string) =>
    request<{ set: number }>("POST", "/breakpoints", { address, kind }),
  breakpointClear: () => request<{ cleared: boolean }>("DELETE", "/breakpoints"),
  breakpointsLimits: () =>
    request<{ used?: number; total?: number | null }>("GET", "/breakpoints/limits"),
  watchpoints: () =>
    request<{ watchpoints: { address: number; access: string }[] }>("GET", "/watchpoints"),
  watchpointSet: (address: number, access?: string) =>
    request<{ set: number }>("POST", "/watchpoints", { address, access }),
  fault: () => request<{ fault: RegisterValue[] }>("GET", "/fault"),
  snapshot: () => request<Record<string, unknown>>("POST", "/snapshot"),
  // P4: ELF / symbols / watch / SVD
  elfUpload: (file: File) => {
    const fd = new FormData();
    fd.append("file", file);
    return upload<{ file_id: string; symbols: number; functions: number; variables: number; name?: string }>("/files/elf", fd);
  },
  symbols: (q: { kind?: string; pattern?: string; offset?: number; limit?: number }) => {
    const p = new URLSearchParams();
    if (q.kind) p.set("kind", q.kind);
    if (q.pattern) p.set("pattern", q.pattern);
    p.set("offset", String(q.offset ?? 0));
    p.set("limit", String(q.limit ?? 200));
    return request<{ total: number; items: SymbolItem[] }>("GET", `/symbols?${p}`);
  },
  watchAdd: (target: Record<string, unknown>) =>
    request<{ id: number }>("POST", "/watch", { target }),
  watchList: () => request<{ items: WatchItem[] }>("GET", "/watch"),
  watchDelete: (id: number) => request<{ deleted: boolean }>("DELETE", `/watch/${id}`),
  watchPatch: (id: number, body: { enabled?: boolean; rate_ms?: number }) =>
    request<{ updated: boolean }>("PATCH", `/watch/${id}`, body),
  watchRefresh: () =>
    request<{ items: { id: number; value?: number | null }[] }>("POST", "/watch/refresh"),
  svdUpload: (file: File) => {
    const fd = new FormData();
    fd.append("file", file);
    return upload<{ name?: string; peripherals: number }>("/svd", fd);
  },
  peripherals: () => request<{ peripherals: PeripheralSummary[] }>("GET", "/peripherals"),
  peripheralGet: (name: string) =>
    request<{ peripheral: PeripheralInfo }>("GET", `/peripherals/${name}`),
  peripheralRead: (name: string, register: string) =>
    request<{ address: number; value: number; decoded?: DecodedRegister }>("POST", `/peripherals/${name}/read`, { register }),
  peripheralWrite: (name: string, register: string, value: number) =>
    request<{ written: boolean }>("POST", `/peripherals/${name}/write`, { register, value }),
  peripheralDecode: (name: string, register: string, value: number) =>
    request<{ decoded: DecodedRegister }>("POST", `/peripherals/${name}/decode`, { register, value }),
  firmwareUpload: (file: File, address?: number) => {
    const fd = new FormData();
    fd.append("file", file);
    if (address !== undefined) fd.append("address", String(address));
    return upload<FirmwareUploadResult>("/files/firmware", fd);
  },
  flashErase: (address: number, size: number) =>
    request<{ erased: boolean }>("POST", "/flash/erase", { address, size }),
  flashProgram: (file_id: string, opts: { address?: number; verify?: boolean; mode?: string }) =>
    request<{ programmed: boolean; bytes: number; verify: boolean; mode: string }>("POST", "/flash/program", {
      file_id,
      ...opts,
    }),
  // P5: monitors / RTT / EVR / disassembly
  monitorList: () => request<{ items: MonitorItem[] }>("GET", "/peripherals/monitor"),
  monitorAdd: (p: { peripheral: string; register: string; rate_ms?: number }) =>
    request<{ id: number; safety: string }>("POST", "/peripherals/monitor", p),
  monitorDelete: (id: number) => request<{ deleted: boolean }>("DELETE", `/peripherals/monitor/${id}`),
  rttStart: (address?: number) => request<{ channels: unknown[] }>("POST", "/rtt/start", { address }),
  rttStop: () => request<{ stopped: boolean }>("POST", "/rtt/stop"),
  evrStart: (info_address: number) => request<{ status: unknown }>("POST", "/evr/start", { info_address }),
  evrStop: () => request<{ stopped: boolean }>("POST", "/evr/stop"),
  disassembly: (address: number, count?: number) =>
    request<{ address: number; instructions: DisasmInsn[] }>("GET", `/disassembly?address=${address}&count=${count ?? 16}`),
  addressDescribe: (address: number) =>
    request<{ address: { region?: string | null; symbol?: unknown; peripheral?: string | null } }>("GET", `/address/${address}`),
};

export interface MonitorItem {
  id: number;
  peripheral: string;
  register: string;
  rate_ms: number;
  safety: string;
}

export interface DisasmInsn {
  address: number;
  bytes: string;
  mnemonic: string;
  op_str: string;
  symbol?: string | null;
  pc: number;
}

async function upload<T>(path: string, fd: FormData): Promise<T> {
  const res = await fetch(`/api${path}`, { method: "POST", body: fd });
  if (!res.ok) {
    let err: ApiError = { code: "http_" + res.status, message: res.statusText };
    try {
      const data = await res.json();
      if (data?.error) err = data.error;
    } catch {
      /* ignore */
    }
    throw new ApiErrorImpl(res.status, err);
  }
  return (await res.json()) as T;
}

export interface SymbolItem {
  id: number;
  name: string;
  address: number;
  size: number;
  kind: "function" | "variable" | "other";
  section?: string | null;
  module?: string | null;
}

export interface WatchItem {
  id: number;
  target: { kind: string; symbol_id?: number; address?: number; name?: string };
  format: string;
  rate_ms: number;
  enabled: boolean;
}

export interface PeripheralSummary {
  name: string;
  base: number;
  registers: number;
}

export interface PeripheralInfo {
  name: string;
  base: number;
  registers: {
    name: string;
    offset: number;
    size_bits: number;
    access?: string | null;
    description?: string | null;
    fields: { name: string; offset: number; width: number; description?: string | null; values: { name: string; value: number }[] }[];
  }[];
}

export interface DecodedRegister {
  value: number;
  fields: { name: string; value: number; offset: number; width: number; text?: string | null; description?: string | null }[];
}

export interface FirmwareUploadResult {
  file_id: string;
  format: "bin" | "hex";
  total_size: number;
  address_range?: [number, number] | null;
  segments: { start: number; end: number; size: number }[];
}

export interface ProbeInfo {
  id: string;
  vendor: string;
  product: string;
  serial?: string;
  protocols: string[];
}

export interface TargetInfo {
  core_type: string;
  core_count: number;
  cores: { index: number; core_type: string; name: string }[];
  memory_regions: { name: string; kind: string; start: number; end: number }[];
}

export interface RegisterValue {
  name: string;
  value: number;
}
