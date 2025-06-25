use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use hyper::{Body, Method, Request, Response, StatusCode, header};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use mime_guess::from_path;
use tracing::{debug, error, info};

use crate::handlers::error;
use crate::modules::Module;
use crate::security;
use crate::vhost::{VirtualHost, Location, find_virtual_host};
use crate::security::auth;

// Main request handler for the server
pub struct RequestHandler {
    // Path to the document root
    doc_root: PathBuf,
    // List of virtual hosts
    vhosts: Vec<VirtualHost>,
    // List of modules
    modules: Vec<Box<dyn Module>>,
}

impl RequestHandler {
    // Create a new request handler
    pub fn new(
        doc_root: PathBuf,
        vhosts: Vec<VirtualHost>,
        modules: Vec<Box<dyn Module>>,
    ) -> Self {
        Self {
            doc_root,
            vhosts,
            modules,
        }
    }
    
    // Handle an incoming HTTP request
    pub async fn handle(&self, req: Request<Body>) -> Result<Response<Body>> {
        let start_time = Instant::now();
        
        // Log the request
        debug!(
            "Handling request: {} {}",
            req.method(),
            req.uri().path()
        );
        
        // Extract the client IP for logging
        let client_ip = security::rate_limiter::RateLimiter::get_client_ip(&req)
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        // Execute pre-processing modules
        for module in &self.modules {
            match module.pre_process(&mut Request::clone(&req)).await {
                Ok(Some(response)) => {
                    // Module handled the request, return the response
                    let processing_time = start_time.elapsed();
                    let bytes_sent = response.body().size_hint().upper().unwrap_or(0);
                    
                    crate::logging::log_request(
                        &req,
                        &response,
                        &client_ip,
                        processing_time,
                        bytes_sent as usize,
                    );
                    
                    return Ok(response);
                },
                Ok(None) => {
                    // Module didn't handle the request, continue
                },
                Err(e) => {
                    // Module encountered an error
                    error!("Module error in pre-processing: {}", e);
                    crate::logging::log_error(&e, Some(&req), Some(&client_ip));
                    return Ok(error::internal_server_error());
                }
            }
        }
        
        // Find the appropriate virtual host for this request
        let host = req.headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");
            
        let vhost = find_virtual_host(&self.vhosts, host)
            .unwrap_or_else(|| self.vhosts.first().expect("No virtual hosts configured"));
        
        // Find matching location for the path
        let path = req.uri().path();
        let location = vhost.find_location(path);
        
        // Handle the request based on method
        let mut response = match *req.method() {
            Method::GET | Method::HEAD => {
                self.handle_get(req.method() == &Method::HEAD, path, vhost, location).await?
            },
            Method::POST => {
                self.handle_post(&req).await?
            },
            Method::PUT => {
                self.handle_put(&req).await?
            },
            Method::DELETE => {
                self.handle_delete(&req).await?
            },
            Method::OPTIONS => {
                self.handle_options(&req, vhost).await?
            },
            _ => {
                let mut resp = Response::new(Body::from("Method Not Allowed"));
                *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                resp
            }
        };
        
        // Execute post-processing modules
        for module in &self.modules {
            if let Err(e) = module.post_process(&mut response, &req).await {
                error!("Module error in post-processing: {}", e);
                crate::logging::log_error(&e, Some(&req), Some(&client_ip));
                // Continue with other modules
            }
        }
        
        // Log the completed request
        let processing_time = start_time.elapsed();
        let bytes_sent = response.body().size_hint().upper().unwrap_or(0);
        
        crate::logging::log_request(
            &req,
            &response,
            &client_ip,
            processing_time,
            bytes_sent as usize,
        );
        
        Ok(response)
    }
    
    // Handle GET and HEAD requests
    async fn handle_get(
        &self,
        head_only: bool,
        path: &str,
        vhost: &VirtualHost,
        location: Option<&Location>,
    ) -> Result<Response<Body>> {
        // Check if path is a special URL
        if path == "/favicon.ico" {
            return self.serve_favicon(vhost).await;
        }
        
        // Check if location has basic auth
        if let Some(loc) = location {
            if let Some(auth_config) = &loc.basic_auth {
                // TODO: Implement basic auth check
            }
        } else if let Some(auth_config) = &vhost.basic_auth {
            // TODO: Implement basic auth check
        }
        
        // Sanitize the path to prevent traversal attacks
        let sanitized_path = security::sanitize_path(path);
        if security::is_path_traversal(&sanitized_path) {
            return Ok(error::forbidden());
        }
        
        // Determine the file path
        let doc_root = if let Some(loc) = location {
            if let Some(root) = &loc.root {
                root.clone()
            } else {
                vhost.document_root.clone()
            }
        } else {
            vhost.document_root.clone()
        };
        
        let mut file_path = doc_root.join(&sanitized_path[1..]); // Remove leading /
        
        // Check if the path is a directory
        if file_path.is_dir() {
            // Try each index file
            for index in &vhost.index_files {
                let index_path = file_path.join(index);
                if index_path.exists() {
                    file_path = index_path;
                    break;
                }
            }
        }
        
        // If it's still a directory, return 403
        if file_path.is_dir() {
            return Ok(error::forbidden());
        }
        
        // Check if the file exists
        if !file_path.exists() {
            // Look for custom error page
            if let Some(error_page) = vhost.get_error_page(404) {
                let error_path = doc_root.join(error_page);
                if error_path.exists() {
                    return self.serve_file(&error_path, head_only).await;
                }
            }
            return Ok(error::not_found());
        }
        
        // Serve the file
        self.serve_file(&file_path, head_only).await
    }
    
