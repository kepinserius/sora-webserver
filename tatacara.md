# Cara Menggunakan Sora-webserver dengan Mudah seperti Apache

Untuk membuat Sora-webserver mudah digunakan oleh orang lain seperti Apache, berikut adalah panduan lengkapnya:

## 1. Metode Penggunaan

### Penggunaan Langsung (Seperti Apache)

1. **Instalasi Paket**
   - Buatlah paket instalasi (.deb, .rpm) agar Sora-webserver dapat diinstal dengan mudah melalui `apt` atau `yum`
   - Contoh perintah instalasi yang ideal:
   ```bash
   sudo apt install sora-webserver
   ```

2. **Struktur Direktori Standar**
   - `/etc/sora-webserver/` - Untuk file konfigurasi
   - `/var/www/html/` - Untuk file website
   - `/var/log/sora-webserver/` - Untuk log server
   - `/var/lib/sora-webserver/` - Untuk data cache dan lainnya

3. **Service System**
   - Daftarkan sebagai systemd service agar mudah dijalankan/dihentikan:
   ```bash
   sudo systemctl start sora-webserver
   sudo systemctl enable sora-webserver
   ```

## 2. Penggunaan dengan Docker

1. **Pull Image Docker**
   ```bash
   docker pull kepinserius/sora-webserver
   ```

2. **Jalankan Container**
   ```bash
   docker run -d -p 8080:8080 -p 8443:8443 \
     -v /path/ke/website:/app/www \
     -v /path/ke/config:/app/config \
     --name sora-webserver kepinserius/sora-webserver
   ```

3. **Menggunakan Docker Compose**
   - Clone repositori
   - Sesuaikan `docker-compose.yml`
   - Jalankan dengan perintah:
   ```bash
   docker-compose up -d
   ```

## 3. Hosting Sora-webserver

### Opsi Self-Hosting (Mengelola Sendiri)

1. **VPS (Virtual Private Server)**
   - Sewa VPS dari penyedia seperti DigitalOcean, Linode, atau AWS EC2
   - Instal Sora-webserver di VPS
   - Konfigurasikan firewall untuk membuka port 80 dan 443
   - Arahkan domain ke IP VPS Anda

2. **Dedicated Server**
   - Untuk trafik tinggi, sewa server dedicated
   - Instal dan konfigurasikan Sora-webserver
   - Optimalkan performa server sesuai kebutuhan

### Opsi Cloud Hosting

1. **Container Service**
   - Gunakan AWS ECS, Google Cloud Run, atau Azure Container Instances
   - Deploy image Docker Sora-webserver
   - Konfigurasikan load balancer dan auto-scaling

2. **Kubernetes**
   - Deploy Sora-webserver di cluster Kubernetes
   - Konfigurasikan Ingress controller untuk routing
   - Manfaatkan auto-scaling dan self-healing

## 4. Langkah-Langkah Penggunaan Praktis

1. **Persiapkan Server**
   - Instal sistem operasi Linux (Ubuntu/Debian direkomendasikan)
   - Pastikan paket dasar terinstal (`build-essential`, `libssl-dev`)

2. **Instal Sora-webserver**
   ```bash
   # Instal Rust dan Cargo
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env

   # Clone dan build Sora-webserver
   git clone https://github.com/kepinserius/sora-webserver
   cd sora-webserver
   cargo build --release
   
   # Pindahkan binary ke direktori sistem
   sudo cp target/release/webserver /usr/local/bin/sora-webserver
   ```

3. **Buat Struktur Direktori**
   ```bash
   sudo mkdir -p /etc/sora-webserver /var/www/html /var/log/sora-webserver /var/lib/sora-webserver/cache
   ```

4. **Salin Konfigurasi Dasar**
   ```bash
   sudo cp config/server.toml /etc/sora-webserver/
   ```

5. **Buat Service Systemd**
   Buat file `/etc/systemd/system/sora-webserver.service`:
   ```
   [Unit]
   Description=Sora-webserver
   After=network.target

   [Service]
   ExecStart=/usr/local/bin/sora-webserver -c /etc/sora-webserver/server.toml
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
   sudo systemctl start sora-webserver
   sudo systemctl enable sora-webserver
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
   
   # Konfigurasikan Sora-webserver untuk menggunakan sertifikat tersebut
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

Sora-webserver dapat digunakan seperti Apache, namun ada beberapa perbedaan:

1. **Performa Lebih Tinggi**
   - Dibuat dengan Rust, memiliki performa dan efisiensi memori lebih baik

2. **Konfigurasi dengan TOML**
   - Apache menggunakan format .conf, Sora-webserver menggunakan TOML yang lebih mudah dibaca

3. **Fitur Modern**
   - Dukungan TLS modern, HTTP/2, dan fitur keamanan terkini

## 7. Contoh Penggunaan untuk Kebutuhan Umum

### Website Statis
```toml
# /etc/sora-webserver/sites/website-statis.toml
[vhost]
hostname = "website-statis.com"
document_root = "/var/www/website-statis"
```

### API Backend
```toml
# /etc/sora-webserver/sites/api.toml
[vhost]
hostname = "api.domain.com"

[[proxy]]
path = "/"
target = "http://localhost:3000"
```

### Website WordPress
```toml
# /etc/sora-webserver/sites/wordpress.toml
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
   sudo systemctl status sora-webserver
   ```

2. **Periksa Log**
   ```bash
   sudo journalctl -u sora-webserver
   cat /var/log/sora-webserver/error.log
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

| Fitur | Sora-webserver | Apache | Nginx |
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

Sora-webserver menawarkan alternatif modern untuk Apache dan Nginx dengan performa tinggi dan konfigurasi yang lebih mudah dibaca. Dengan mengikuti panduan ini, Anda dapat menggunakan Sora-webserver dengan cara yang serupa dengan Apache, namun dengan keuntungan dari teknologi yang lebih baru dan efisien.

Untuk pertanyaan lebih lanjut atau dukungan, silakan kunjungi forum komunitas atau buat issue di repositori GitHub. 