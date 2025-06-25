# Panduan Membuat Sora-webserver Mudah Digunakan dan Diinstal

Dokumen ini menjelaskan langkah-langkah yang diperlukan untuk mengubah Sora-webserver menjadi webserver yang mudah digunakan dan diinstal seperti Apache atau Nginx.

## 1. Pembuatan Installer Paket

### Paket Debian/Ubuntu (.deb)
- Gunakan cargo-deb untuk membuat paket Debian:
  ```bash
  cargo install cargo-deb
  cargo deb
  ```
- Konfigurasi dalam Cargo.toml:
  ```toml
  [package.metadata.deb]
  maintainer = "Nama Anda <email@anda.com>"
  copyright = "2023, Nama Anda <email@anda.com>"
  license-file = ["LICENSE", "4"]
  depends = "$auto, libssl1.1"
  section = "web"
  priority = "optional"
  assets = [
    ["target/release/webserver", "usr/bin/sora-webserver", "755"],
    ["config/*", "etc/sora-webserver/", "644"],
    ["README.md", "usr/share/doc/sora-webserver/README", "644"],
    ["scripts/sora-webserver.service", "lib/systemd/system/sora-webserver.service", "644"],
  ]
  conf-files = ["/etc/sora-webserver/server.toml"]
  ```

### Paket RedHat/Fedora/CentOS (.rpm)
- Gunakan FPM (Effing Package Management):
  ```bash
  gem install fpm
  fpm -s dir -t rpm -n sora-webserver -v 1.0.0 \
      --rpm-user sora-webserver --rpm-group sora-webserver \
      target/release/webserver=/usr/bin/sora-webserver \
      config/=/etc/sora-webserver/ \
      scripts/sora-webserver.service=/lib/systemd/system/sora-webserver.service
  ```

### Pembuatan Direktori dan Repositori APT/YUM
- Siapkan server untuk hosting repository:
  ```bash
  sudo apt install reprepro # untuk Debian/Ubuntu
  sudo yum install createrepo # untuk RHEL/CentOS
  ```
- Buat file sources.list untuk pengguna:
  ```
  deb https://repo.sora-webserver.com/apt stable main
  ```

## 2. Standarisasi Struktur Direktori

### Struktur Direktori yang Disarankan
```
/etc/sora-webserver/                # Konfigurasi utama
├── server.toml              # Konfigurasi server utama
├── sites-available/         # Konfigurasi virtual host tersedia
│   ├── default.toml
│   └── example.com.toml
├── sites-enabled/           # Symlinks ke sites-available (aktif)
│   └── default.toml -> ../sites-available/default.toml
├── modules-available/       # Modul tersedia
├── modules-enabled/         # Modul yang aktif (symlink)
└── ssl/                     # Sertifikat SSL

/var/www/                    # Dokumen web
├── html/                    # Default document root
└── example.com/             # Virtual host document root

/var/log/sora-webserver/            # Log files
├── access.log
└── error.log

/var/lib/sora-webserver/            # Data runtime
├── cache/                   # File cache
└── sessions/                # Data sesi

/usr/lib/sora-webserver/            # Library pendukung
└── modules/                 # Modul dinamis
```

### Pembuatan Direktori dalam Installer
```bash
#!/bin/bash
mkdir -p /etc/sora-webserver/{sites-available,sites-enabled,modules-available,modules-enabled,ssl}
mkdir -p /var/www/html
mkdir -p /var/log/sora-webserver
mkdir -p /var/lib/sora-webserver/{cache,sessions}
mkdir -p /usr/lib/sora-webserver/modules

# Set izin yang tepat
chown -R root:root /etc/sora-webserver
chmod -R 755 /etc/sora-webserver
chmod 644 /etc/sora-webserver/*.toml

chown -R www-data:www-data /var/www
chmod -R 755 /var/www

chown -R www-data:www-data /var/log/sora-webserver
chmod -R 750 /var/log/sora-webserver

chown -R www-data:www-data /var/lib/sora-webserver
chmod -R 750 /var/lib/sora-webserver
```

## 3. Integrasi dengan Systemd

