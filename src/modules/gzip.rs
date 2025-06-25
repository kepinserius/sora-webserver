use std::collections::HashSet;
use std::io::Write;

use anyhow::Result;
use async_trait::async_trait;
use flate2::{Compression, write::GzEncoder};
use hyper::{Body, Request, Response, header};
use serde_json::Value;
use tracing::debug;

use crate::modules::Module;

// Module for compressing responses with gzip
#[derive(Debug)]
pub struct GzipModule {
    // Compression level (0-9)
    level: u32,
    // Minimum size to compress (in bytes)
    min_size: usize,
    // Content types to compress
    compress_types: HashSet<String>,
}

impl Default for GzipModule {
    fn default() -> Self {
        let mut compress_types = HashSet::new();
        // Default content types to compress
        compress_types.insert("text/html".to_string());
        compress_types.insert("text/css".to_string());
        compress_types.insert("text/plain".to_string());
        compress_types.insert("text/javascript".to_string());
        compress_types.insert("application/javascript".to_string());
        compress_types.insert("application/json".to_string());
        compress_types.insert("application/xml".to_string());
        compress_types.insert("image/svg+xml".to_string());
        
        Self {
            level: 6, // Default compression level
            min_size: 1024, // 1KB minimum size
            compress_types,
        }
    }
}

impl GzipModule {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Set the compression level
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level.min(9);
        self
    }
    
    // Set the minimum size for compression
    pub fn with_min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }
    
    // Add content types to compress
    pub fn with_content_types(mut self, types: Vec<String>) -> Self {
        for ctype in types {
            self.compress_types.insert(ctype);
        }
        self
    }
    
    // Check if the response should be compressed
    fn should_compress(&self, req: &Request<Body>, res: &Response<Body>) -> bool {
        // Check if client accepts gzip encoding
        let accepts_gzip = req.headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|h| h.to_str().ok())
            .map(|h| h.contains("gzip"))
            .unwrap_or(false);
        
        if !accepts_gzip {
            return false;
        }
        
        // Don't compress already encoded responses
        if res.headers().contains_key(header::CONTENT_ENCODING) {
            return false;
        }
        
        // Check content type
        let should_compress_type = res.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(|ct| {
                // Get the base content type (without parameters)
                let base_type = ct.split(';').next().unwrap_or("").trim();
                self.compress_types.iter().any(|t| base_type.starts_with(t))
            })
            .unwrap_or(false);
        
        if !should_compress_type {
            return false;
        }
        
        // Check content length if available
        if let Some(len) = res.headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok()) {
            
            return len >= self.min_size;
        }
        
        // If content length is not available, assume it's large enough
        true
    }
    
    // Compress the response body
    async fn compress_body(&self, body: Body) -> Result<(Vec<u8>, usize)> {
        // Convert the body to bytes
        let bytes = hyper::body::to_bytes(body).await?;
        let original_size = bytes.len();
        
        // Don't compress if too small
        if original_size < self.min_size {
            return Ok((bytes.to_vec(), original_size));
        }
        
        // Compress the data with gzip
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        encoder.write_all(&bytes)?;
        let compressed_data = encoder.finish()?;
        
        debug!(
            "Compressed response: {} bytes -> {} bytes ({:.1}%)", 
            original_size, 
            compressed_data.len(),
            (1.0 - (compressed_data.len() as f64 / original_size as f64)) * 100.0
        );
        
        Ok((compressed_data, original_size))
    }
}

#[async_trait]
impl Module for GzipModule {
    fn name(&self) -> &str {
        "gzip"
    }
    
    fn init(&mut self, config: &Value) -> Result<()> {
        if let Some(obj) = config.as_object() {
            if let Some(level) = obj.get("level").and_then(Value::as_u64) {
                self.level = level.min(9) as u32;
            }
            
            if let Some(min_size) = obj.get("min_size").and_then(Value::as_u64) {
                self.min_size = min_size as usize;
            }
            
            if let Some(types) = obj.get("compress_types").and_then(Value::as_array) {
                let content_types = types.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                
                if !content_types.is_empty() {
                    self.compress_types.clear();
                    for ctype in content_types {
                        self.compress_types.insert(ctype);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn pre_process(&self, _req: &mut Request<Body>) -> Result<Option<Response<Body>>> {
        // This module doesn't do pre-processing
        Ok(None)
    }
    
    async fn post_process(&self, res: &mut Response<Body>, req: &Request<Body>) -> Result<()> {
        // Check if we should compress this response
        if !self.should_compress(req, res) {
            return Ok(());
        }
        
        // Extract the body
        let body = std::mem::replace(res.body_mut(), Body::empty());
        
        // Compress the body
        let (compressed_data, original_size) = self.compress_body(body).await?;
        
        // Only use compressed data if it's smaller
        if compressed_data.len() < original_size {
            // Update headers
            res.headers_mut().insert(
                header::CONTENT_ENCODING, 
                header::HeaderValue::from_static("gzip")
            );
            
            res.headers_mut().insert(
                header::CONTENT_LENGTH,
                header::HeaderValue::from(compressed_data.len() as u64)
            );
            
            // Replace the body with compressed data
            *res.body_mut() = Body::from(compressed_data);
        } else {
            // Compression didn't help, use original data
            *res.body_mut() = Body::from(compressed_data);
        }
        
        // Add Vary header to indicate that response varies based on Accept-Encoding
        res.headers_mut().insert(
            header::VARY,
            header::HeaderValue::from_static("Accept-Encoding")
        );
        
        Ok(())
    }
    
    fn cleanup(&self) -> Result<()> {
        // No cleanup needed
        Ok(())
    }
} 