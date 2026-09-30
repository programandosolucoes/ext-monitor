//! Native Rust OTA Deployment Engine for ExtMonitor
//!
//! Sends the newly compiled initramfs.cpio.gz directly to the running
//! Pi Zero appliance in RAM via HTTP POST /api/system/update with zero SD card wear.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn run_deploy(project_root: &Path, target_ip: &str) -> Result<(), String> {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ExtMonitor OTA Live Deployment Engine                                 \x1b[0m");
    println!("\x1b[1;34m  Target: http://{}:8080/api/system/update (Zero SD Wear)              \x1b[0m", target_ip);
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    let cpio_gz = project_root.join("build-appliance/boot/initramfs.cpio.gz");
    if !cpio_gz.exists() {
        return Err(format!(
            "Arquivo initramfs não encontrado em: {:?}. Execute 'ext-tool build' primeiro.",
            cpio_gz
        ));
    }

    let file_size = fs::metadata(&cpio_gz)
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "\x1b[1;34m[*] Enviando payload OTA ({:.2} MB) para http://{}:8080/api/system/update...\x1b[0m",
        file_size as f64 / (1024.0 * 1024.0),
        target_ip
    );

    let url = format!("http://{}:8080/api/system/update", target_ip);
    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "--data-binary",
            &format!("@{}", cpio_gz.display()),
            &url,
        ])
        .output()
        .map_err(|e| format!("Falha ao executar curl para deploy OTA: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Falha na comunicação OTA: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let resp = String::from_utf8_lossy(&output.stdout);
    println!("\x1b[1;32m[+] Resposta do Pi Zero:\x1b[0m {}", resp);

    println!("\x1b[1;34m[*] Aguardando reinicialização ultrarrápida do Pi Zero (RAM boot < 3s)...\x1b[0m");
    thread::sleep(Duration::from_millis(3000));

    // Verifica se voltou a responder
    let status_url = format!("http://{}:8080/api/status", target_ip);
    for _attempt in 1..=5 {
        if let Ok(chk) = Command::new("curl").args(["-s", "-m", "2", &status_url]).output() {
            if chk.status.success() && !chk.stdout.is_empty() {
                println!("\x1b[1;32m[✔] Pi Zero reinicializado com o novo firmware e 100% online!\x1b[0m\n");
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(1000));
    }

    println!("\x1b[1;33m[!] Pi Zero ainda está reiniciando ou mudando de IP.\x1b[0m");
    Ok(())
}