### File Service Systemd
Buat file `scripts/sora-webserver.service`:
```
[Unit]
Description=Sora-webserver
Documentation=https://sora-webserver.com/docs
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=www-data
Group=www-data
ExecStartPre=/usr/bin/sora-webserver -t
ExecStart=/usr/bin/sora-webserver -c /etc/sora-webserver/server.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s
LimitNOFILE=1048576
LimitNPROC=512
PrivateTmp=true
ProtectSystem=full
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

### Konfigurasi Sistem untuk Auto-start
```bash
#!/bin/bash
# Instal service
cp scripts/sora-webserver.service /lib/systemd/system/
systemctl daemon-reload
systemctl enable sora-webserver
systemctl start sora-webserver
```

## 4. Script Manajemen

### Script Manajemen Virtual Host
Buat script `bin/sora-webserver-site` untuk mengelola virtual host (mirip a2ensite/a2dissite):

```bash
#!/bin/bash
AVAILABLE_DIR="/etc/sora-webserver/sites-available"
ENABLED_DIR="/etc/sora-webserver/sites-enabled"

case "$1" in
  enable)
    if [ -f "$AVAILABLE_DIR/$2.toml" ]; then
      ln -sf "$AVAILABLE_DIR/$2.toml" "$ENABLED_DIR/$2.toml"
      echo "Site $2 enabled. Reload Sora-webserver to apply changes."
    else
      echo "Site configuration $2.toml not found."
      exit 1
    fi
    ;;
  disable)
    if [ -L "$ENABLED_DIR/$2.toml" ]; then
      rm "$ENABLED_DIR/$2.toml"
      echo "Site $2 disabled. Reload Sora-webserver to apply changes."
    else
      echo "Site $2 not enabled."
      exit 1
    fi
    ;;
  list)
    echo "Available sites:"
    ls -1 "$AVAILABLE_DIR" | sed 's/\.toml$//'
    echo ""
    echo "Enabled sites:"
    ls -1 "$ENABLED_DIR" | sed 's/\.toml$//'
    ;;
  *)
    echo "Usage: sora-webserver-site {enable|disable|list} [site]"
    exit 1
    ;;
esac

exit 0
```

### Script Manajemen Modul
Buat script `bin/sora-webserver-module` untuk mengelola modul:

```bash
#!/bin/bash
AVAILABLE_DIR="/etc/sora-webserver/modules-available"
ENABLED_DIR="/etc/sora-webserver/modules-enabled"

case "$1" in
  enable)
    if [ -f "$AVAILABLE_DIR/$2.toml" ]; then
      ln -sf "$AVAILABLE_DIR/$2.toml" "$ENABLED_DIR/$2.toml"
      echo "Module $2 enabled. Reload Sora-webserver to apply changes."
    else
      echo "Module configuration $2.toml not found."
      exit 1
    fi
    ;;
  disable)
    if [ -L "$ENABLED_DIR/$2.toml" ]; then
      rm "$ENABLED_DIR/$2.toml"
      echo "Module $2 disabled. Reload Sora-webserver to apply changes."
    else
      echo "Module $2 not enabled."
      exit 1
    fi
    ;;
  list)
    echo "Available modules:"
    ls -1 "$AVAILABLE_DIR" | sed 's/\.toml$//'
    echo ""
    echo "Enabled modules:"
    ls -1 "$ENABLED_DIR" | sed 's/\.toml$//'
    ;;
  *)
    echo "Usage: sora-webserver-module {enable|disable|list} [module]"
    exit 1
    ;;
esac

exit 0
```

## 5. Antarmuka Konfigurasi Web

### Pengembangan Panel Admin Web
1. Buat aplikasi web untuk manajemen:
   - Framework: React/Vue.js untuk frontend
   - API REST backend dengan Rust atau Node.js
   - Fitur:
     - Dashboard dengan statistik server
     - Manajemen virtual host
     - Konfigurasi SSL/TLS
     - Pengelolaan modul
     - Log viewer

2. Struktur panel admin:
   ```
   /usr/share/sora-webserver/admin-panel/
   ├── index.html
   ├── css/
   ├── js/
   └── api/
   ```

3. Integrasi dengan server:
   ```toml
   # Konfigurasi panel admin di server.toml
   [admin_panel]
   enable = true
   path = "/admin"
   port = 8088
   ssl = true
   auth_type = "basic"  # basic, digest, oauth
   users = [
     { username = "admin", password_hash = "..." }
   ]
   ```

## 6. Dukungan untuk PHP dan Bahasa Server-Side

### Integrasi dengan PHP-FPM
Buat modul PHP untuk integrasi dengan PHP-FPM:

```toml
# /etc/sora-webserver/modules-available/php.toml
[module]
name = "php"
enabled = true

