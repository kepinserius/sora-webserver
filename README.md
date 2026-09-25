# Sora-webserver

A high-performance, modular web server built in Rust with features similar to Apache/Nginx.

## Features

- **High Performance**: Built with async Rust using Tokio and Hyper
- **HTTPS/TLS Support**: Secure connections with TLS 1.3
- **Virtual Hosts**: Host multiple sites on a single server
- **Security Features**:
  - HTTP Security Headers
  - CSRF Protection
  - IP-based Firewall
  - Rate Limiting
  - Authentication
- **Performance Optimizations**:
  - Gzip Compression
  - Content Caching
  - ETags support
- **URL Rewriting**: Configure URL paths and redirects
- **Reverse Proxy**: Forward requests to backend services
- **Static File Serving**: Efficiently serve static websites and assets

## Quick Start

### Installation

1. Clone the repository
```bash
git clone https://github.com/kepinserius/sora-webserver.git
cd sora-webserver
```

2. Build the project
```bash
cargo build --release
```

3. Run the server
```bash
./target/release/webserver
```

### Configuration

The server uses TOML configuration files located in the `config` directory:

- `server.toml` - Main server configuration
- Create virtual host configs in the `config` directory

### Directory Structure

- `www/` - Root directory for static websites
- `logs/` - Server logs
- `certs/` - TLS certificates
- `config/` - Configuration files

## Usage Examples

### Static Website Hosting

Place your HTML, CSS, and other assets in the `www` directory or a virtual host directory:

```
www/
├── index.html
├── css/
├── js/
└── images/
```

### API Server

Configure as a reverse proxy to forward requests to your API services:

```toml
# In config/server.toml
[[proxy]]
path = "/api"
target = "http://localhost:3000"
```

### Multiple Websites (Virtual Hosts)

```toml
# In config/example.com.toml
[vhost]
hostname = "example.com"
root = "www/example.com"
```

## Configuration Reference

See the `config/server.toml` file for a complete reference of configuration options:

- HTTP/HTTPS settings
- Virtual hosts
- Security policies
- Caching directives
- Compression settings
- Module configuration

