FROM rust:1.70 as builder

WORKDIR /usr/src/webserver

# Copy the Cargo files to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {println!(\"Dummy build\");}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY . .

# Build the application
RUN cargo build --release

# Use a smaller image for the runtime environment
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl1.1 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Create necessary directories
RUN mkdir -p /app/www /app/logs /app/certs /app/config

WORKDIR /app

# Copy the built binary
COPY --from=builder /usr/src/webserver/target/release/webserver /app/

# Copy default config and static files
COPY --from=builder /usr/src/webserver/config /app/config/
COPY --from=builder /usr/src/webserver/www /app/www/
COPY --from=builder /usr/src/webserver/certs /app/certs/

# Expose HTTP and HTTPS ports
EXPOSE 8080 8443

# Set environment variables
ENV RUST_LOG=info

# Run the web server
CMD ["/app/webserver", "-c", "config/server.toml"] 