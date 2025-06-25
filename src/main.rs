use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use http::{HeaderValue, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

mod config;
mod security;
mod handlers;
mod vhost;
mod modules;
mod logging;

use crate::config::ServerConfig;
use crate::security::tls::TlsConfig;
use crate::handlers::request::RequestHandler;
use crate::vhost::VirtualHost;

#[derive(Parser, Debug)]
#[command(version, about = "Secure web server with features similar to Apache/Nginx")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config/server.toml")]
    config: PathBuf,

    /// Document root directory
    #[arg(short, long, default_value = "www")]
    docroot: PathBuf,

    /// Address to bind to
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    address: String,

    /// Enable HTTPS
    #[arg(short, long)]
    secure: bool,

    /// Path to TLS certificate
    #[arg(long, default_value = "certs/cert.pem")]
    cert: PathBuf,

    /// Path to TLS key
    #[arg(long, default_value = "certs/key.pem")]
    key: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    logging::init_logger()?;

    // Parse command-line arguments
    let args = Args::parse();

    // Load configuration
    let config = match config::load_config(&args.config).await {
        Ok(cfg) => {
            info!("Configuration loaded successfully from {:?}", args.config);
            cfg
        }
        Err(e) => {
            warn!("Failed to load config from {:?}: {}", args.config, e);
            warn!("Using default configuration");
            ServerConfig::default()
        }
    };

    // Set up document root
    let doc_root = if args.docroot.exists() {
        args.docroot.clone()
    } else {
        warn!("Document root {:?} does not exist, creating it", args.docroot);
        std::fs::create_dir_all(&args.docroot)?;
        args.docroot.clone()
    };

    info!("Document root set to {:?}", doc_root);

    // Parse address
    let addr: SocketAddr = args.address.parse()
        .context("Failed to parse address")?;

    // Set up virtual hosts
    let vhosts = vhost::load_virtual_hosts(&config).await?;
    info!("Loaded {} virtual hosts", vhosts.len());

    // Set up modules
    let modules = modules::load_modules(&config).await?;
    info!("Loaded {} modules", modules.len());

    // Start server depending on HTTPS option
    if args.secure {
        info!("Starting HTTPS server on {}", addr);
        run_https_server(addr, args.cert, args.key, doc_root, vhosts, modules).await?;
    } else {
        info!("Starting HTTP server on {}", addr);
        run_http_server(addr, doc_root, vhosts, modules).await?;
    }

    Ok(())
}

async fn run_http_server(
    addr: SocketAddr,
    doc_root: PathBuf,
    vhosts: Vec<VirtualHost>,
    modules: Vec<Box<dyn modules::Module>>,
) -> Result<()> {
    // Create a request handler
    let handler = RequestHandler::new(doc_root, vhosts, modules);
    let handler = Arc::new(handler);

    let make_svc = make_service_fn(move |_conn| {
        let handler = handler.clone();
        async move {
            Ok::<_, anyhow::Error>(service_fn(move |req| {
                let handler = handler.clone();
                async move { handler.handle(req).await }
            }))
        }
    });

    // Build and start the server
    let server = Server::bind(&addr)
        .tcp_nodelay(true)
        .serve(make_svc);

    info!("Server started successfully on http://{}", addr);
    server.await.context("Server error")?;
    
    Ok(())
}

async fn run_https_server(
    addr: SocketAddr,
    cert_path: PathBuf,
    key_path: PathBuf,
    doc_root: PathBuf,
    vhosts: Vec<VirtualHost>,
    modules: Vec<Box<dyn modules::Module>>,
) -> Result<()> {
    // Set up TLS configuration
    let tls_config = TlsConfig::new(cert_path, key_path).await?;
    let tls_acceptor = tls_config.acceptor()?;

    // Create a TCP listener
    let tcp_listener = TcpListener::bind(&addr).await?;
    info!("Listening on https://{}", addr);

    // Create request handler
    let handler = RequestHandler::new(doc_root, vhosts, modules);
    let handler = Arc::new(handler);

    loop {
        let (tcp_stream, remote_addr) = tcp_listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let handler = handler.clone();

        // Spawn a new task for each connection
        tokio::spawn(async move {
            match tls_acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => {
                    debug!("TLS connection established from {}", remote_addr);
                    
                    // Process the connection with hyper
                    if let Err(e) = security::process_tls_connection(tls_stream, handler, remote_addr).await {
                        error!("Error processing TLS connection: {}", e);
                    }
                }
                Err(e) => {
                    error!("TLS handshake with {} failed: {}", remote_addr, e);
                }
            }
        });
    }
} 