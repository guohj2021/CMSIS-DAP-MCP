// Value formatters (HEX / DEC / BIN) shared across panels.

export type NumFormat = "hex" | "dec" | "bin";

export function formatValue(value: number, fmt: NumFormat): string {
  switch (fmt) {
    case "hex":
      return `0x${value.toString(16).toUpperCase().padStart(8, "0")}`;
    case "bin":
      return `0b${value.toString(2).padStart(32, "0")}`;
    default:
      return String(value);
  }
}

export function hex8(value: number): string {
  return value.toString(16).toUpperCase().padStart(2, "0");
}

export function ascii(bytes: number[]): string {
  let out = "";
  for (const b of bytes) {
    out += b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : ".";
  }
  return out;
}

export function parseNumber(input: string): number | null {
  const t = input.trim();
  if (/^0x/i.test(t)) return parseInt(t.slice(2), 16);
  if (/^0b/i.test(t)) return parseInt(t.slice(2), 2);
  const n = parseInt(t, 10);
  return Number.isNaN(n) ? null : n;
}
