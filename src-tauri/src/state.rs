use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub cloudflared_exe_path: Option<PathBuf>,
    #[serde(default = "default_metrics_interval")]
    pub metrics_poll_interval_secs: u64,
    #[serde(default = "default_true")]
    pub always_cleanup_orphans: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub cloudflared_home_override: Option<PathBuf>,
}

fn default_metrics_interval() -> u64 { 5 }
fn default_true() -> bool { true }
fn default_theme() -> String { "system".to_string() }

impl Default for Settings {
    fn default() -> Self {
        Self {
            cloudflared_exe_path: None,
            metrics_poll_interval_secs: 5,
            always_cleanup_orphans: true,
            theme: "system".to_string(),
            cloudflared_home_override: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelProfile {
    pub id: Uuid,
    pub name: String,
    pub hostname: String,
    pub target_service_url: String,
    pub config_yaml_path: PathBuf,
    #[serde(default)]
    pub credentials_json_path: Option<PathBuf>,
    #[serde(default)]
    pub cloudflared_tunnel_uuid: Option<String>,
    #[serde(default)]
    pub dns_routed: bool,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTunnelProfile {
    pub name: String,
    pub hostname: String,
    pub target_service_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelPatch {
    pub hostname: Option<String>,
    pub target_service_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub tunnels: Vec<TunnelProfile>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            tunnels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertInfo {
    pub path: String,
    pub logged_in: bool,
    pub account_email: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateSnapshot {
    pub settings: Settings,
    pub tunnels: Vec<TunnelProfile>,
    pub cert: Option<CertInfo>,
    pub running: HashMap<String, TunnelStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Connected { count: u32, edge_locations: Vec<String> },
    Error(String),
    Stopping,
}

impl TunnelStatus {
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            TunnelStatus::Starting | TunnelStatus::Connected { .. } | TunnelStatus::Stopping
        )
    }
    pub fn aggregate_color(&self) -> &'static str {
        match self {
            TunnelStatus::Connected { .. } => "green",
            TunnelStatus::Starting | TunnelStatus::Stopping => "yellow",
            TunnelStatus::Error(_) => "red",
            TunnelStatus::Stopped => "grey",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub ts: DateTime<Utc>,
    pub level: String,
    pub line: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub active_streams: u64,
    pub total_requests: u64,
    pub server_locations: Vec<String>,
    pub ha_connections: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTunnel {
    pub id: String,
    pub name: String,
    pub created_at: Option<String>,
    pub connections: Option<u32>,
}

pub struct RunningTunnel {
    pub child: Child,
    pub pid: u32,
    pub metrics_port: Option<u16>,
    pub status: TunnelStatus,
    pub log_buffer: VecDeque<LogLine>,
    pub metrics_handle: Option<tokio::task::JoinHandle<()>>,
}

impl RunningTunnel {
    pub fn new(child: Child, pid: u32) -> Self {
        Self {
            child,
            pid,
            metrics_port: None,
            status: TunnelStatus::Starting,
            log_buffer: VecDeque::with_capacity(1024),
            metrics_handle: None,
        }
    }
}

pub struct Session {
    pub running: HashMap<Uuid, RunningTunnel>,
    pub cert: Option<CertInfo>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            running: HashMap::new(),
            cert: None,
        }
    }
}

pub type SharedState = Arc<Mutex<PersistedState>>;
pub type SharedSession = Arc<Mutex<Session>>;

pub const LOG_BUFFER_CAP: usize = 1000;
