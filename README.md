# Sora-webserver

A high-performance, modular web server built in Rust with features similar to Apache/Nginx.

## ✨ Features

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

## 🚀 Quick Start

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

## 📖 Usage Examples

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

## 🔧 Configuration Reference

See the `config/server.toml` file for a complete reference of configuration options:

- HTTP/HTTPS settings
- Virtual hosts
- Security policies
- Caching directives
- Compression settings
- Module configuration

## 📝 API Documentation

For API details and integration options, see the API documentation at `/docs` when the server is running.

---

# Sora-webserver (Bahasa Indonesia)

Server web modular berkinerja tinggi yang dibangun dengan Rust dengan fitur seperti Apache/Nginx.

## ✨ Fitur

- **Performa Tinggi**: Dibangun dengan async Rust menggunakan Tokio dan Hyper
- **Dukungan HTTPS/TLS**: Koneksi aman dengan TLS 1.3
- **Virtual Host**: Hosting banyak situs dalam satu server
- **Fitur Keamanan**:
  - Header Keamanan HTTP
  - Proteksi CSRF
  - Firewall berbasis IP
  - Pembatasan Rate
  - Autentikasi
- **Optimasi Performa**:
  - Kompresi Gzip
  - Caching Konten
  - Dukungan ETag
- **Penulisan Ulang URL**: Konfigurasi path URL dan redirect
- **Reverse Proxy**: Meneruskan permintaan ke layanan backend
- **Penyajian File Statis**: Efisien menyajikan website statis dan aset

## 🚀 Mulai Cepat

### Instalasi

1. Clone repositori
```bash
git clone https://github.com/kepinserius/sora-webserver.git
cd sora-webserver
```

2. Build proyek
```bash
cargo build --release
```

3. Jalankan server
```bash
./target/release/webserver
```

### Konfigurasi

Server menggunakan file konfigurasi TOML yang terletak di direktori `config`:

- `server.toml` - Konfigurasi server utama
- Buat konfigurasi virtual host di direktori `config`

### Struktur Direktori

- `www/` - Direktori root untuk website statis
- `logs/` - Log server
- `certs/` - Sertifikat TLS
- `config/` - File konfigurasi

## 📖 Contoh Penggunaan

### Hosting Website Statis

Letakkan HTML, CSS, dan aset lainnya di direktori `www` atau direktori virtual host:

```
www/
├── index.html
├── css/
├── js/
└── images/
```

### Server API

Konfigurasi sebagai reverse proxy untuk meneruskan permintaan ke layanan API Anda:

```toml
# Di config/server.toml
[[proxy]]
path = "/api"
target = "http://localhost:3000"
```

### Multiple Website (Virtual Host)

```toml
# Di config/example.com.toml
[vhost]
hostname = "example.com"
root = "www/example.com"
```

## 🔧 Referensi Konfigurasi

Lihat file `config/server.toml` untuk referensi lengkap opsi konfigurasi:

- Pengaturan HTTP/HTTPS
- Virtual host
- Kebijakan keamanan
- Arahan caching
- Pengaturan kompresi
- Konfigurasi modul

## 📝 Dokumentasi API

Untuk detail API dan opsi integrasi, lihat dokumentasi API di `/docs` saat server berjalan.