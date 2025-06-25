use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use tokio::fs::read_to_string;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub security: SecuritySettings,
    pub logging: LoggingSettings,
    pub virtual_hosts: Vec<VirtualHostConfig>,
    pub modules: Vec<ModuleConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerSettings {
    pub name: String,
    pub workers: usize,
    pub keep_alive: bool,
    pub keep_alive_timeout: u64,
    pub max_connections: usize,
    pub request_timeout: u64,
    pub send_timeout: u64,
    pub client_header_timeout: u64,
    pub client_body_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SecuritySettings {
    pub enable_https: bool,
    pub enable_hsts: bool,
    pub hsts_max_age: u64,
    pub enable_cors: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub enable_csrf_protection: bool,
    pub enable_xss_protection: bool,
    pub enable_clickjacking_protection: bool,
    pub enable_csp: bool,
    pub content_security_policy: String,
    pub ssl_protocols: Vec<String>,
    pub ssl_ciphers: Vec<String>,
    pub dhparam_file: Option<String>,
    pub enable_ocsp_stapling: bool,
    pub ocsp_responder: Option<String>,
    pub enable_http2: bool,
    pub rate_limiting: bool,
    pub rate_limit_requests: usize,
    pub rate_limit_window: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingSettings {
    pub level: String,
    pub access_log: String,
    pub error_log: String,
    pub log_format: String,
    pub buffer_size: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VirtualHostConfig {
    pub server_name: String,
    pub document_root: String,
    pub index: Vec<String>,
    pub error_pages: HashMap<u16, String>,
    pub locations: Vec<LocationConfig>,
    pub aliases: Vec<AliasConfig>,
    pub ssl_certificate: Option<String>,
    pub ssl_certificate_key: Option<String>,
    pub enable_php: bool,
    pub enable_cgi: bool,
    pub rewrites: Vec<RewriteRule>,
    pub gzip: bool,
    pub gzip_types: Vec<String>,
    pub allow_methods: Vec<String>,
    pub deny_methods: Vec<String>,
    pub basic_auth: Option<BasicAuthConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LocationConfig {
    pub path: String,
    pub root: Option<String>,
    pub proxy_pass: Option<String>,
    pub allow_methods: Vec<String>,
    pub deny_methods: Vec<String>,
    pub basic_auth: Option<BasicAuthConfig>,
    pub allow_ips: Vec<String>,
    pub deny_ips: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AliasConfig {
    pub path: String,
    pub alias: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RewriteRule {
    pub pattern: String,
    pub target: String,
    pub flags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BasicAuthConfig {
    pub realm: String,
    pub users: HashMap<String, String>, // username -> hashed password
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModuleConfig {
    pub name: String,
    pub enabled: bool,
    pub settings: serde_json::Value,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                name: "Rust Web Server".to_string(),
                workers: num_cpus::get(),
                keep_alive: true,
                keep_alive_timeout: 65,
                max_connections: 1024,
                request_timeout: 60,
                send_timeout: 60,
                client_header_timeout: 60,
                client_body_timeout: 60,
            },
            security: SecuritySettings {
                enable_https: false,
                enable_hsts: true,
                hsts_max_age: 31536000,
                enable_cors: true,
                allowed_origins: vec!["*".to_string()],
                allowed_methods: vec!["GET".to_string(), "POST".to_string(), "HEAD".to_string()],
                enable_csrf_protection: true,
                enable_xss_protection: true,
                enable_clickjacking_protection: true,
                enable_csp: true,
                content_security_policy: "default-src 'self';".to_string(),
                ssl_protocols: vec!["TLSv1.2".to_string(), "TLSv1.3".to_string()],
                ssl_ciphers: vec![
                    "ECDHE-ECDSA-AES128-GCM-SHA256".to_string(),
                    "ECDHE-RSA-AES128-GCM-SHA256".to_string(),
                    "ECDHE-ECDSA-AES256-GCM-SHA384".to_string(),
                    "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
                ],
                dhparam_file: None,
                enable_ocsp_stapling: false,
                ocsp_responder: None,
                enable_http2: true,
                rate_limiting: true,
                rate_limit_requests: 100,
                rate_limit_window: 60,
            },
            logging: LoggingSettings {
                level: "info".to_string(),
                access_log: "logs/access.log".to_string(),
                error_log: "logs/error.log".to_string(),
                log_format: "$remote_addr - $remote_user [$time_local] \"$request\" $status $body_bytes_sent".to_string(),
                buffer_size: 4096,
            },
            virtual_hosts: vec![
                VirtualHostConfig {
                    server_name: "localhost".to_string(),
                    document_root: "www".to_string(),
                    index: vec!["index.html".to_string(), "index.htm".to_string()],
                    error_pages: HashMap::new(),
                    locations: Vec::new(),
                    aliases: Vec::new(),
                    ssl_certificate: None,
                    ssl_certificate_key: None,
                    enable_php: false,
                    enable_cgi: false,
                    rewrites: Vec::new(),
                    gzip: true,
                    gzip_types: vec!["text/plain".to_string(), "text/html".to_string(), "text/css".to_string(), "application/javascript".to_string()],
                    allow_methods: vec!["GET".to_string(), "HEAD".to_string(), "POST".to_string()],
                    deny_methods: Vec::new(),
                    basic_auth: None,
                }
            ],
            modules: Vec::new(),
        }
    }
}

pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    let config_str = read_to_string(path).await.context("Failed to read config file")?;
    
    // Parse the TOML config file
    let config: ServerConfig = toml::from_str(&config_str).context("Failed to parse config file")?;
    
    Ok(config)
}

pub async fn save_config<P: AsRef<Path>>(config: &ServerConfig, path: P) -> Result<()> {
    let config_str = toml::to_string_pretty(config).context("Failed to serialize config")?;
    tokio::fs::write(path, config_str).await.context("Failed to write config file")?;
    Ok(())
}

pub fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = ServerConfig::default();
    let config_str = toml::to_string_pretty(&config).context("Failed to serialize default config")?;
    std::fs::write(path, config_str).context("Failed to write default config file")?;
    Ok(())
} 