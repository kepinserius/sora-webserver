# Getting Started with RustWeb Server

This guide will help you set up, configure, and run RustWeb Server for your web hosting needs.

## Prerequisites

Before you begin, make sure you have the following installed:

- Rust and Cargo (1.60.0 or newer)
- OpenSSL development libraries
- Git (optional, for cloning the repository)

### Installing Rust

If you don't have Rust installed, you can install it using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions to complete the installation.

### Installing OpenSSL development libraries

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install libssl-dev pkg-config
```

#### Fedora/RHEL/CentOS
```bash
sudo dnf install openssl-devel
```

#### macOS
```bash
brew install openssl
```

## Installation

### Option 1: Clone and Build from Source

```bash
# Clone the repository
git clone https://github.com/username/rustweb-server.git
cd rustweb-server

# Build in release mode
cargo build --release
```

The compiled binary will be located at `target/release/webserver`.

### Option 2: Install using Cargo

```bash
cargo install webserver
```

## Directory Structure Setup

RustWeb Server requires a specific directory structure to function correctly:

```bash
mkdir -p www logs certs config
```

These directories serve the following purposes:

- `www`: Root directory for websites and static files
- `logs`: Server logs (access and error logs)
- `certs`: SSL/TLS certificates for HTTPS
- `config`: Configuration files in TOML format

## Creating a Self-Signed Certificate for HTTPS

For development and testing, you can create a self-signed certificate:

```bash
cd certs
openssl req -x509 -newkey rsa:4096 -keyout server.key -out server.crt -days 365 -nodes -subj "/CN=localhost"
```

For production use, obtain a proper certificate from a Certificate Authority such as Let's Encrypt.

## Basic Configuration

Create a basic server configuration file:

```bash
touch config/server.toml
```

Add the following content to `config/server.toml`:

```toml
# Basic server configuration
[server]
http_addr = "0.0.0.0:8080"
enable_https = true
https_addr = "0.0.0.0:8443"
workers = 0  # Auto-detect based on CPU cores
document_root = "www"

# TLS configuration
[tls]
cert_file = "certs/server.crt"
key_file = "certs/server.key"

# Enable compression for better performance
[compression]
enable = true
level = 6
min_size = 1024
```

## Running the Server

### Starting in HTTP mode

```bash
./target/release/webserver
```

### Starting in HTTPS mode

```bash
./target/release/webserver -s
```

### Using a specific configuration file

```bash
./target/release/webserver -c /path/to/config/custom-server.toml
```

### Setting a different document root

```bash
./target/release/webserver -d /path/to/www
```

## Adding Your First Website

1. Create an `index.html` file in the `www` directory:

```html
<!DOCTYPE html>
<html>
<head>
    <title>My Website</title>
</head>
<body>
    <h1>Welcome to My Website</h1>
    <p>This site is served by RustWeb Server!</p>
</body>
</html>
```

2. Access your website at `http://localhost:8080` or `https://localhost:8443`

## Setting Up Virtual Hosts

To host multiple websites on the same server:

1. Create directories for each site:

```bash
mkdir -p www/site1.example.com
mkdir -p www/site2.example.com
```

2. Create an index file for each site:

```bash
echo "<h1>Site 1</h1>" > www/site1.example.com/index.html
echo "<h1>Site 2</h1>" > www/site2.example.com/index.html
```

3. Add virtual host configurations to `config/server.toml`:

```toml
[[vhosts]]
hostname = "site1.example.com"
document_root = "www/site1.example.com"

[[vhosts]]
hostname = "site2.example.com"
document_root = "www/site2.example.com"
```

4. Update your local hosts file for testing:

```
127.0.0.1 site1.example.com
127.0.0.1 site2.example.com
```

## Setting Up a Reverse Proxy

To use RustWeb Server as a reverse proxy for backend services:

```toml
[[proxy]]
path = "/api"
target = "http://localhost:3000"
preserve_host = true
```

This will forward all requests to `/api/*` to a service running on port 3000.

## Next Steps

- Check out the [Configuration Reference](configuration.md) for advanced options
- Learn about [Security Best Practices](security.md) for your web server
- Explore [Performance Tuning](performance.md) for high-traffic sites 