use std::path::Path;
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;

use rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use anyhow::{Context, Result};

pub struct TlsConfig {
    server_config: Arc<ServerConfig>,
}

impl TlsConfig {
    pub async fn new<P: AsRef<Path>>(cert_path: P, key_path: P) -> Result<Self> {
        // Load certificate chain
        let cert_file = File::open(&cert_path)
            .with_context(|| format!("Failed to open certificate file {:?}", cert_path.as_ref()))?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain = rustls_pemfile::certs(&mut cert_reader)
            .with_context(|| "Failed to parse certificate")?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_file = File::open(&key_path)
            .with_context(|| format!("Failed to open key file {:?}", key_path.as_ref()))?;
        let mut key_reader = BufReader::new(key_file);
        
        // Try RSA keys first, then PKCS8
        let key = match rustls_pemfile::rsa_private_keys(&mut key_reader) {
            Ok(keys) if !keys.is_empty() => keys[0].clone(),
            _ => {
                // Rewind the reader and try PKCS8
                let key_file = File::open(&key_path)?;
                let mut key_reader = BufReader::new(key_file);
                match rustls_pemfile::pkcs8_private_keys(&mut key_reader) {
                    Ok(keys) if !keys.is_empty() => keys[0].clone(),
                    _ => {
                        // Try EC keys as last resort
                        let key_file = File::open(&key_path)?;
                        let mut key_reader = BufReader::new(key_file);
                        rustls_pemfile::ec_private_keys(&mut key_reader)?
                            .get(0)
                            .context("No private key found")?
                            .clone()
                    }
                }
            }
        };
        
        let private_key = PrivateKey(key);

        // Create TLS configuration
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .context("Failed to create TLS configuration")?;

        Ok(Self {
            server_config: Arc::new(config),
        })
    }

    pub fn acceptor(&self) -> Result<TlsAcceptor> {
        Ok(TlsAcceptor::from(self.server_config.clone()))
    }

    pub fn server_config(&self) -> Arc<ServerConfig> {
        self.server_config.clone()
    }

    pub fn with_client_auth<P: AsRef<Path>>(mut self, ca_cert_path: P) -> Result<Self> {
        let ca_cert_file = File::open(&ca_cert_path)
            .with_context(|| format!("Failed to open CA certificate file {:?}", ca_cert_path.as_ref()))?;
        let mut ca_cert_reader = BufReader::new(ca_cert_file);
        let ca_certs = rustls_pemfile::certs(&mut ca_cert_reader)
            .with_context(|| "Failed to parse CA certificate")?
            .into_iter()
            .map(Certificate)
            .collect::<Vec<_>>();
        
        // Create client verification config
        let mut client_auth_roots = rustls::RootCertStore::empty();
        for ca_cert in ca_certs {
            client_auth_roots.add(&ca_cert)?;
        }

        // Create new config with client auth
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(rustls::server::AllowAnyAuthenticatedClient::new(client_auth_roots)))
            .with_single_cert(
                self.server_config.certificates().to_vec(), 
                self.server_config.key_log.clone()
            )
            .context("Failed to create TLS configuration with client auth")?;

        self.server_config = Arc::new(config);
        Ok(self)
    }
} 