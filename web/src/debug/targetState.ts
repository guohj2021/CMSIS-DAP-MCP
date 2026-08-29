// Target state helpers (frozen v5 §3).
import { useDebugStore, TargetState } from "../store/debugStore";

export function useTargetState(): TargetState {
  return useDebugStore((s) => s.status.target);
}

export function useServerState() {
  return useDebugStore((s) => s.status.server);
}

export function isHalted(): boolean {
  return useDebugStore.getState().status.target === "halted";
}

export function isFault(): boolean {
  return useDebugStore.getState().status.target === "fault";
}
