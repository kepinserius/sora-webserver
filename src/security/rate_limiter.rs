use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hyper::{Body, Request, Response, StatusCode};
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

// Simple in-memory rate limiter
#[derive(Debug, Clone)]
pub struct RateLimiter {
    // Maximum number of requests allowed in the time window
    max_requests: usize,
    // Time window in seconds
    window_duration: Duration,
    // Store IP addresses and their request counts
    requests: Arc<RwLock<HashMap<IpAddr, RequestTracker>>>,
}

#[derive(Debug)]
struct RequestTracker {
    // Count of requests in current window
    count: usize,
    // When the current window started
    window_start: Instant,
    // Last request time
    last_seen: Instant,
}

// Global rate limiter instance
static GLOBAL_RATE_LIMITER: Lazy<Mutex<Option<RateLimiter>>> = Lazy::new(|| Mutex::new(None));

impl RateLimiter {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
            requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    // Set up the global rate limiter
    pub fn setup_global(max_requests: usize, window_seconds: u64) {
        let mut global = GLOBAL_RATE_LIMITER.lock().unwrap();
        *global = Some(RateLimiter::new(max_requests, window_seconds));
    }
    
    // Get the global rate limiter
    pub fn global() -> Option<RateLimiter> {
        let global = GLOBAL_RATE_LIMITER.lock().unwrap();
        global.clone()
    }
    
    // Check if a request is allowed and update counters
    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let mut requests = self.requests.write().await;
        
        let now = Instant::now();
        
        // Clean up old entries occasionally
        if requests.len() > 1000 {
            self.cleanup(&mut requests, now);
        }
        
        // Get or create tracker for this IP
        let tracker = requests.entry(ip).or_insert_with(|| RequestTracker {
            count: 0,
            window_start: now,
            last_seen: now,
        });
        
        // Check if we need to reset the window
        if now.duration_since(tracker.window_start) > self.window_duration {
            // Reset the window
            tracker.window_start = now;
            tracker.count = 0;
        }
        
        // Update the tracker
        tracker.last_seen = now;
        tracker.count += 1;
        
        // Check if limit is exceeded
        tracker.count <= self.max_requests
    }
    
    // Remove old entries to avoid memory leaks
    fn cleanup(&self, requests: &mut HashMap<IpAddr, RequestTracker>, now: Instant) {
        // Remove entries that haven't been seen in 3x the window duration
        let cutoff = now - (self.window_duration * 3);
        
        requests.retain(|_, tracker| tracker.last_seen >= cutoff);
    }
    
    // Creates a 429 Too Many Requests response
    pub fn rate_limited_response() -> Response<Body> {
        let mut response = Response::new(Body::from("Rate limit exceeded"));
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        
        // Add Retry-After header
        response.headers_mut().insert(
            hyper::header::RETRY_AFTER,
            hyper::header::HeaderValue::from_static("60"),
        );
        
        response
    }
    
    // Get client IP from request
    pub fn get_client_ip(req: &Request<Body>) -> Option<IpAddr> {
        // Try to get IP from extensions first (set by our TLS handler)
        if let Some(addr) = req.extensions().get::<std::net::SocketAddr>() {
            return Some(addr.ip());
        }
        
        // Try to get from X-Forwarded-For header
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                // Take the first IP in the list
                if let Some(client_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = client_ip.trim().parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
        
        // Try to get from remote_addr if available
        if let Some(remote_addr) = req.extensions().get::<String>() {
            if let Ok(socket_addr) = remote_addr.parse::<std::net::SocketAddr>() {
                return Some(socket_addr.ip());
            }
        }
        
        None
    }
}

// Middleware check function
pub async fn check_rate_limit(req: &Request<Body>) -> Option<Response<Body>> {
    // Get the global rate limiter
    if let Some(limiter) = RateLimiter::global() {
        // Get client IP
        if let Some(ip) = RateLimiter::get_client_ip(req) {
            // Check rate limit
            if !limiter.check_rate_limit(ip).await {
                return Some(RateLimiter::rate_limited_response());
            }
        }
    }
    
    None
} 