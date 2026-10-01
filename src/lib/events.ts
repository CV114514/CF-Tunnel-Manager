import { listen } from "@tauri-apps/api/event";
import type { LogLine, PidEntry, TunnelStatus } from "./types";

export function onTunnelLog(
  tunnelId: string,
  cb: (line: LogLine) => void
): Promise<() => void> {
  return listen<LogLine>(`tunnel:${tunnelId}:log`, (e) => cb(e.payload));
}

export function onTunnelStatus(
  tunnelId: string,
  cb: (status: TunnelStatus) => void
): Promise<() => void> {
  return listen<TunnelStatus>(`tunnel:${tunnelId}:status`, (e) =>
    cb(e.payload)
  );
}

export function onGlobalStateChanged(cb: () => void): Promise<() => void> {
  return listen("global:state_changed", () => cb());
}

export function onLoginCertChanged(cb: () => void): Promise<() => void> {
  return listen("login:cert_changed", () => cb());
}

export function onWindowHidden(cb: () => void): Promise<() => void> {
  return listen("window:hidden", () => cb());
}

export function onOrphansDetected(
  cb: (orphans: PidEntry[]) => void
): Promise<() => void> {
  return listen<PidEntry[]>("orphans:detected", (e) => cb(e.payload));
}
