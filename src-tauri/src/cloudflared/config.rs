use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub tunnel: String,
    #[serde(rename = "credentials-file")]
    pub credentials_file: String,
    pub ingress: Vec<IngressRule>,
}

pub const NAME_RE: &str = r"^[a-z0-9][a-z0-9-]{0,62}[a-z0-9]$";

pub fn validate_name(name: &str) -> AppResult<()> {
    let re = regex::Regex::new(NAME_RE)?;
    if !re.is_match(name) {
        return Err(AppError::InvalidInput(format!(
            "tunnel name must match {NAME_RE}, got: {name}"
        )));
    }
    Ok(())
}

pub fn validate_hostname(hostname: &str) -> AppResult<()> {
    if hostname.is_empty() {
        return Err(AppError::InvalidInput("hostname is empty".to_string()));
    }
    let labels: Vec<&str> = hostname.split('.').collect();
    if labels.len() < 2 {
        return Err(AppError::InvalidInput(format!(
            "hostname '{hostname}' must have at least 2 dot-separated labels"
        )));
    }
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return Err(AppError::InvalidInput(format!(
                "invalid label in hostname: {hostname}"
            )));
        }
        if !label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::InvalidInput(format!(
                "invalid char in hostname label: {label}"
            )));
        }
    }
    Ok(())
}

pub fn validate_service_url(url: &str) -> AppResult<()> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::InvalidInput(format!(
            "service url must start with http:// or https://, got: {url}"
        )));
    }
    Ok(())
}

pub fn write_config(
    path: &Path,
    tunnel_name: &str,
    credentials_file: &str,
    hostname: &str,
    service_url: &str,
) -> AppResult<()> {
    let config = TunnelConfig {
        tunnel: tunnel_name.to_string(),
        credentials_file: credentials_file.to_string(),
        ingress: vec![
            IngressRule {
                hostname: Some(hostname.to_string()),
                service: service_url.to_string(),
            },
            IngressRule {
                hostname: None,
                service: "http_status:404".to_string(),
            },
        ],
    };
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let yaml = serde_yaml::to_string(&config)?;
    std::fs::write(path, yaml)?;
    Ok(())
}
