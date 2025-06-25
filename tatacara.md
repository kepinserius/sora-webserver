# Cara Menggunakan RustWeb Server dengan Mudah seperti Apache

Untuk membuat RustWeb Server mudah digunakan oleh orang lain seperti Apache, berikut adalah panduan lengkapnya:

## 1. Metode Penggunaan

### Penggunaan Langsung (Seperti Apache)

1. **Instalasi Paket**
   - Buatlah paket instalasi (.deb, .rpm) agar RustWeb Server dapat diinstal dengan mudah melalui `apt` atau `yum`
   - Contoh perintah instalasi yang ideal:
   ```bash
   sudo apt install rustweb-server
   ```

2. **Struktur Direktori Standar**
   - `/etc/rustweb/` - Untuk file konfigurasi
   - `/var/www/html/` - Untuk file website
   - `/var/log/rustweb/` - Untuk log server
   - `/var/lib/rustweb/` - Untuk data cache dan lainnya

3. **Service System**
   - Daftarkan sebagai systemd service agar mudah dijalankan/dihentikan:
   ```bash
   sudo systemctl start rustweb
   sudo systemctl enable rustweb
   ```

## 2. Penggunaan dengan Docker

1. **Pull Image Docker**
   ```bash
   docker pull username/rustweb-server
   ```

2. **Jalankan Container**
   ```bash
   docker run -d -p 8080:8080 -p 8443:8443 \
     -v /path/ke/website:/app/www \
     -v /path/ke/config:/app/config \
     --name rustweb username/rustweb-server
   ```

3. **Menggunakan Docker Compose**
   - Clone repositori
   - Sesuaikan `docker-compose.yml`
   - Jalankan dengan perintah:
   ```bash
   docker-compose up -d
   ```

## 3. Hosting RustWeb Server

### Opsi Self-Hosting (Mengelola Sendiri)

1. **VPS (Virtual Private Server)**
   - Sewa VPS dari penyedia seperti DigitalOcean, Linode, atau AWS EC2
   - Instal RustWeb Server di VPS
   - Konfigurasikan firewall untuk membuka port 80 dan 443
   - Arahkan domain ke IP VPS Anda

2. **Dedicated Server**
   - Untuk trafik tinggi, sewa server dedicated
   - Instal dan konfigurasikan RustWeb Server
   - Optimalkan performa server sesuai kebutuhan

### Opsi Cloud Hosting

1. **Container Service**
   - Gunakan AWS ECS, Google Cloud Run, atau Azure Container Instances
   - Deploy image Docker RustWeb Server
   - Konfigurasikan load balancer dan auto-scaling

2. **Kubernetes**
   - Deploy RustWeb Server di cluster Kubernetes
   - Konfigurasikan Ingress controller untuk routing
   - Manfaatkan auto-scaling dan self-healing

## 4. Langkah-Langkah Penggunaan Praktis

1. **Persiapkan Server**
   - Instal sistem operasi Linux (Ubuntu/Debian direkomendasikan)
   - Pastikan paket dasar terinstal (`build-essential`, `libssl-dev`)

2. **Instal RustWeb Server**
   ```bash
   # Instal Rust dan Cargo
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env

   # Clone dan build RustWeb Server
   git clone https://github.com/username/rustweb-server
   cd rustweb-server
   cargo build --release
   
   # Pindahkan binary ke direktori sistem
   sudo cp target/release/webserver /usr/local/bin/rustweb
   ```

3. **Buat Struktur Direktori**
   ```bash
   sudo mkdir -p /etc/rustweb /var/www/html /var/log/rustweb /var/lib/rustweb/cache
   ```

4. **Salin Konfigurasi Dasar**
   ```bash
   sudo cp config/server.toml /etc/rustweb/
   ```

5. **Buat Service Systemd**
   Buat file `/etc/systemd/system/rustweb.service`:
   ```
   [Unit]
   Description=RustWeb Server
   After=network.target

   [Service]
   ExecStart=/usr/local/bin/rustweb -c /etc/rustweb/server.toml
   WorkingDirectory=/var/www
   Restart=always
   User=www-data
   Group=www-data

   [Install]
   WantedBy=multi-user.target
   ```

