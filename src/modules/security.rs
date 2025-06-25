use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response, StatusCode, header};
use serde_json::Value;
use tracing::debug;

use crate::modules::Module;
use crate::security::{firewall::Firewall, rate_limiter::RateLimiter};
use crate::security::headers;

// Module for applying security features
#[derive(Debug)]
pub struct SecurityModule {
    // Firewall for filtering requests
    firewall: Firewall,
    // Whether rate limiting is enabled
    rate_limiting: bool,
    // Whether to add security headers
    add_security_headers: bool,
    // Security headers to add
    security_headers: crate::config::SecuritySettings,
}

impl Default for SecurityModule {
    fn default() -> Self {
        Self {
            firewall: Firewall::new(),
            rate_limiting: false,
            add_security_headers: true,
            security_headers: crate::config::SecuritySettings::default(),
        }
    }
}

impl SecurityModule {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Enable/disable rate limiting
    pub fn with_rate_limiting(mut self, enable: bool) -> Self {
        self.rate_limiting = enable;
        self
    }
    
    // Enable/disable security headers
    pub fn with_security_headers(mut self, enable: bool) -> Self {
        self.add_security_headers = enable;
        self
    }
    
    // Set allowed HTTP methods
    pub fn with_allowed_methods(mut self, methods: Vec<String>) -> Self {
        self.firewall.set_allowed_methods(methods);
        self
    }
    
    // Block an IP address
    pub fn block_ip(mut self, ip: &str) -> Self {
        if let Ok(ip_addr) = ip.parse() {
            self.firewall.block_ip(ip_addr);
        }
        self
    }
    
    // Block an IP range
    pub fn block_ip_range(mut self, cidr: &str) -> Self {
        let _ = self.firewall.block_ip_range(cidr);
        self
    }
}

#[async_trait]
impl Module for SecurityModule {
    fn name(&self) -> &str {
        "security"
    }
    
    fn init(&mut self, config: &Value) -> Result<()> {
        if let Some(obj) = config.as_object() {
            // Initialize rate limiting
            if let Some(rate_limit_obj) = obj.get("rate_limiting").and_then(Value::as_object) {
                let enabled = rate_limit_obj.get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                    
                if enabled {
                    self.rate_limiting = true;
                    
                    let requests = rate_limit_obj.get("requests")
                        .and_then(Value::as_u64)
                        .unwrap_or(100);
                        
                    let window = rate_limit_obj.get("window")
                        .and_then(Value::as_u64)
                        .unwrap_or(60);
                        
                    // Setup global rate limiter
                    RateLimiter::setup_global(requests as usize, window);
                }
            }
            
            // Initialize firewall
            if let Some(firewall_obj) = obj.get("firewall").and_then(Value::as_object) {
                // Process allowed methods
                if let Some(methods_array) = firewall_obj.get("allowed_methods").and_then(Value::as_array) {
                    let methods = methods_array.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>();
                    
                    if !methods.is_empty() {
                        self.firewall.set_allowed_methods(methods);
                    }
                }
                
                // Process blocked IPs
                if let Some(ips_array) = firewall_obj.get("blocked_ips").and_then(Value::as_array) {
                    for ip_value in ips_array {
                        if let Some(ip_str) = ip_value.as_str() {
                            if let Ok(ip_addr) = ip_str.parse() {
                                self.firewall.block_ip(ip_addr);
                            }
                        }
                    }
                }
                
                // Process blocked IP ranges
                if let Some(ranges_array) = firewall_obj.get("blocked_ranges").and_then(Value::as_array) {
                    for range_value in ranges_array {
                        if let Some(range_str) = range_value.as_str() {
                            let _ = self.firewall.block_ip_range(range_str);
                        }
                    }
                }
                
                // Process blocked paths
                if let Some(paths_array) = firewall_obj.get("blocked_paths").and_then(Value::as_array) {
                    for path_value in paths_array {
                        if let Some(path_str) = path_value.as_str() {
                            self.firewall.block_path(path_str);
                        }
                    }
                }
                
                // Process attack detection
                if let Some(detect_attacks) = firewall_obj.get("detect_attacks").and_then(Value::as_bool) {
                    self.firewall.set_attack_detection(detect_attacks);
                }
            }
            
            // Initialize security headers
            if let Some(headers_obj) = obj.get("headers").and_then(Value::as_object) {
                if let Some(enable) = headers_obj.get("enabled").and_then(Value::as_bool) {
                    self.add_security_headers = enable;
                }
                
                // Load header configuration
                if let Some(hsts_enabled) = headers_obj.get("hsts").and_then(Value::as_bool) {
                    self.security_headers.enable_hsts = hsts_enabled;
                }
                
                if let Some(hsts_age) = headers_obj.get("hsts_max_age").and_then(Value::as_u64) {
                    self.security_headers.hsts_max_age = hsts_age;
                }
                
                if let Some(xss_protection) = headers_obj.get("xss_protection").and_then(Value::as_bool) {
                    self.security_headers.enable_xss_protection = xss_protection;
                }
                
                if let Some(frame_options) = headers_obj.get("frame_options").and_then(Value::as_bool) {
                    self.security_headers.enable_clickjacking_protection = frame_options;
                }
                
                if let Some(csp_enabled) = headers_obj.get("content_security_policy").and_then(Value::as_bool) {
                    self.security_headers.enable_csp = csp_enabled;
                }
                
                if let Some(csp_value) = headers_obj.get("csp_value").and_then(Value::as_str) {
                    self.security_headers.content_security_policy = csp_value.to_string();
                }
            }
        }
        
        Ok(())
    }
    
    async fn pre_process(&self, req: &mut Request<Body>) -> Result<Option<Response<Body>>> {
        // Check firewall rules
        if let Some(status) = self.firewall.should_block(req) {
            debug!("Firewall blocked request: {}", status);
            return Ok(Some(Firewall::create_blocked_response(status)));
        }
        
        // Check rate limit
        if self.rate_limiting {
            if let Some(response) = crate::security::rate_limiter::check_rate_limit(req).await {
                debug!("Rate limit exceeded");
                return Ok(Some(response));
            }
        }
        
        Ok(None)
    }
    
    async fn post_process(&self, res: &mut Response<Body>, req: &Request<Body>) -> Result<()> {
        // Add security headers if enabled
        if self.add_security_headers {
            let is_https = req.uri().scheme_str() == Some("https");
            headers::add_security_headers(res, &self.security_headers, is_https);
        }
        
        Ok(())
    }
    
    fn cleanup(&self) -> Result<()> {
        // No cleanup needed
        Ok(())
    }
} 