[module.settings]
socket = "/var/run/php/php8.1-fpm.sock"
index = ["index.php", "index.html"]
extensions = ["php"]
timeout = 30
max_children = 20
```

Contoh konfigurasi virtual host untuk WordPress:
```toml
# /etc/sora-webserver/sites-available/wordpress.toml
[vhost]
hostname = "blog.example.com"
document_root = "/var/www/wordpress"
index = ["index.php", "index.html"]

[vhost.modules]
php = true

[vhost.rewrites]
rules = [
  { pattern = "^/wp-admin$", target = "/wp-admin/", type = "redirect", code = 301 },
  { pattern = "^/(.*)\\.php$", target = "/index.php", type = "pass" }
]
```

## 7. Integrasi SSL Otomatis

### Integrasi dengan Let's Encrypt
Buat modul untuk integrasi dengan certbot:

```toml
# /etc/sora-webserver/modules-available/letsencrypt.toml
[module]
name = "letsencrypt"
enabled = true

[module.settings]
email = "admin@example.com"
agree_tos = true
webroot = "/var/www"
auto_renew = true
renew_hook = "systemctl reload sora-webserver"
```

Buat script helper untuk Let's Encrypt:

```bash
#!/bin/bash
# /usr/bin/sora-webserver-ssl

DOMAIN=$1
EMAIL=$2

if [ -z "$DOMAIN" ]; then
  echo "Usage: sora-webserver-ssl domain.com [email@example.com]"
  exit 1
fi

if [ -z "$EMAIL" ]; then
  # Gunakan default dari konfigurasi
  EMAIL=$(grep email /etc/sora-webserver/modules-enabled/letsencrypt.toml | cut -d '"' -f 2)
fi

