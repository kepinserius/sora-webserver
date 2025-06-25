use std::collections::HashMap;
use std::path::{Path, PathBuf};
use regex::Regex;
use anyhow::{Context, Result};

use crate::config::{ServerConfig, VirtualHostConfig, BasicAuthConfig, LocationConfig};

// Represents a virtual host configuration after processing
#[derive(Debug, Clone)]
pub struct VirtualHost {
    pub server_name: String,
    pub document_root: PathBuf,
    pub index_files: Vec<String>,
    pub error_pages: HashMap<u16, String>,
    pub locations: Vec<Location>,
    pub aliases: HashMap<String, String>,
    pub ssl_certificate: Option<PathBuf>,
    pub ssl_certificate_key: Option<PathBuf>,
    pub enable_php: bool,
    pub enable_cgi: bool,
    pub rewrites: Vec<RewriteRule>,
    pub gzip: bool,
    pub gzip_types: Vec<String>,
    pub allow_methods: Vec<String>,
    pub deny_methods: Vec<String>,
    pub basic_auth: Option<BasicAuthConfig>,
    // Server name regex for wildcard matches
    server_name_regex: Option<Regex>,
}

// Represents a location within a virtual host
#[derive(Debug, Clone)]
pub struct Location {
    pub path: String,
    pub root: Option<PathBuf>,
    pub proxy_pass: Option<String>,
    pub allow_methods: Vec<String>,
    pub deny_methods: Vec<String>,
    pub basic_auth: Option<BasicAuthConfig>,
    pub allow_ips: Vec<String>,
    pub deny_ips: Vec<String>,
    // Path regex for pattern matches
    pub path_regex: Option<Regex>,
}

// URL rewrite rule
#[derive(Debug, Clone)]
pub struct RewriteRule {
    pub pattern: Regex,
    pub target: String,
    pub flags: Vec<String>,
}

impl VirtualHost {
    // Create a new virtual host from configuration
    pub fn new(config: &VirtualHostConfig, base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref();
        
        // Process document root path
        let document_root = if config.document_root.starts_with('/') {
            PathBuf::from(&config.document_root)
        } else {
            base_path.join(&config.document_root)
        };
        
        // Create locations
        let mut locations = Vec::new();
        for loc_config in &config.locations {
            locations.push(Location::new(loc_config, &document_root)?);
        }
        
        // Process aliases
        let mut aliases = HashMap::new();
        for alias in &config.aliases {
            aliases.insert(alias.path.clone(), alias.alias.clone());
        }
        
        // Process SSL certificate paths
        let ssl_certificate = config.ssl_certificate.as_ref().map(|cert| {
            if cert.starts_with('/') {
                PathBuf::from(cert)
            } else {
                base_path.join(cert)
            }
        });
        
        let ssl_certificate_key = config.ssl_certificate_key.as_ref().map(|key| {
            if key.starts_with('/') {
                PathBuf::from(key)
            } else {
                base_path.join(key)
            }
        });
        
        // Process rewrite rules
        let mut rewrites = Vec::new();
        for rewrite in &config.rewrites {
            let pattern = Regex::new(&rewrite.pattern)
                .with_context(|| format!("Invalid rewrite pattern: {}", rewrite.pattern))?;
            
            rewrites.push(RewriteRule {
                pattern,
                target: rewrite.target.clone(),
                flags: rewrite.flags.clone(),
            });
        }
        
        // Create server name regex for wildcard matching
        let server_name_regex = if config.server_name.contains('*') {
            let pattern = config.server_name.replace('.', "\\.")
                                           .replace('*', ".*");
            Some(Regex::new(&format!("^{}$", pattern))?)
        } else {
            None
        };
        
        Ok(Self {
            server_name: config.server_name.clone(),
            document_root,
            index_files: config.index.clone(),
            error_pages: config.error_pages.clone(),
            locations,
            aliases,
            ssl_certificate,
            ssl_certificate_key,
            enable_php: config.enable_php,
            enable_cgi: config.enable_cgi,
            rewrites,
            gzip: config.gzip,
            gzip_types: config.gzip_types.clone(),
            allow_methods: config.allow_methods.clone(),
            deny_methods: config.deny_methods.clone(),
            basic_auth: config.basic_auth.clone(),
            server_name_regex,
        })
    }
    
    // Check if this virtual host matches the given server name
    pub fn matches_server_name(&self, server_name: &str) -> bool {
        if self.server_name == server_name {
            return true;
        }
        
        if let Some(regex) = &self.server_name_regex {
            return regex.is_match(server_name);
        }
        
        false
    }
    
    // Find matching location for a path
    pub fn find_location(&self, path: &str) -> Option<&Location> {
        // First try exact path matches
        for location in &self.locations {
            if location.path == path {
                return Some(location);
            }
        }
        
        // Then try prefix matches (longest prefix first)
        let mut prefix_matches: Vec<&Location> = self.locations.iter()
            .filter(|loc| loc.path.ends_with('/') && path.starts_with(&loc.path))
            .collect();
        
        // Sort by path length (longest first)
        prefix_matches.sort_by(|a, b| b.path.len().cmp(&a.path.len()));
        
        if let Some(location) = prefix_matches.first() {
            return Some(location);
        }
        
        // Finally try regex matches
        for location in &self.locations {
            if let Some(regex) = &location.path_regex {
                if regex.is_match(path) {
                    return Some(location);
                }
            }
        }
        
        None
    }
    
    // Find an error page for a status code
    pub fn get_error_page(&self, status_code: u16) -> Option<&String> {
        self.error_pages.get(&status_code)
    }
}

impl Location {
    // Create a new location from configuration
    pub fn new(config: &LocationConfig, base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref();
        
        // Process root path
        let root = config.root.as_ref().map(|root| {
            if root.starts_with('/') {
                PathBuf::from(root)
            } else {
                base_path.join(root)
            }
        });
        
        // Create path regex for pattern matches
        let path_regex = if config.path.contains('*') || config.path.contains('?') || 
                           config.path.contains('(') || config.path.contains('|') {
            // Looks like a regex pattern
            Some(Regex::new(&config.path)
                .with_context(|| format!("Invalid location path pattern: {}", config.path))?)
        } else {
            None
        };
        
        Ok(Self {
            path: config.path.clone(),
            root,
            proxy_pass: config.proxy_pass.clone(),
            allow_methods: config.allow_methods.clone(),
            deny_methods: config.deny_methods.clone(),
            basic_auth: config.basic_auth.clone(),
            allow_ips: config.allow_ips.clone(),
            deny_ips: config.deny_ips.clone(),
            path_regex,
        })
    }
}

// Load virtual hosts from server configuration
pub async fn load_virtual_hosts(config: &ServerConfig) -> Result<Vec<VirtualHost>> {
    let mut vhosts = Vec::new();
    
    for vhost_config in &config.virtual_hosts {
        let vhost = VirtualHost::new(vhost_config, ".")?;
        vhosts.push(vhost);
    }
    
    Ok(vhosts)
}

// Find the best matching virtual host for a server name
pub fn find_virtual_host<'a>(vhosts: &'a [VirtualHost], server_name: &str) -> Option<&'a VirtualHost> {
    // First try exact matches
    for vhost in vhosts {
        if vhost.server_name == server_name {
            return Some(vhost);
        }
    }
    
    // Then try wildcard matches
    for vhost in vhosts {
        if vhost.matches_server_name(server_name) {
            return Some(vhost);
        }
    }
    
    // Default to the first virtual host if no match
    vhosts.first()
} 