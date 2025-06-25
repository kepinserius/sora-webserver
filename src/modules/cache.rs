use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Method, Request, Response, StatusCode, header};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::modules::Module;

#[derive(Debug)]
pub struct CacheModule {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    default_ttl: u64,
    max_size: usize,
    cacheable_methods: Vec<Method>,
    cacheable_status: Vec<StatusCode>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    headers: HashMap<String, String>,
    body: Vec<u8>,
    created: Instant,
    ttl: u64,
    etag: Option<String>,
    last_modified: Option<String>,
}

impl Default for CacheModule {
    fn default() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: 60,
            max_size: 1000,
            cacheable_methods: vec![Method::GET, Method::HEAD],
            cacheable_status: vec![StatusCode::OK, StatusCode::NOT_MODIFIED],
        }
    }
}

impl CacheModule {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Check if a request is cacheable
    fn is_cacheable_request(&self, req: &Request<Body>) -> bool {
        self.cacheable_methods.contains(req.method())
    }
    
    // Check if a response is cacheable
    fn is_cacheable_response(&self, res: &Response<Body>) -> bool {
        self.cacheable_status.contains(res.status())
    }
    
    // Generate a cache key for a request
    fn cache_key(&self, req: &Request<Body>) -> String {
        format!("{} {}", req.method(), req.uri())
    }
    
    // Try to get a cached response
    async fn get_cached(&self, req: &Request<Body>) -> Option<Response<Body>> {
        let key = self.cache_key(req);
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(&key) {
            // Check if entry has expired
            if entry.created.elapsed().as_secs() > entry.ttl {
                return None;
            }
            
            // Build response from cache
            let mut resp = Response::new(Body::from(entry.body.clone()));
            
            // Add headers
            for (name, value) in &entry.headers {
                if let Ok(header_name) = http::header::HeaderName::from_bytes(name.as_bytes()) {
                    if let Ok(header_value) = http::header::HeaderValue::from_str(value) {
                        resp.headers_mut().insert(header_name, header_value);
                    }
                }
            }
            
            debug!("Cache hit for {}", key);
            Some(resp)
        } else {
            None
        }
    }
    
    // Cache a response
    async fn cache_response(&self, req: &Request<Body>, res: &Response<Body>, body: Vec<u8>) -> Result<()> {
        if !self.is_cacheable_request(req) || !self.is_cacheable_response(res) {
            return Ok(());
        }
        
        let key = self.cache_key(req);
        
        // Extract headers to cache
        let mut headers = HashMap::new();
        for (name, value) in res.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.as_str().to_string(), value_str.to_string());
            }
        }
        
        // Get ETag and Last-Modified headers if present
        let etag = res.headers()
            .get(header::ETAG)
            .and_then(|h| h.to_str().ok())
            .map(String::from);
        
        let last_modified = res.headers()
            .get(header::LAST_MODIFIED)
            .and_then(|h| h.to_str().ok())
            .map(String::from);
        
        // Create cache entry
        let entry = CacheEntry {
            headers,
            body,
            created: Instant::now(),
            ttl: self.default_ttl,
            etag,
            last_modified,
        };
        
        // Add to cache
        let mut cache = self.cache.write().await;
        
        // Check if we need to evict entries
        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            // Simple eviction: remove oldest entry
            if let Some((oldest_key, _)) = cache.iter()
                .min_by_key(|(_, entry)| entry.created) {
                let oldest_key = oldest_key.clone();
                cache.remove(&oldest_key);
            }
        }
        
        cache.insert(key, entry);
        
        Ok(())
    }
}

#[async_trait]
impl Module for CacheModule {
    fn name(&self) -> &str {
        "cache"
    }
    
    fn init(&mut self, config: &Value) -> Result<()> {
        if let Some(obj) = config.as_object() {
            if let Some(ttl) = obj.get("default_ttl").and_then(Value::as_u64) {
                self.default_ttl = ttl;
            }
            
            if let Some(max_size) = obj.get("max_size").and_then(Value::as_u64) {
                self.max_size = max_size as usize;
            }
        }
        
        // Start periodic cleanup
        let cache = self.cache.clone();
        let default_ttl = self.default_ttl;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut cache = cache.write().await;
                
                // Remove expired entries
                cache.retain(|_, entry| {
                    entry.created.elapsed().as_secs() <= default_ttl
                });
            }
        });
        
        info!("Cache module initialized");
        
        Ok(())
    }
    
    async fn pre_process(&self, req: &mut Request<Body>) -> Result<Option<Response<Body>>> {
        if self.is_cacheable_request(req) {
            if let Some(cached_response) = self.get_cached(req).await {
                return Ok(Some(cached_response));
            }
        }
        
        Ok(None)
    }
    
    async fn post_process(&self, res: &mut Response<Body>, req: &Request<Body>) -> Result<()> {
        if self.is_cacheable_request(req) && self.is_cacheable_response(res) {
            // Extract the body
            let body = std::mem::replace(res.body_mut(), Body::empty());
            // Convert to bytes
            let bytes = hyper::body::to_bytes(body).await?;
            // Cache the response
            self.cache_response(req, res, bytes.to_vec()).await?;
            // Replace the body
            *res.body_mut() = Body::from(bytes);
        }
        
        Ok(())
    }
    
    fn cleanup(&self) -> Result<()> {
        Ok(())
    }
} 