import { invoke } from "@tauri-apps/api/core";
import type {
  AppStateSnapshot,
  CertInfo,
  LogLine,
  MetricsSnapshot,
  NewTunnelProfile,
  PidEntry,
  RemoteTunnel,
  TunnelPatch,
  TunnelProfile,
  TunnelStatus,
  VersionInfo,
} from "./types";

export const api = {
  getState: () => invoke<AppStateSnapshot>("get_state"),
  setCloudflaredExe: (path: string) =>
    invoke<VersionInfo>("set_cloudflared_exe", { path }),
  testCloudflaredVersion: () =>
    invoke<VersionInfo>("test_cloudflared_version"),
  checkLogin: () => invoke<CertInfo>("check_login"),
  startLogin: () => invoke<boolean>("start_login"),
  listTunnelsRemote: () => invoke<RemoteTunnel[]>("list_tunnels_remote"),
  createTunnel: (profile: NewTunnelProfile) =>
    invoke<TunnelProfile>("create_tunnel", { profile }),
  editTunnel: (id: string, patch: TunnelPatch) =>
    invoke<TunnelProfile>("edit_tunnel", { id, patch }),
  deleteTunnel: (id: string) => invoke<void>("delete_tunnel", { id }),
  routeDns: (id: string) => invoke<void>("route_dns", { id }),
  startTunnel: (id: string) => invoke<void>("start_tunnel", { id }),
  stopTunnel: (id: string) => invoke<void>("stop_tunnel", { id }),
  restartTunnel: (id: string) => invoke<void>("restart_tunnel", { id }),
  getTunnelStatus: (id: string) =>
    invoke<TunnelStatus>("get_tunnel_status", { id }),
  getTunnelLogs: (id: string) =>
    invoke<LogLine[]>("get_tunnel_logs", { id }),
  getTunnelMetrics: (id: string) =>
    invoke<MetricsSnapshot | null>("get_tunnel_metrics", { id }),
  openCloudflaredDir: () => invoke<void>("open_cloudflared_dir"),
  openConfigFile: (id: string) => invoke<void>("open_config_file", { id }),
  cleanupOrphans: () => invoke<PidEntry[]>("cleanup_orphans"),
  quitApp: () => invoke<void>("quit_app"),
};