    // Serve a file
    async fn serve_file(&self, file_path: &Path, head_only: bool) -> Result<Response<Body>> {
        // Open the file
        let mut file = match File::open(file_path).await {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to open file {:?}: {}", file_path, e);
                return Ok(error::not_found());
            }
        };
        
        // Get file metadata
        let metadata = match file.metadata().await {
            Ok(meta) => meta,
            Err(e) => {
                error!("Failed to read metadata for {:?}: {}", file_path, e);
                return Ok(error::internal_server_error());
            }
        };
        
        // Read the file content if not head request
        let body = if head_only {
            Body::empty()
        } else {
            let mut contents = Vec::with_capacity(metadata.len() as usize);
            if let Err(e) = file.read_to_end(&mut contents).await {
                error!("Failed to read file {:?}: {}", file_path, e);
                return Ok(error::internal_server_error());
            }
            Body::from(contents)
        };
        
        // Determine content type
        let content_type = from_path(file_path)
            .first_or_octet_stream()
            .as_ref()
            .to_owned();
        
        // Create the response
        let mut response = Response::new(body);
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_str(&content_type)
                .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
        );
        
        // Add content length
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            header::HeaderValue::from(metadata.len()),
        );
        
        // Add Last-Modified header
        if let Ok(modified) = metadata.modified() {
            if let Ok(modified_time) = httpdate::fmt_http_date(modified) {
                if let Ok(header_value) = header::HeaderValue::from_str(&modified_time) {
                    response.headers_mut().insert(header::LAST_MODIFIED, header_value);
                }
            }
        }
        
        Ok(response)
    }
    
    // Serve favicon.ico
    async fn serve_favicon(&self, vhost: &VirtualHost) -> Result<Response<Body>> {
        // Look for favicon in standard locations
        let favicon_paths = [
            vhost.document_root.join("favicon.ico"),
            vhost.document_root.join("images/favicon.ico"),
            vhost.document_root.join("img/favicon.ico"),
            vhost.document_root.join("static/favicon.ico"),
        ];
        
        for path in &favicon_paths {
            if path.exists() {
                return self.serve_file(path, false).await;
            }
        }
        
        // No favicon found
        Ok(error::not_found())
    }
    
    // Handle POST requests
    async fn handle_post(&self, req: &Request<Body>) -> Result<Response<Body>> {
        // TODO: Implement POST handling (for form submissions, etc)
        
        // For now, just return 405 Method Not Allowed
        let mut response = Response::new(Body::from("Method Not Allowed"));
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        response.headers_mut().insert(
            header::ALLOW,
            header::HeaderValue::from_static("GET, HEAD, OPTIONS"),
        );
        
        Ok(response)
    }
    
    // Handle PUT requests
    async fn handle_put(&self, req: &Request<Body>) -> Result<Response<Body>> {
        // TODO: Implement PUT handling (for uploads, etc)
        
        // For now, just return 405 Method Not Allowed
        let mut response = Response::new(Body::from("Method Not Allowed"));
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        response.headers_mut().insert(
            header::ALLOW,
            header::HeaderValue::from_static("GET, HEAD, OPTIONS"),
        );
        
        Ok(response)
    }
    
    // Handle DELETE requests
    async fn handle_delete(&self, req: &Request<Body>) -> Result<Response<Body>> {
        // TODO: Implement DELETE handling
        
        // For now, just return 405 Method Not Allowed
        let mut response = Response::new(Body::from("Method Not Allowed"));
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        response.headers_mut().insert(
            header::ALLOW,
            header::HeaderValue::from_static("GET, HEAD, OPTIONS"),
        );
        
        Ok(response)
    }
    
    // Handle OPTIONS requests
    async fn handle_options(&self, req: &Request<Body>, vhost: &VirtualHost) -> Result<Response<Body>> {
        let mut response = Response::new(Body::empty());
        
        // Set Allow header
        let allow = vhost.allow_methods.join(", ");
        response.headers_mut().insert(
            header::ALLOW,
            header::HeaderValue::from_str(&allow)
                .unwrap_or_else(|_| header::HeaderValue::from_static("GET, HEAD, OPTIONS")),
        );
        
        // Set CORS headers if needed
        if let Some(origin) = req.headers().get(header::ORIGIN) {
            // Check if origin is allowed
            let origin_str = origin.to_str().unwrap_or("");
            if security::headers::is_origin_allowed(origin_str, &["*".to_string()]) {
                // Allow the origin
                response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
                
                // Allow credentials
                response.headers_mut().insert(
                    header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                    header::HeaderValue::from_static("true"),
                );
                
                // Allow methods
                response.headers_mut().insert(
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    header::HeaderValue::from_str(&allow)
                        .unwrap_or_else(|_| header::HeaderValue::from_static("GET, HEAD, OPTIONS")),
                );
                
                // Allow headers
                if let Some(request_headers) = req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
                    response.headers_mut().insert(
                        header::ACCESS_CONTROL_ALLOW_HEADERS,
                        request_headers.clone(),
                    );
                }
                
                // Max age
                response.headers_mut().insert(
                    header::ACCESS_CONTROL_MAX_AGE,
                    header::HeaderValue::from_static("86400"),
                );
            }
        }
        
        Ok(response)
    }
} 