use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::Arc;

use cidr::{Cidr, Ipv4Cidr, Ipv6Cidr};
use hyper::{Body, Request, Response, StatusCode};
use regex::Regex;
use once_cell::sync::Lazy;

// Common attack patterns to block
static ATTACK_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // SQL injection
        Regex::new(r"(?i)(\%27)|(')|(--)|(%23)|(#)|(%3B)|(;)").unwrap(),
        Regex::new(r"(?i)(select\s+)|(\s+union\s+)|(\s+insert\s+)|(\s+drop\s+)|(\s+update\s+)|(\s+delete\s+)").unwrap(),
        
        // XSS
        Regex::new(r"(?i)<script.*?>").unwrap(),
        Regex::new(r"(?i)on\w+\s*=").unwrap(),
        Regex::new(r"(?i)javascript\s*:").unwrap(),
        
        // Local/Remote file inclusion
        Regex::new(r"(?i)(/etc/passwd)|(/etc/shadow)|(/proc/self/environ)|(wp-config\.php)").unwrap(),
        Regex::new(r"(?i)(\.\./)|(\.\.\\)").unwrap(),
        
        // Command injection
        Regex::new(r"(?i)(\|\s*\w+)|(\&\s*\w+)|(;\s*\w+)|(system\s*\()|(exec\s*\()").unwrap(),
    ]
});

// Firewall rules for IP blocking, path filtering, etc.
pub struct Firewall {
    // IP addresses to block
    blocked_ips: HashSet<IpAddr>,
    // IP ranges to block
    blocked_ip_ranges: Vec<IpRange>,
    // Allowed HTTP methods
    allowed_methods: HashSet<String>,
    // Blocked request paths
    blocked_paths: HashSet<String>,
    // Blocked path patterns
    blocked_path_patterns: Vec<Regex>,
    // Whether to enable attack pattern detection
    detect_attacks: bool,
}

// Represents an IP range using CIDR
enum IpRange {
    V4(Ipv4Cidr),
    V6(Ipv6Cidr),
}

impl IpRange {
    fn contains(&self, ip: &IpAddr) -> bool {
        match (self, ip) {
            (IpRange::V4(range), IpAddr::V4(addr)) => range.contains(addr),
            (IpRange::V6(range), IpAddr::V6(addr)) => range.contains(addr),
            _ => false, // IPv4 range can't contain IPv6 address and vice versa
        }
    }
}

impl Default for Firewall {
    fn default() -> Self {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert("GET".to_string());
        allowed_methods.insert("HEAD".to_string());
        allowed_methods.insert("POST".to_string());
        allowed_methods.insert("PUT".to_string());
        allowed_methods.insert("DELETE".to_string());
        allowed_methods.insert("OPTIONS".to_string());
        
        Self {
            blocked_ips: HashSet::new(),
            blocked_ip_ranges: Vec::new(),
            allowed_methods,
            blocked_paths: HashSet::new(),
            blocked_path_patterns: Vec::new(),
            detect_attacks: true,
        }
    }
}

impl Firewall {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Block a specific IP address
    pub fn block_ip(&mut self, ip: IpAddr) {
        self.blocked_ips.insert(ip);
    }
    
    // Block an IP range using CIDR notation
    pub fn block_ip_range(&mut self, cidr: &str) -> Result<(), String> {
        if cidr.contains('.') {
            // IPv4
            match Ipv4Cidr::from_str(cidr) {
                Ok(range) => {
                    self.blocked_ip_ranges.push(IpRange::V4(range));
                    Ok(())
                }
                Err(_) => Err(format!("Invalid IPv4 CIDR: {}", cidr)),
            }
        } else {
            // IPv6
            match Ipv6Cidr::from_str(cidr) {
                Ok(range) => {
                    self.blocked_ip_ranges.push(IpRange::V6(range));
                    Ok(())
                }
                Err(_) => Err(format!("Invalid IPv6 CIDR: {}", cidr)),
            }
        }
    }
    
    // Set allowed HTTP methods (overrides defaults)
    pub fn set_allowed_methods(&mut self, methods: Vec<String>) {
        self.allowed_methods = methods.into_iter().collect();
    }
    
    // Block a specific path
    pub fn block_path(&mut self, path: &str) {
        self.blocked_paths.insert(path.to_string());
    }
    
    // Block paths matching a pattern
    pub fn block_path_pattern(&mut self, pattern: &str) -> Result<(), String> {
        match Regex::new(pattern) {
            Ok(regex) => {
                self.blocked_path_patterns.push(regex);
                Ok(())
            }
            Err(e) => Err(format!("Invalid regex pattern: {}", e)),
        }
    }
    
    // Enable or disable attack pattern detection
    pub fn set_attack_detection(&mut self, enabled: bool) {
        self.detect_attacks = enabled;
    }
    
    // Check if a request should be blocked
    pub fn should_block(&self, req: &Request<Body>) -> Option<StatusCode> {
        // Check client IP
        if let Some(ip) = crate::security::rate_limiter::RateLimiter::get_client_ip(req) {
            if self.blocked_ips.contains(&ip) {
                return Some(StatusCode::FORBIDDEN);
            }
            
            for range in &self.blocked_ip_ranges {
                if range.contains(&ip) {
                    return Some(StatusCode::FORBIDDEN);
                }
            }
        }
        
        // Check HTTP method
        if !self.allowed_methods.contains(req.method().as_str()) {
            return Some(StatusCode::METHOD_NOT_ALLOWED);
        }
        
        // Get the path from the request
        let path = req.uri().path();
        
        // Check blocked paths
        if self.blocked_paths.contains(path) {
            return Some(StatusCode::FORBIDDEN);
        }
        
        // Check blocked path patterns
        for pattern in &self.blocked_path_patterns {
            if pattern.is_match(path) {
                return Some(StatusCode::FORBIDDEN);
            }
        }
        
        // Check for attack patterns in path and query
        if self.detect_attacks {
            let uri = req.uri().to_string();
            
            for pattern in ATTACK_PATTERNS.iter() {
                if pattern.is_match(&uri) {
                    return Some(StatusCode::BAD_REQUEST);
                }
            }
            
            // Check headers for attack patterns
            for (_, value) in req.headers() {
                if let Ok(header_str) = value.to_str() {
                    for pattern in ATTACK_PATTERNS.iter() {
                        if pattern.is_match(header_str) {
                            return Some(StatusCode::BAD_REQUEST);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    // Create a response for blocked requests
    pub fn create_blocked_response(status: StatusCode) -> Response<Body> {
        let body = match status {
            StatusCode::FORBIDDEN => "Forbidden",
            StatusCode::METHOD_NOT_ALLOWED => "Method Not Allowed",
            StatusCode::BAD_REQUEST => "Bad Request",
            _ => "Request Blocked",
        };
        
        let mut resp = Response::new(Body::from(body));
        *resp.status_mut() = status;
        
        if status == StatusCode::METHOD_NOT_ALLOWED {
            // Add allowed methods header
            resp.headers_mut().insert(
                hyper::header::ALLOW,
                hyper::header::HeaderValue::from_static("GET, HEAD, POST"),
            );
        }
        
        resp
    }
} 