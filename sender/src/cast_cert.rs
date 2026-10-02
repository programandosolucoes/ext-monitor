//! Pure-Rust Cast V2 Certificate Management & Google Chrome Auto-Provisioning
//!
//! Generates dedicated Root CA and Device certificates for Ext-Monitor Cast V2,
//! and automatically injects `--cast-developer-certificate-path` into the local
//! Google Chrome installation on Linux so Chrome accepts Ext-Monitor as a genuine
//! Chromecast for all websites, tab mirroring, and desktop mirroring.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
pub struct CastCertPaths {
    pub ca_cert: PathBuf,
    pub ca_key: PathBuf,
    pub dev_cert: PathBuf,
    pub dev_key: PathBuf,
}

/// Returns the configuration directory for Ext-Monitor Cast certificates
pub fn get_cast_cert_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/carlos".to_string());
    PathBuf::from(home).join(".config/ext-monitor/cast-certs")
}

/// Ensures valid Cast V2 CA and Device certificates exist on disk
pub fn ensure_cast_certificates() -> Result<CastCertPaths, Box<dyn std::error::Error>> {
    let dir = get_cast_cert_dir();
    fs::create_dir_all(&dir)?;

    let ca_cert = dir.join("cast-ca.pem");
    let ca_key = dir.join("cast-ca.key");
    let dev_cert = dir.join("cast-dev.pem");
    let dev_key = dir.join("cast-dev.key");
    let dev_csr = dir.join("cast-dev.csr");

    if !ca_cert.exists() || !ca_key.exists() || !dev_cert.exists() || !dev_key.exists() {
        println!("\x1b[1;34m[cast-cert]\x1b[0m Generating high-grade 2048-bit RSA Cast Root CA & Device Certificates...");

        // 1. Generate Root CA
        let status = Command::new("openssl")
            .args([
                "req", "-x509", "-newkey", "rsa:2048", "-days", "3650", "-nodes",
                "-keyout", ca_key.to_str().unwrap(),
                "-out", ca_cert.to_str().unwrap(),
                "-subj", "/CN=Ext-Monitor Cast Root CA/O=Ext-Monitor/C=BR",
            ])
            .status()?;

        if !status.success() {
            return Err("Falha ao gerar Root CA com openssl".into());
        }

        // 2. Generate Device Key & CSR
        let status = Command::new("openssl")
            .args([
                "req", "-newkey", "rsa:2048", "-nodes",
                "-keyout", dev_key.to_str().unwrap(),
                "-out", dev_csr.to_str().unwrap(),
                "-subj", "/CN=Ext-Monitor/O=Ext-Monitor/C=BR",
            ])
            .status()?;

        if !status.success() {
            return Err("Falha ao gerar Device CSR com openssl".into());
        }

        // 3. Sign Device Certificate with CA (Subject Alternative Names for local IP & MDNS)
        let ext_content = "subjectAltName=IP:192.168.7.2,IP:192.168.7.1,IP:127.0.0.1,DNS:pi-zero.local,DNS:localhost\n";
        let ext_file = dir.join("extfile.cnf");
        fs::write(&ext_file, ext_content)?;

        let status = Command::new("openssl")
            .args([
                "x509", "-req",
                "-in", dev_csr.to_str().unwrap(),
                "-CA", ca_cert.to_str().unwrap(),
                "-CAkey", ca_key.to_str().unwrap(),
                "-CAcreateserial",
                "-out", dev_cert.to_str().unwrap(),
                "-days", "3650",
                "-extfile", ext_file.to_str().unwrap(),
            ])
            .status()?;

        let _ = fs::remove_file(ext_file);
        let _ = fs::remove_file(dev_csr);

        if !status.success() {
            return Err("Falha ao assinar certificado do dispositivo".into());
        }

        println!("\x1b[1;32m[cast-cert]\x1b[0m Cast V2 Certificates generated at {:?}", dir);
    }

    Ok(CastCertPaths {
        ca_cert,
        ca_key,
        dev_cert,
        dev_key,
    })
}

