use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response};
use serde_json::Value;

// Module implementation trait
#[async_trait]
pub trait Module: Debug + Send + Sync {
    // Get the module name
    fn name(&self) -> &str;
    
    // Initialize the module with configuration
    fn init(&mut self, config: &Value) -> Result<()>;
    
    // Process a request (called before the main request handler)
    async fn pre_process(&self, req: &mut Request<Body>) -> Result<Option<Response<Body>>>;
    
    // Process a response (called after the main request handler)
    async fn post_process(&self, res: &mut Response<Body>, req: &Request<Body>) -> Result<()>;
    
    // Clean up resources
    fn cleanup(&self) -> Result<()>;
}

// Standard modules
pub mod static_files;
pub mod gzip;
pub mod cache;
pub mod rewrite;
pub mod proxy;
pub mod security;

use crate::config::ServerConfig;

// Load enabled modules from configuration
pub async fn load_modules(config: &ServerConfig) -> Result<Vec<Box<dyn Module>>> {
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    
    // Always include essential modules
    modules.push(Box::new(static_files::StaticFilesModule::new()));
    modules.push(Box::new(gzip::GzipModule::new()));
    
    // Load other modules from configuration
    for module_config in &config.modules {
        if !module_config.enabled {
            continue;
        }
        
        match module_config.name.as_str() {
            "cache" => {
                let mut module = cache::CacheModule::new();
                module.init(&module_config.settings)?;
                modules.push(Box::new(module));
            },
            "rewrite" => {
                let mut module = rewrite::RewriteModule::new();
                module.init(&module_config.settings)?;
                modules.push(Box::new(module));
            },
            "proxy" => {
                let mut module = proxy::ProxyModule::new();
                module.init(&module_config.settings)?;
                modules.push(Box::new(module));
            },
            "security" => {
                let mut module = security::SecurityModule::new();
                module.init(&module_config.settings)?;
                modules.push(Box::new(module));
            },
            _ => {
                tracing::warn!("Unknown module: {}", module_config.name);
            }
        }
    }
    
    Ok(modules)
} 