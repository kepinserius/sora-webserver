use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use hyper::{Body, Request, Response, StatusCode, Method, header};
use mime_guess::from_path;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, error};

use crate::modules::Module;
use crate::security;
use crate::vhost::{VirtualHost, Location};

// Module for serving static files
#[derive(Debug)]
pub struct StaticFilesModule {
    // Default cache control header for static files
    cache_control: String,
    // Default index files to try when directory is requested
    index_files: Vec<String>,
    // Whether to allow directory listing
    allow_directory_listing: bool,
}

impl Default for StaticFilesModule {
    fn default() -> Self {
        Self {
            cache_control: "public, max-age=3600".to_string(),
            index_files: vec!["index.html".to_string(), "index.htm".to_string()],
            allow_directory_listing: false,
        }
    }
}

impl StaticFilesModule {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Set the cache control header value
    pub fn with_cache_control(mut self, cache_control: &str) -> Self {
        self.cache_control = cache_control.to_string();
        self
    }
    
    // Set the index files
    pub fn with_index_files(mut self, index_files: Vec<String>) -> Self {
        self.index_files = index_files;
        self
    }
    
    // Enable/disable directory listing
    pub fn with_directory_listing(mut self, allow_listing: bool) -> Self {
        self.allow_directory_listing = allow_listing;
        self
    }
    
    // Get the appropriate document root based on vhost and location
    fn get_document_root(
        &self,
        vhost: &VirtualHost,
        location: Option<&Location>,
        path: &str,
    ) -> PathBuf {
        // If location has a root, use that
        if let Some(loc) = location {
            if let Some(root) = &loc.root {
                return root.clone();
            }
        }
        
        // Otherwise use the vhost's document root
        vhost.document_root.clone()
    }
    
    // Resolve the file path from request path
    async fn resolve_file_path(
        &self,
        req_path: &str,
        vhost: &VirtualHost,
        location: Option<&Location>,
    ) -> Result<Option<PathBuf>> {
        // Sanitize path to prevent path traversal attacks
        let sanitized_path = security::sanitize_path(req_path);
        
        // Get the document root
        let doc_root = self.get_document_root(vhost, location, &sanitized_path);
        
        // Join the document root and path
        let mut file_path = doc_root.join(&sanitized_path[1..]); // remove leading /
        
        // Check if path is a directory
        if file_path.is_dir() {
            // Try to find an index file
            for index in &self.index_files {
                let index_path = file_path.join(index);
                if index_path.exists() {
                    file_path = index_path;
                    break;
                }
            }
            
            // If still a directory and listing is not allowed, return None
            if file_path.is_dir() && !self.allow_directory_listing {
                return Ok(None);
            }
        }
        
        // Check if file exists
        if !file_path.exists() {
            return Ok(None);
        }
        
        Ok(Some(file_path))
    }
    
    // Serve a file
    async fn serve_file(&self, file_path: &Path) -> Result<Response<Body>> {
        // Open and read the file
        let mut file = File::open(file_path).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;
        
        // Create response
        let mut response = Response::new(Body::from(contents));
        
        // Set content type
        let content_type = from_path(file_path)
            .first_or_octet_stream()
            .as_ref()
            .to_owned();
        
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_str(&content_type)?
        );
        
        // Set cache control
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_str(&self.cache_control)?
        );
        
        Ok(response)
    }
    
    // Generate directory listing
    async fn generate_directory_listing(&self, dir_path: &Path, req_path: &str) -> Result<Response<Body>> {
        let entries = tokio::fs::read_dir(dir_path).await?;
        let mut listing = String::new();
        
        // Build HTML for directory listing
        listing.push_str("<!DOCTYPE html>\n");
        listing.push_str("<html>\n<head>\n");
        listing.push_str(&format!("<title>Directory listing for {}</title>\n", req_path));
        listing.push_str("<style>\n");
        listing.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        listing.push_str("h1 { border-bottom: 1px solid #ccc; padding-bottom: 10px; }\n");
        listing.push_str("table { border-collapse: collapse; width: 100%; }\n");
        listing.push_str("th, td { text-align: left; padding: 8px; border-bottom: 1px solid #ddd; }\n");
        listing.push_str("tr:hover { background-color: #f5f5f5; }\n");
        listing.push_str("a { text-decoration: none; color: #0366d6; }\n");
        listing.push_str("</style>\n");
        listing.push_str("</head>\n<body>\n");
        listing.push_str(&format!("<h1>Directory listing for {}</h1>\n", req_path));
        
        listing.push_str("<table>\n");
        listing.push_str("<tr><th>Name</th><th>Size</th><th>Last Modified</th></tr>\n");
        
        // Add parent directory link if not at root
        if req_path != "/" {
            listing.push_str("<tr><td><a href=\"../\">../</a></td><td>-</td><td>-</td></tr>\n");
        }
        
        // Collect all entries
        let mut entries_vec = Vec::new();
        let mut read_dir = entries;
        while let Some(entry) = read_dir.next_entry().await? {
            entries_vec.push(entry);
        }
        
        // Sort entries (directories first, then files)
        entries_vec.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });
        
        // Add entries to the listing
        for entry in entries_vec {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().await?;
            let size = if metadata.is_dir() { "-" } else { &format!("{} KB", metadata.len() / 1024) };
            let modified = metadata.modified()?;
            let modified_time = chrono::DateTime::<chrono::Local>::from(modified)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            
            let link_name = if metadata.is_dir() {
                format!("{}/", file_name)
            } else {
                file_name.clone()
            };
            
            listing.push_str(&format!(
                "<tr><td><a href=\"{}\">{}</a></td><td>{}</td><td>{}</td></tr>\n",
                link_name, link_name, size, modified_time
            ));
        }
        
        listing.push_str("</table>\n");
        listing.push_str("</body>\n</html>");
        
        // Create response
        let mut response = Response::new(Body::from(listing));
        
        // Set content type
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/html; charset=utf-8")
        );
        
        Ok(response)
    }
}

#[async_trait]
impl Module for StaticFilesModule {
    fn name(&self) -> &str {
        "static_files"
    }
    
    fn init(&mut self, config: &Value) -> Result<()> {
        if let Some(obj) = config.as_object() {
            if let Some(cache_control) = obj.get("cache_control").and_then(Value::as_str) {
                self.cache_control = cache_control.to_string();
            }
            
            if let Some(allow_listing) = obj.get("allow_directory_listing").and_then(Value::as_bool) {
                self.allow_directory_listing = allow_listing;
            }
            
            if let Some(index_files) = obj.get("index_files").and_then(Value::as_array) {
                self.index_files = index_files.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
        
        Ok(())
    }
    
    async fn pre_process(&self, _req: &mut Request<Body>) -> Result<Option<Response<Body>>> {
        // This module doesn't do pre-processing
        Ok(None)
    }
    
    async fn post_process(&self, _res: &mut Response<Body>, _req: &Request<Body>) -> Result<()> {
        // This module doesn't do post-processing
        Ok(())
    }
    
    fn cleanup(&self) -> Result<()> {
        // No cleanup needed
        Ok(())
    }
} 