/// Automatically configures Google Chrome to accept the Ext-Monitor Cast Certificate
pub fn auto_provision_chrome() -> Result<(), Box<dyn std::error::Error>> {
    let certs = ensure_cast_certificates()?;
    let ca_path_str = certs.ca_cert.to_str().unwrap_or("");
    let flag_arg = format!("--cast-developer-certificate-path={}", ca_path_str);

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/carlos".to_string());

    // 1. Update ~/.local/share/applications/google-chrome.desktop
    let desktop_path = PathBuf::from(&home).join(".local/share/applications/google-chrome.desktop");
    if desktop_path.exists() {
        if let Ok(content) = fs::read_to_string(&desktop_path) {
            let mut modified = false;
            let mut new_lines = Vec::new();

            for line in content.lines() {
                if line.starts_with("Exec=") && !line.contains("--cast-developer-certificate-path") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        // Insert flag right after executable
                        let exec_cmd = parts[0];
                        let rest = &parts[1..];
                        let new_line = format!("{} {} {}", exec_cmd, flag_arg, rest.join(" "));
                        new_lines.push(new_line);
                    } else {
                        new_lines.push(format!("{} {}", line, flag_arg));
                    }
                    modified = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }

            if modified {
                let _ = fs::write(&desktop_path, new_lines.join("\n"));
                println!("\x1b[1;32m[cast-cert]\x1b[0m Auto-provisioned Google Chrome launcher: {:?}", desktop_path);
            }
        }
    }

    // 2. Write to ~/.config/chrome-flags.conf and ~/.config/chromium-flags.conf
    let config_dir = PathBuf::from(&home).join(".config");
    let _ = fs::create_dir_all(&config_dir);

    for conf_name in &["chrome-flags.conf", "chromium-flags.conf"] {
        let conf_path = config_dir.join(conf_name);
        let existing = fs::read_to_string(&conf_path).unwrap_or_default();
        if !existing.contains("--cast-developer-certificate-path") {
            let mut new_content = existing.trim().to_string();
            if !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(&flag_arg);
            new_content.push('\n');
            let _ = fs::write(&conf_path, new_content);
            println!("\x1b[1;32m[cast-cert]\x1b[0m Auto-configured Chrome flags file: {:?}", conf_path);
        }
    }

    // 3. Create helper wrapper ~/.local/bin/google-chrome-cast
    let local_bin = PathBuf::from(&home).join(".local/bin");
    let _ = fs::create_dir_all(&local_bin);
    let wrapper_path = local_bin.join("google-chrome-cast");
    let wrapper_script = format!(
        "#!/bin/sh\nexec /usr/bin/google-chrome-stable {} \"$@\"\n",
        flag_arg
    );
    let _ = fs::write(&wrapper_path, wrapper_script);
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&wrapper_path)?.permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&wrapper_path, perms);
    }
    println!("\x1b[1;32m[cast-cert]\x1b[0m Created dedicated launcher wrapper: {:?}", wrapper_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_cert_dir_path() {
        let dir = get_cast_cert_dir();
        assert!(dir.ends_with(".config/ext-monitor/cast-certs"));
    }

    #[test]
    fn test_ensure_certificates_generation() {
        let certs = ensure_cast_certificates().expect("Failed to ensure certificates");
        assert!(certs.ca_cert.exists());
        assert!(certs.ca_key.exists());
        assert!(certs.dev_cert.exists());
        assert!(certs.dev_key.exists());

        let ca_data = fs::read_to_string(&certs.ca_cert).unwrap();
        assert!(ca_data.contains("BEGIN CERTIFICATE"));

        let dev_data = fs::read_to_string(&certs.dev_cert).unwrap();
        assert!(dev_data.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn test_auto_provision_chrome() {
        let res = auto_provision_chrome();
        assert!(res.is_ok());

        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/carlos".to_string());
        let desktop = PathBuf::from(&home).join(".local/share/applications/google-chrome.desktop");
        if desktop.exists() {
            let content = fs::read_to_string(desktop).unwrap();
            assert!(content.contains("--cast-developer-certificate-path="));
        }

        let wrapper = PathBuf::from(&home).join(".local/bin/google-chrome-cast");
        assert!(wrapper.exists());
    }
}