# Verifikasi domain ada dalam konfigurasi
if ! grep -q "$DOMAIN" /etc/sora-webserver/sites-enabled/*; then
  echo "Error: Domain $DOMAIN tidak ditemukan di konfigurasi virtual host."
  exit 1
fi

# Jalankan certbot
certbot certonly --webroot -w /var/www/$DOMAIN -d $DOMAIN -d www.$DOMAIN --email $EMAIL --agree-tos --non-interactive

# Update konfigurasi virtual host
if [ -f "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" ]; then
  # Temukan file konfigurasi
  CONFIG_FILE=$(grep -l "$DOMAIN" /etc/sora-webserver/sites-available/*)
  
  # Tambahkan konfigurasi SSL
  cat >> $CONFIG_FILE << EOF

[vhost.ssl]
enabled = true
certificate = "/etc/letsencrypt/live/$DOMAIN/fullchain.pem"
private_key = "/etc/letsencrypt/live/$DOMAIN/privkey.pem"
protocols = ["TLSv1.2", "TLSv1.3"]
hsts = true
EOF

  echo "SSL dikonfigurasi untuk $DOMAIN. Reload server untuk menerapkan perubahan."
  systemctl reload sora-webserver
else
  echo "Gagal mendapatkan sertifikat SSL untuk $DOMAIN."
  exit 1
fi
```

## 8. Containerization

### Dockerfile Optimized
Buat Dockerfile yang dioptimalkan untuk produksi:

```Dockerfile
# Build stage
FROM rust:1.70-slim as builder
WORKDIR /usr/src/sora-webserver
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --features production

# Runtime stage
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates libssl1.1 && rm -rf /var/lib/apt/lists/*
RUN groupadd -r sora-webserver && useradd -r -g sora-webserver sora-webserver

COPY --from=builder /usr/src/sora-webserver/target/release/webserver /usr/bin/sora-webserver

# Buat struktur direktori
RUN mkdir -p /etc/sora-webserver /var/www/html /var/log/sora-webserver /var/lib/sora-webserver
COPY config/ /etc/sora-webserver/
COPY www/ /var/www/html/

# Set izin yang sesuai
RUN chown -R sora-webserver:sora-webserver /var/www /var/log/sora-webserver /var/lib/sora-webserver

# Expose port
EXPOSE 80 443

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://localhost/ || exit 1

# Jalankan service
USER sora-webserver
CMD ["/usr/bin/sora-webserver", "-c", "/etc/sora-webserver/server.toml"]
```

### Docker Compose untuk Deployment
Buat file docker-compose.yml yang komprehensif:

```yaml
version: '3.8'

services:
  sora-webserver:
    image: kepinserius/sora-webserver:latest
    container_name: sora-webserver
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./config:/etc/sora-webserver
      - ./www:/var/www
      - ./logs:/var/log/sora-webserver
      - ./data:/var/lib/sora-webserver
      - ./certs:/etc/sora-webserver/ssl
    environment:
      - RUST_LOG=info
      - WORKERS=auto
    networks:
      - web-network
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 1G

  php:
    image: php:8.1-fpm
    container_name: sora-webserver-php
    restart: unless-stopped
    volumes:
      - ./www:/var/www
    networks:
      - web-network

  # Optional: Database untuk aplikasi web
  mariadb:
    image: mariadb:10.6
    container_name: sora-webserver-db
    restart: unless-stopped
    volumes:
      - ./db_data:/var/lib/mysql
    environment:
      - MYSQL_ROOT_PASSWORD=secret
      - MYSQL_DATABASE=wordpress
      - MYSQL_USER=wpuser
      - MYSQL_PASSWORD=wppass
    networks:
      - web-network

networks:
  web-network:
    driver: bridge
```

### Helm Chart untuk Kubernetes
Buat struktur Helm chart untuk Kubernetes deployment.

## 9. CI/CD Pipeline

### GitHub Actions untuk Build dan Test
Buat file `.github/workflows/build.yml`:

```yaml
name: Build and Test

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
          components: rustfmt, clippy
          
      - name: Format check
        uses: actions-rs/cargo@v1
        with:
          command: fmt
          args: --all -- --check
          
      - name: Clippy check
        uses: actions-rs/cargo@v1
        with:
          command: clippy
          args: -- -D warnings
          
      - name: Build
        uses: actions-rs/cargo@v1
        with:
          command: build
          args: --release
          
      - name: Test
        uses: actions-rs/cargo@v1
        with:
          command: test
          
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: sora-webserver
          path: target/release/webserver
```

### Automatic Release dan Deployment
Buat file `.github/workflows/release.yml` untuk rilis otomatis:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

jobs:
  build-and-release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build
        uses: actions-rs/cargo@v1
        with:
          command: build
          args: --release
          
      - name: Create Debian Package
        run: |
          cargo install cargo-deb
          cargo deb
          
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            target/release/webserver
            target/debian/*.deb
          body_path: CHANGELOG.md
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## 10. Branding dan Dokumentasi

### Situs Web Resmi
- Buat situs web dengan informasi tentang produk
- Sertakan:
  - Halaman unduhan
  - Dokumentasi
  - Panduan konfigurasi
  - Contoh penggunaan
  - FAQ
  - Forum komunitas
  - Blog tentang pembaruan dan fitur

### Buku Panduan Komprehensif
- Tulis dokumentasi terperinci untuk pengguna
- Format yang mudah dibaca
- Tersedia dalam beberapa bahasa

### Video Tutorial
- Buat video panduan untuk instalasi dan konfigurasi
- Penjelasan tentang fitur-fitur utama
- Tutorial pemecahan masalah umum

## 11. Perluasan untuk Penggunaan Global

### Internationalization
- Terjemahkan antarmuka ke beberapa bahasa
- Sediakan dokumentasi dalam bahasa yang berbeda
- Dukungan untuk format tanggal dan waktu internasional

### Kepatuhan Regulasi
- Pastikan kepatuhan dengan standar seperti GDPR, CCPA, dll.
- Sediakan panduan untuk konfigurasi yang sesuai regulasi

## 12. Perluasan Ekosistem

### Marketplace untuk Modul dan Tema
- Sediakan platform untuk berbagi dan mengunduh modul
- Publikasikan API untuk pengembang pihak ketiga

### Integrasi dengan Tools Lain
- Sediakan plugin untuk IDE populer
- Buat integrasi dengan alat monitoring
- Dukungan untuk alat analitik

## Kesimpulan

Dengan mengikuti langkah-langkah di atas, Sora-webserver Anda akan menjadi produk yang mudah diinstal, dikonfigurasi, dan digunakan seperti webserver populer lainnya seperti Apache dan Nginx. Perhatikan bahwa proses ini membutuhkan waktu dan upaya yang signifikan, terutama dalam mengembangkan alat pengelolaan dan dokumentasi yang baik.

Fokus utama Anda harus pada:
1. Pengalaman pengguna yang mulus selama instalasi
2. Dokumentasi yang jelas dan komprehensif
3. Alat manajemen yang intuitif
4. Kompatibilitas dengan aplikasi web populer

Dengan pendekatan ini, Anda dapat membangun basis pengguna yang solid dan memposisikan Sora-webserver sebagai alternatif yang layak untuk solusi webserver yang sudah mapan. 