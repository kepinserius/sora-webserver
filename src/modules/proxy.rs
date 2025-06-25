use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use hyper::{Body, Client, Method, Request, Response, StatusCode, Uri, client::HttpConnector};
use hyper_rustls::HttpsConnector;
use serde_json::Value;
use tracing::{debug, error};

use crate::modules::Module;

// Module for reverse proxy functionality
#[derive(Debug)]
pub struct ProxyModule {
    // HTTP client for making requests
    client: Client<HttpsConnector<HttpConnector>>,
    // Mapping of path prefixes to backend URLs
    backends: HashMap<String, String>,
    // Headers to remove when forwarding
    remove_headers: Vec<String>,
    // Headers to add when forwarding
    add_headers: HashMap<String, String>,
    // Timeout for proxy requests (seconds)
    timeout: u64,
    // Whether to forward the client IP
    forward_ip: bool,
}

impl Default for ProxyModule {
    fn default() -> Self {
        // Create HTTPS connector
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http1()
            .enable_http2()
            .build();
        
        // Create HTTP client with connector
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(32)
            .build(https);
        
        Self {
            client,
            backends: HashMap::new(),
            remove_headers: vec![
                "connection".to_string(),
                "keep-alive".to_string(),
                "transfer-encoding".to_string(),
                "te".to_string(),
                "trailer".to_string(),
                "proxy-authorization".to_string(),
                "proxy-authenticate".to_string(),
                "upgrade".to_string(),
                "expect".to_string(),
            ],
            add_headers: HashMap::new(),
            timeout: 30,
            forward_ip: true,
        }
    }
}

impl ProxyModule {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Add a backend mapping
    pub fn add_backend(&mut self, path_prefix: &str, backend_url: &str) {
        self.backends.insert(path_prefix.to_string(), backend_url.to_string());
    }
    
