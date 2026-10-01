import type { TunnelStatus } from "./types";

export function statusText(s: TunnelStatus | undefined): string {
  if (!s) return "未知";
  if (s === "Stopped") return "已停止";
  if (s === "Starting") return "启动中";
  if (s === "Stopping") return "停止中";
  if (typeof s === "object" && "Connected" in s) {
    return `已连接 (${s.Connected.count})`;
  }
  if (typeof s === "object" && "Error" in s) {
    return `错误: ${s.Error.slice(0, 40)}`;
  }
  return "未知";
}

export function statusColor(s: TunnelStatus | undefined): string {
  if (!s || s === "Stopped") return "#888";
  if (s === "Starting" || s === "Stopping") return "#f0ad4e";
  if (typeof s === "object" && "Connected" in s) return "#28a745";
  if (typeof s === "object" && "Error" in s) return "#dc3545";
  return "#888";
}

export function isRunning(s: TunnelStatus | undefined): boolean {
  if (!s) return false;
  if (s === "Starting" || s === "Stopping") return true;
  if (typeof s === "object" && "Connected" in s) return true;
  return false;
}
