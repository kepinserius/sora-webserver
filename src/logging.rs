use std::io::Write;

use anyhow::{Context, Result};
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use hyper::{Body, Request, Response};
use chrono::Local;

// Initialize the logger
pub fn init_logger() -> Result<()> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs").context("Failed to create logs directory")?;
    
    // Set up file appender for access logs
    let _access_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/access.log")
        .context("Failed to open access log file")?;

    // Set up file appender for error logs
    let error_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/error.log")
        .context("Failed to open error log file")?;

    // Set up console logger
    let console_layer = fmt::layer()
        .with_target(true)
        .compact();

    // Set up file loggers
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(error_file));

    // Setup filter based on env var or default to info
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Register the subscriber
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(filter)
        .init();

    info!("Logging initialized");
    
    Ok(())
}

// Log an HTTP request to the access log
pub fn log_request(
    req: &Request<Body>,
    res: &Response<Body>,
    client_ip: &str,
    processing_time: std::time::Duration,
    bytes_sent: usize,
) {
    let now = Local::now();
    let status = res.status().as_u16();
    let method = req.method();
    let uri = req.uri();
    let version = format!("{:?}", req.version());
    let user_agent = req.headers().get(hyper::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("-");
    let referer = req.headers().get(hyper::header::REFERER)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("-");
    
    let log_entry = format!(
        "{} - - [{}] \"{} {} {}\" {} {} {:.3} ms \"{}\" \"{}\"",
        client_ip,
        now.format("%d/%b/%Y:%H:%M:%S %z"),
        method,
        uri,
        version,
        status,
        bytes_sent,
        processing_time.as_secs_f64() * 1000.0,
        referer,
        user_agent
    );
    
    // Log to access log file
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/access.log") {
        
        let _ = writeln!(file, "{}", log_entry);
    }
    
    // Also log to the console if status is an error
    if status >= 400 {
        if status < 500 {
            info!("{}", log_entry);
        } else {
            error!("{}", log_entry);
        }
    }
}

// Log an error
pub fn log_error(err: &anyhow::Error, req: Option<&Request<Body>>, client_ip: Option<&str>) {
    let now = Local::now();
    let request_info = if let Some(req) = req {
        format!("{} {} {:?}", req.method(), req.uri(), req.version())
    } else {
        "No request info".to_string()
    };
    
    let client = client_ip.unwrap_or("unknown");
    
    let log_entry = format!(
        "[{}] [error] [client {}] {}: {}\n{:?}",
        now.format("%a %b %d %H:%M:%S %.3f %Y"),
        client,
        request_info,
        err,
        err
    );
    
    // Log to error log file
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/error.log") {
        
        let _ = writeln!(file, "{}", log_entry);
    }
    
    // Also log to the console
    error!("{}", log_entry);
} 