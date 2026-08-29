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
  fault: () => request<{ fault: RegisterValue[] }>("GET", "/fault"),
  snapshot: () => request<Record<string, unknown>>("POST", "/snapshot"),
};

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
