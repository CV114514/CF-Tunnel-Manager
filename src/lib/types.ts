export type TunnelStatus =
  | "Stopped"
  | "Starting"
  | { Connected: { count: number; edge_locations: string[] } }
  | { Error: string }
  | "Stopping";

export interface Settings {
  cloudflared_exe_path: string | null;
  metrics_poll_interval_secs: number;
  always_cleanup_orphans: boolean;
  theme: string;
  cloudflared_home_override: string | null;
}

export interface TunnelProfile {
  id: string;
  name: string;
  hostname: string;
  target_service_url: string;
  config_yaml_path: string;
  credentials_json_path: string | null;
  cloudflared_tunnel_uuid: string | null;
  dns_routed: boolean;
  created_at: string;
  last_started_at: string | null;
}

export interface NewTunnelProfile {
  name: string;
  hostname: string;
  target_service_url: string;
}

export interface TunnelPatch {
  hostname?: string;
  target_service_url?: string;
}

export interface CertInfo {
  path: string;
  logged_in: boolean;
  account_email: string | null;
  expires_at: string | null;
}

export interface VersionInfo {
  version: string;
  raw: string;
}

export interface LogLine {
  ts: string;
  level: string;
  line: string;
  source: string;
}

export interface MetricsSnapshot {
  active_streams: number;
  total_requests: number;
  server_locations: string[];
  ha_connections: number;
  updated_at: string;
}

export interface RemoteTunnel {
  id: string;
  name: string;
  created_at: string | null;
  connections: number | null;
}

export interface AppStateSnapshot {
  settings: Settings;
  tunnels: TunnelProfile[];
  cert: CertInfo | null;
  running: Record<string, TunnelStatus>;
}

export interface PidEntry {
  tunnel_id: string;
  pid: number;
  tunnel_name: string;
  started_at: string;
}
