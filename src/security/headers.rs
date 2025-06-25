use hyper::{Body, Response, header};
use http::header::HeaderName;

use crate::config::SecuritySettings;

/// Add security headers to response based on server settings
pub fn add_security_headers(res: &mut Response<Body>, security: &SecuritySettings, is_https: bool) {
    // Add standard security headers
    
    // X-Content-Type-Options prevents browsers from MIME-sniffing
    res.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    
    // XSS Protection header
    if security.enable_xss_protection {
        res.headers_mut().insert(
            HeaderName::from_static("x-xss-protection"),
            header::HeaderValue::from_static("1; mode=block"),
        );
    }
    
    // X-Frame-Options prevents clickjacking
    if security.enable_clickjacking_protection {
        res.headers_mut().insert(
            header::X_FRAME_OPTIONS,
            header::HeaderValue::from_static("SAMEORIGIN"),
        );
    }
    
    // Content-Security-Policy restricts resource loading
    if security.enable_csp {
        res.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_str(&security.content_security_policy)
                .unwrap_or_else(|_| header::HeaderValue::from_static("default-src 'self';")),
        );
    }
    
    // HTTP Strict Transport Security (HSTS)
    if is_https && security.enable_hsts {
        let hsts_value = format!("max-age={}; includeSubDomains", security.hsts_max_age);
        res.headers_mut().insert(
            header::STRICT_TRANSPORT_SECURITY,
            header::HeaderValue::from_str(&hsts_value).unwrap_or_else(|_| {
                header::HeaderValue::from_static("max-age=31536000; includeSubDomains")
            }),
        );
    }
    
    // Referrer-Policy controls how much referrer information is sent
    res.headers_mut().insert(
        HeaderName::from_static("referrer-policy"),
        header::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    
    // Feature-Policy/Permissions-Policy controls which browser features can be used
    res.headers_mut().insert(
        HeaderName::from_static("permissions-policy"),
        header::HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), interest-cohort=()"
        ),
    );
}

/// Check if request has a valid CSRF token
pub fn verify_csrf_token(req: &hyper::Request<Body>, token: &str) -> bool {
    // Check CSRF token from various places
    let header_token = req.headers().get("X-CSRF-Token")
        .or_else(|| req.headers().get("X-XSRF-Token"));
    
    if let Some(header_value) = header_token {
        if let Ok(header_str) = header_value.to_str() {
            return header_str == token;
        }
    }
    
    // Could also check form data or cookies for the token
    // but would need to parse the request body
    
    false
}

/// Check if the origin is allowed based on CORS configuration
pub fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    if allowed_origins.contains(&"*".to_string()) {
        return true;
    }
    
    allowed_origins.iter().any(|allowed| {
        if allowed.starts_with("*.") {
            // Wildcard subdomain
            let domain = allowed.trim_start_matches("*.");
            origin.ends_with(domain)
        } else {
            origin == allowed
        }
    })
}

/// Generate a random CSRF token
pub fn generate_csrf_token() -> String {
    use rand::{thread_rng, Rng};
    
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const TOKEN_LEN: usize = 32;
    
    let mut rng = thread_rng();
    let token: String = (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
        
    token
} 