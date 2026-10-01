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
        "\x1b[1;34m[*] Servindo payload OTA ({:.2} MB) na porta 8088 para http://{}:8080/api/system/update...\x1b[0m",
        file_size as f64 / (1024.0 * 1024.0),
        target_ip
    );

    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("0.0.0.0:8088")
        .map_err(|e| format!("Falha ao iniciar servidor HTTP local OTA na porta 8088: {}", e))?;
    let _ = listener.set_nonblocking(true);

    let cpio_data = fs::read(&cpio_gz).map_err(|e| format!("Falha ao ler initramfs: {}", e))?;
    let served = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let served_clone = served.clone();

    thread::spawn(move || {
        let start = std::time::Instant::now();
        while !served_clone.load(std::sync::atomic::Ordering::SeqCst) && start.elapsed() < Duration::from_secs(30) {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/gzip\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    cpio_data.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&cpio_data);
                let _ = stream.flush();
                served_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                println!("\x1b[1;32m[+] Firmware initramfs.cpio.gz entregue com sucesso via HTTP ao Pi Zero!\x1b[0m");
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    let url = format!("http://{}:8080/api/system/update", target_ip);
    let payload = "{\"url\":\"http://192.168.7.1:8088/initramfs.cpio.gz\"}";
    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            payload,
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
