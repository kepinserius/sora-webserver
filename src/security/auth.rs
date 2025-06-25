use std::collections::HashMap;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hyper::header::{HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE};
use hyper::{Body, Request, Response, StatusCode};
use anyhow::Result;
use ring::digest;
use ring::pbkdf2;

use crate::config::BasicAuthConfig;

const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
const PBKDF2_ITERATIONS: u32 = 100_000;

/// Verifies HTTP Basic Authentication credentials
pub fn verify_basic_auth(req: &Request<Body>, auth_config: &BasicAuthConfig) -> bool {
    // Get authorization header
    if let Some(auth) = req.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            // Check if it's basic auth
            if auth_str.starts_with("Basic ") {
                let credentials = &auth_str["Basic ".len()..];
                if let Ok(decoded) = BASE64.decode(credentials) {
                    if let Ok(auth_string) = String::from_utf8(decoded) {
                        // Split into username and password
                        if let Some(separator_pos) = auth_string.find(':') {
                            let username = &auth_string[..separator_pos];
                            let password = &auth_string[separator_pos + 1..];
                            
                            // Check if user exists
                            if let Some(stored_password_hash) = auth_config.users.get(username) {
                                // Verify password
                                return verify_password(password, stored_password_hash);
                            }
                        }
                    }
                }
            }
        }
    }
    
    false
}

/// Creates a response that requests basic authentication from client
pub fn request_basic_auth(realm: &str) -> Response<Body> {
    let mut response = Response::new(Body::from("Authentication required"));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    
    let auth_header = format!("Basic realm=\"{}\"", realm);
    response.headers_mut().insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_str(&auth_header).unwrap_or_else(|_| {
            HeaderValue::from_static("Basic realm=\"restricted\"")
        }),
    );
    
    response
}

/// Creates a new password hash with PBKDF2 and SHA-512
pub fn hash_password(password: &str) -> String {
    let salt = generate_salt();
    let mut hash = [0u8; CREDENTIAL_LEN];
    
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA512,
        std::num::NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
        &salt,
        password.as_bytes(),
        &mut hash,
    );
    
    // Encode as base64 and include salt
    let salt_b64 = BASE64.encode(&salt);
    let hash_b64 = BASE64.encode(&hash);
    
    format!("$pbkdf2-sha512${}${}${}", PBKDF2_ITERATIONS, salt_b64, hash_b64)
}

/// Verifies a password against a stored hash
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    // Parse the stored hash
    let parts: Vec<&str> = stored_hash.split('$').collect();
    if parts.len() != 5 || parts[1] != "pbkdf2-sha512" {
        return false;
    }
    
    // Extract iterations, salt, and hash
    let iterations = match parts[2].parse::<u32>() {
        Ok(i) => i,
        Err(_) => return false,
    };
    
    let salt = match BASE64.decode(parts[3]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    let hash = match BASE64.decode(parts[4]) {
        Ok(h) => h,
        Err(_) => return false,
    };
    
    // Verify the password
    pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA512,
        std::num::NonZeroU32::new(iterations).unwrap(),
        &salt,
        password.as_bytes(),
        &hash,
    )
    .is_ok()
}

/// Generate a random salt for password hashing
fn generate_salt() -> Vec<u8> {
    use rand::{thread_rng, Rng};
    
    let mut salt = vec![0u8; 16];
    thread_rng().fill(&mut salt[..]);
    salt
}

/// Load users from htpasswd-like file
pub async fn load_users_from_file(path: &str) -> Result<HashMap<String, String>> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut users = HashMap::new();
    
    for line in content.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        
        if let Some(separator_pos) = line.find(':') {
            let username = line[..separator_pos].to_string();
            let password_hash = line[separator_pos + 1..].to_string();
            users.insert(username, password_hash);
        }
    }
    
    Ok(users)
}

/// Save users to htpasswd-like file
pub async fn save_users_to_file(path: &str, users: &HashMap<String, String>) -> Result<()> {
    let mut content = String::new();
    
    for (username, password_hash) in users {
        content.push_str(&format!("{}:{}\n", username, password_hash));
    }
    
    tokio::fs::write(path, content).await?;
    Ok(())
} 