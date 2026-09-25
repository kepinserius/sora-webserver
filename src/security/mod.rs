pub mod tls;
pub mod auth;
pub mod headers;
pub mod rate_limiter;
pub mod firewall;

use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{Body, Request};
use tokio_rustls::server::TlsStream;
use tokio::net::TcpStream;

use crate::handlers::request::RequestHandler;
use anyhow::Result;

pub async fn process_tls_connection(
    tls_stream: TlsStream<TcpStream>, 
    handler: Arc<RequestHandler>, 
    remote_addr: SocketAddr
) -> Result<()> {
    // Convert TlsStream to something hyper can use
    let io = hyper::server::conn::Http::new()
        .http2_only(false)
        .serve_connection(
            tls_stream,
            hyper::service::service_fn(move |req: Request<Body>| {
                let handler = handler.clone();
                async move {
                    // Add client IP to request extensions
                    let mut req_with_ip = req;
                    req_with_ip.extensions_mut().insert(remote_addr);
                    
                    // Pass to request handler
                    handler.handle(req_with_ip).await
                }
            }),
        );

    // Wait for the connection to complete
    io.await?;
    
    Ok(())
}

pub fn is_path_traversal(path: &str) -> bool {
    path.contains("../") || path.contains("..\\")
}

pub fn sanitize_path(path: &str) -> String {
    let mut result = path.to_string();
    
    // Remove any consecutive slashes
    while result.contains("//") {
        result = result.replace("//", "/");
    }
    
    // Remove trailing slash if present (except for root)
    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }
    
    result
} 