    // Set the request timeout
    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }
    
    // Enable/disable IP forwarding
    pub fn set_forward_ip(&mut self, enable: bool) {
        self.forward_ip = enable;
    }
    
    // Add a header to forward
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.add_headers.insert(name.to_string(), value.to_string());
    }
    
    // Find a matching backend for a path
    fn find_backend(&self, path: &str) -> Option<(&str, &str)> {
        // Find the longest matching prefix
        self.backends.iter()
            .filter(|(prefix, _)| path.starts_with(*prefix))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(prefix, backend)| (prefix.as_str(), backend.as_str()))
    }
    
    // Rewrite request URI for the backend
    fn rewrite_uri(&self, req: &Request<Body>, prefix: &str, backend: &str) -> Result<Uri> {
        let uri = req.uri();
        let path = uri.path();
        
        // Remove the prefix from the path
        let backend_path = if prefix == "/" {
            path.to_string()
        } else {
            path.replacen(prefix, "", 1)
        };
        
        // Build the new URI
        let mut backend_uri = backend.to_string();
        
        // Make sure backend URI ends with a slash if it's just a hostname
        if !backend_uri.ends_with('/') && !backend_uri.contains("?") && !backend_path.starts_with('/') {
            backend_uri.push('/');
        }
        
        // Add the backend path, making sure there's only one '/' between
        if backend_uri.ends_with('/') && backend_path.starts_with('/') {
            backend_uri.push_str(&backend_path[1..]);
        } else if !backend_uri.ends_with('/') && !backend_path.starts_with('/') {
            backend_uri.push('/');
            backend_uri.push_str(&backend_path);
        } else {
            backend_uri.push_str(&backend_path);
        }
        
        // Add query parameters if present
        if let Some(query) = uri.query() {
            backend_uri.push('?');
            backend_uri.push_str(query);
        }
        
        // Parse the new URI
        let uri = backend_uri.parse::<Uri>()
            .context("Failed to parse backend URI")?;
        
        Ok(uri)
    }
    
    // Create a proxied request
    fn create_proxied_request(&self, req: &Request<Body>, prefix: &str, backend: &str) -> Result<Request<Body>> {
        // Rewrite the URI
        let uri = self.rewrite_uri(req, prefix, backend)?;
        
        // Create a new request with the same method and body
        let mut proxied_req = Request::builder()
            .method(req.method())
            .uri(uri);
        
        // Copy headers from the original request
        let headers = proxied_req.headers_mut().unwrap();
        
        for (name, value) in req.headers() {
            // Skip headers that should be removed
            if !self.remove_headers.iter().any(|h| h.eq_ignore_ascii_case(name.as_str())) {
                headers.insert(name.clone(), value.clone());
            }
        }
        
        // Add X-Forwarded headers
        if self.forward_ip {
            // X-Forwarded-For
            let forwarded_for = if let Some(existing) = req.headers().get("x-forwarded-for") {
                let mut value = existing.to_str().unwrap_or("").to_string();
                
                // Add client IP if available
                if let Some(socket_addr) = req.extensions().get::<std::net::SocketAddr>() {
                    if !value.is_empty() {
                        value.push_str(", ");
                    }
                    value.push_str(&socket_addr.ip().to_string());
                }
                
                value
            } else if let Some(socket_addr) = req.extensions().get::<std::net::SocketAddr>() {
                socket_addr.ip().to_string()
            } else {
                "unknown".to_string()
            };
            
            headers.insert(
                "x-forwarded-for",
                hyper::header::HeaderValue::from_str(&forwarded_for)
                    .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("unknown")),
            );
            
            // X-Forwarded-Proto
            headers.insert(
                "x-forwarded-proto",
                hyper::header::HeaderValue::from_static(
                    if req.uri().scheme_str() == Some("https") { "https" } else { "http" }
                ),
            );
            
            // X-Forwarded-Host
            if let Some(host) = req.headers().get(hyper::header::HOST) {
                headers.insert("x-forwarded-host", host.clone());
            }
        }
        
        // Add custom headers
        for (name, value) in &self.add_headers {
            if let Ok(header_name) = hyper::header::HeaderName::from_bytes(name.as_bytes()) {
                if let Ok(header_value) = hyper::header::HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                }
            }
        }
        
        // Add Host header for the backend
        if let Some(host) = uri.host() {
            let host_value = if let Some(port) = uri.port() {
                format!("{}:{}", host, port)
            } else {
                host.to_string()
            };
            
            headers.insert(
                hyper::header::HOST,
                hyper::header::HeaderValue::from_str(&host_value)
                    .unwrap_or_else(|_| hyper::header::HeaderValue::from_static("unknown")),
            );
        }
        
        // Get the body from the original request
        let (parts, body) = req.clone().into_parts();
        
        // Build the request
        let proxied_req = proxied_req.body(body)
            .context("Failed to create proxied request")?;
        
        Ok(proxied_req)
    }
    
    // Proxy a request to a backend
    async fn proxy_request(&self, req: &Request<Body>) -> Result<Option<Response<Body>>> {
        // Find a matching backend
        if let Some((prefix, backend)) = self.find_backend(req.uri().path()) {
            debug!("Proxying request to backend: {}", backend);
            
            // Create a proxied request
            let proxied_req = self.create_proxied_request(req, prefix, backend)?;
            
            // Send the request with a timeout
            match tokio::time::timeout(
                Duration::from_secs(self.timeout),
                self.client.request(proxied_req)
            ).await {
                Ok(result) => {
                    match result {
                        Ok(mut response) => {
                            debug!(
                                "Proxy response: {} {} -> {}",
                                req.method(),
                                req.uri(),
                                response.status()
                            );
                            
                            // Remove hop-by-hop headers
                            for header in &self.remove_headers {
                                response.headers_mut().remove(header);
                            }
                            
                            // Add X-Proxied-By header
                            response.headers_mut().insert(
                                "x-proxied-by",
                                hyper::header::HeaderValue::from_static("Rust Web Server"),
                            );
                            
                            Ok(Some(response))
                        }
                        Err(e) => {
                            error!("Proxy error: {}", e);
                            
                            // Create a 502 Bad Gateway response
                            let mut response = Response::new(Body::from("Bad Gateway"));
                            *response.status_mut() = StatusCode::BAD_GATEWAY;
                            Ok(Some(response))
                        }
                    }
                }
                Err(_) => {
                    // Request timed out
                    let mut response = Response::new(Body::from("Gateway Timeout"));
                    *response.status_mut() = StatusCode::GATEWAY_TIMEOUT;
                    Ok(Some(response))
                }
            }
        } else {
            // No matching backend
            Ok(None)
        }
    }
}

#[async_trait]
impl Module for ProxyModule {
    fn name(&self) -> &str {
        "proxy"
    }
    
    fn init(&mut self, config: &Value) -> Result<()> {
        if let Some(obj) = config.as_object() {
            // Load backends
            if let Some(backends) = obj.get("backends").and_then(Value::as_object) {
                for (prefix, backend) in backends {
                    if let Some(backend_url) = backend.as_str() {
                        self.add_backend(prefix, backend_url);
                    }
                }
            }
            
            // Load timeout
            if let Some(timeout) = obj.get("timeout").and_then(Value::as_u64) {
                self.set_timeout(timeout);
            }
            
            // Load forward_ip setting
            if let Some(forward_ip) = obj.get("forward_ip").and_then(Value::as_bool) {
                self.set_forward_ip(forward_ip);
            }
            
            // Load custom headers
            if let Some(headers) = obj.get("headers").and_then(Value::as_object) {
                for (name, value) in headers {
                    if let Some(header_value) = value.as_str() {
                        self.add_header(name, header_value);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn pre_process(&self, req: &mut Request<Body>) -> Result<Option<Response<Body>>> {
        // Check if we should proxy this request
        self.proxy_request(req).await
    }
    
    async fn post_process(&self, _res: &mut Response<Body>, _req: &Request<Body>) -> Result<()> {
        // No post-processing for this module
        Ok(())
    }
    
    fn cleanup(&self) -> Result<()> {
        // No cleanup needed
        Ok(())
    }
} 