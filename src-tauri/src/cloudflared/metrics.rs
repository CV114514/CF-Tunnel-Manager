use crate::error::AppResult;
use crate::state::MetricsSnapshot;
use chrono::Utc;
use regex::Regex;
use std::time::Duration;

pub async fn poll_once(port: u16) -> AppResult<MetricsSnapshot> {
    let url = format!("http://127.0.0.1:{port}/metrics");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| crate::error::AppError::Other(format!("reqwest build: {e}")))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| crate::error::AppError::Other(format!("metrics fetch: {e}")))?;
    let text = resp
        .text()
        .await
        .map_err(|e| crate::error::AppError::Other(format!("metrics body: {e}")))?;
    Ok(parse_prometheus(&text))
}

pub fn parse_prometheus(text: &str) -> MetricsSnapshot {
    let active_re = Regex::new(r"cloudflared_tunnel_active_streams\{[^}]*\}\s+(\d+)").unwrap();
    let total_re = Regex::new(r"cloudflared_tunnel_total_requests\{[^}]*\}\s+(\d+)").unwrap();
    let ha_re = Regex::new(r"cloudflared_tunnel_ha_connections\s+(\d+)").unwrap();
    let loc_re = Regex::new(r#"cloudflared_tunnel_server_locations\{[^}]*colo="([^"]+)"[^}]*\}\s+\d+"#).unwrap();

    let active_streams = active_re
        .captures(text)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
        .unwrap_or(0);
    let total_requests = total_re
        .captures(text)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
        .unwrap_or(0);
    let ha_connections = ha_re
        .captures(text)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
        .unwrap_or(0);

    let mut locations = std::collections::HashSet::new();
    for cap in loc_re.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            locations.insert(m.as_str().to_string());
        }
    }

    MetricsSnapshot {
        active_streams,
        total_requests,
        server_locations: locations.into_iter().collect(),
        ha_connections,
        updated_at: Utc::now(),
    }
}
