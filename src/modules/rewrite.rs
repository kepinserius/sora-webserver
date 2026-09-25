
use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response, Uri};
use regex::Regex;
use serde_json::Value;
use tracing::debug;

use crate::modules::Module;

// Module for URL rewriting (similar to Apache/Nginx mod_rewrite)
#[derive(Debug)]
pub struct RewriteModule {
    // Rewrite rules (pattern -> replacement)
    rules: Vec<RewriteRule>,
}

// Rewrite rule with regex pattern and replacement
#[derive(Debug)]
struct RewriteRule {
    // Regex pattern to match the URL
    pattern: Regex,
    // Replacement string (can include captures)
    replacement: String,
    // Flags for the rule
    flags: RewriteFlags,
}

// Flags for rewrite rules
#[derive(Debug, Default)]
struct RewriteFlags {
    // Last rule - stop processing
    last: bool,
    // Redirect - send 301/302 redirect instead of internal rewrite
    redirect: bool,
    // Permanent redirect (301 instead of 302)
    permanent: bool,
    // Apply to query string as well
    query_string: bool,
}

impl Default for RewriteModule {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
        }
    }
}

impl RewriteModule {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Add a rewrite rule
    pub fn add_rule(&mut self, pattern: &str, replacement: &str, flags: Option<Vec<&str>>) -> Result<()> {
        let regex = Regex::new(pattern)?;
        
        let mut rule_flags = RewriteFlags::default();
        
        // Process flags
        if let Some(flag_list) = flags {
            for flag in flag_list {
                match flag {
                    "L" | "last" => rule_flags.last = true,
                    "R" | "redirect" => rule_flags.redirect = true,
                    "P" | "permanent" => {
                        rule_flags.redirect = true;
                        rule_flags.permanent = true;
                    },
                    "QSA" | "qsappend" => rule_flags.query_string = true,
                    _ => {} // Ignore unknown flags
                }
            }
        }
        
        self.rules.push(RewriteRule {
            pattern: regex,
            replacement: replacement.to_string(),
            flags: rule_flags,
        });
        
        Ok(())
    }
    
    // Process a request through the rewrite rules
    fn process_request(&self, req: &mut Request<Body>) -> Option<Response<Body>> {
        let path = req.uri().path().to_string();
        let query = req.uri().query().map(|q| q.to_string());
        
        for rule in &self.rules {
            if let Some(_captures) = rule.pattern.captures(&path) {
                let mut new_path = rule.pattern.replace(&path, &rule.replacement).to_string();
                
                // Append query string if needed
                let new_query = if rule.flags.query_string {
                    query.clone()
                } else {
                    // Extract query string from replacement if present
                    if let Some(query_pos) = new_path.find('?') {
                        let query_part = new_path[query_pos + 1..].to_string();
                        new_path = new_path[..query_pos].to_string();
                        Some(query_part)
                    } else {
                        None
                    }
                };
                
                // Build the new URI
                let mut uri_parts = req.uri().clone().into_parts();
                
                // Update path and query
                let path_and_query = match new_query {
                    Some(q) => format!("{}?{}", new_path, q),
                    None => new_path,
                };
                
                uri_parts.path_and_query = Some(path_and_query.parse().ok()?);
                
                // Create the new URI
                if let Ok(new_uri) = Uri::from_parts(uri_parts) {
                    debug!("Rewriting {} to {}", req.uri(), new_uri);
                    
                    // Handle redirects
                    if rule.flags.redirect {
                        let mut response = Response::new(Body::empty());
                        
                        // Set status code (301 for permanent, 302 for temporary)
                        if rule.flags.permanent {
                            *response.status_mut() = hyper::StatusCode::MOVED_PERMANENTLY;
                        } else {
                            *response.status_mut() = hyper::StatusCode::FOUND;
                        }
                        
                        // Add Location header
                        response.headers_mut().insert(
                            hyper::header::LOCATION,
                            hyper::header::HeaderValue::from_str(new_uri.to_string().as_str()).ok()?
                        );
                        
                        return Some(response);
                    } else {
                        // Internal rewrite
                        *req.uri_mut() = new_uri;
                        
                        // If this is the last rule, stop processing
                        if rule.flags.last {
                            break;
                        }
                    }
                }
            }
        }
        
        None
    }
}

#[async_trait]
impl Module for RewriteModule {
    fn name(&self) -> &str {
        "rewrite"
    }
    
    fn init(&mut self, config: &Value) -> Result<()> {
        if let Some(rules_array) = config.get("rules").and_then(Value::as_array) {
            for rule_obj in rules_array {
                if let Some(rule) = rule_obj.as_object() {
                    // Get pattern
                    let pattern = rule.get("pattern")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    
                    // Get replacement
                    let replacement = rule.get("replacement")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    
                    // Get flags
                    let flags = rule.get("flags")
                        .and_then(Value::as_array)
                        .map(|flags_array| {
                            flags_array.iter()
                                .filter_map(|f| f.as_str())
                                .collect::<Vec<_>>()
                        });
                    
                    // Add the rule
                    if !pattern.is_empty() && !replacement.is_empty() {
                        if let Err(e) = self.add_rule(pattern, replacement, flags) {
                            tracing::error!("Failed to add rewrite rule: {}", e);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn pre_process(&self, req: &mut Request<Body>) -> Result<Option<Response<Body>>> {
        // Process rewrite rules
        let response = self.process_request(req);
        Ok(response)
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