6. **Mulai dan Aktifkan Service**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl start rustweb
   sudo systemctl enable rustweb
   ```

7. **Tambahkan Website Anda**
   - Letakkan file HTML, CSS, dan JS di `/var/www/html`
   - Atau gunakan virtual host untuk beberapa website

8. **Dapatkan Sertifikat SSL**
   ```bash
   # Instal Certbot
   sudo apt install certbot
   
   # Dapatkan sertifikat
   sudo certbot certonly --standalone -d domain-anda.com
   
   # Konfigurasikan RustWeb untuk menggunakan sertifikat tersebut
   ```

## 5. Mempermudah Pengelolaan

1. **Buat Panel Admin Web**
   - Tambahkan fitur panel admin seperti Apache/Nginx untuk konfigurasi visual
   - Berikan opsi untuk mengelola virtual host, SSL, dan modul

2. **Sediakan Template Konfigurasi**
   - Sediakan contoh konfigurasi untuk berbagai kebutuhan (PHP, Node.js, dll)
   - Buat wizard untuk pembuatan konfigurasi

3. **Dokumentasi Lengkap**
   - Buat dokumentasi dalam Bahasa Indonesia
   - Sertakan contoh penggunaan umum
   - Tambahkan troubleshooting dan FAQ

## 6. Perbedaan dengan Apache

RustWeb Server dapat digunakan seperti Apache, namun ada beberapa perbedaan:

1. **Performa Lebih Tinggi**
   - Dibuat dengan Rust, memiliki performa dan efisiensi memori lebih baik

2. **Konfigurasi dengan TOML**
   - Apache menggunakan format .conf, RustWeb menggunakan TOML yang lebih mudah dibaca

3. **Fitur Modern**
   - Dukungan TLS modern, HTTP/2, dan fitur keamanan terkini

## 7. Contoh Penggunaan untuk Kebutuhan Umum

### Website Statis
```toml
# /etc/rustweb/sites/website-statis.toml
[vhost]
hostname = "website-statis.com"
document_root = "/var/www/website-statis"
```

### API Backend
```toml
# /etc/rustweb/sites/api.toml
[vhost]
hostname = "api.domain.com"

[[proxy]]
path = "/"
target = "http://localhost:3000"
```

### Website WordPress
```toml
# /etc/rustweb/sites/wordpress.toml
[vhost]
hostname = "blog.domain.com"
document_root = "/var/www/wordpress"

[[proxy]]
path = "/"
target = "http://localhost:9000"  # Untuk PHP-FPM
```

## 8. Panduan Pemecahan Masalah Umum

### Server Tidak Dapat Dimulai

1. **Periksa Status Service**
   ```bash
   sudo systemctl status rustweb
   ```

2. **Periksa Log**
   ```bash
   sudo journalctl -u rustweb
   cat /var/log/rustweb/error.log
   ```

3. **Masalah Port**
   - Pastikan port 80/443 tidak digunakan oleh layanan lain
   ```bash
   sudo netstat -tulpn | grep -E ':(80|443)'
   ```

### Situs Tidak Dapat Diakses

1. **Periksa Konfigurasi Virtual Host**
   - Pastikan domain tercantum di konfigurasi
   - Periksa path document_root

2. **Periksa Izin File**
   ```bash
   sudo chown -R www-data:www-data /var/www/
   sudo chmod -R 755 /var/www/
   ```

3. **Firewall**
   - Pastikan port dibuka di firewall
   ```bash
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   ```

## 9. Perbandingan Dengan Server Web Lainnya

| Fitur | RustWeb Server | Apache | Nginx |
|-------|----------------|--------|-------|
| Bahasa | Rust | C | C |
| Performa | Tinggi | Sedang | Tinggi |
| Penggunaan Memori | Rendah | Tinggi | Rendah |
| Konfigurasi | TOML | .conf | .conf |
| Virtual Hosts | Ya | Ya | Ya |
| Reverse Proxy | Ya | Ya | Ya |
| HTTP/2 | Ya | Ya | Ya |
| TLS 1.3 | Ya | Ya | Ya |
| Modular | Ya | Ya | Terbatas |

## 10. Kesimpulan

RustWeb Server menawarkan alternatif modern untuk Apache dan Nginx dengan performa tinggi dan konfigurasi yang lebih mudah dibaca. Dengan mengikuti panduan ini, Anda dapat menggunakan RustWeb Server dengan cara yang serupa dengan Apache, namun dengan keuntungan dari teknologi yang lebih baru dan efisien.

Untuk pertanyaan lebih lanjut atau dukungan, silakan kunjungi forum komunitas atau buat issue di repositori GitHub. 