//! Pi Zero Discovery and Scanner for ExtMonitor
//!
//! Scans USB interfaces and IP subnet for connected Raspberry Pi Zero appliances.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::process::Command;

pub fn run_scan() -> Result<(), String> {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ExtMonitor Pi Zero Discovery & Diagnostic Scanner                     \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;34m[*] Escaneando barramento USB por dispositivos Raspberry Pi...\x1b[0m");
    if let Ok(lsusb) = Command::new("lsusb").output() {
        let text = String::from_utf8_lossy(&lsusb.stdout);
        for line in text.lines() {
            if line.contains("1d50:614d") || line.to_lowercase().contains("raspberry") || line.contains("0a5c:") {
                println!("\x1b[1;32m[+] Dispositivo USB encontrado:\x1b[0m {}", line);
            }
        }
    }

    println!("\x1b[1;34m[*] Verificando interface de rede USB (192.168.7.2)...\x1b[0m");
    let ping = Command::new("ping")
        .args(["-c", "1", "-W", "1", "192.168.7.2"])
        .output();

    match ping {
        Ok(p) if p.status.success() => {
            println!("\x1b[1;32m[+] Pi Zero respondendo em 192.168.7.2 (Ping OK)\x1b[0m");

            // Query REST API
            if let Ok(api) = Command::new("curl").args(["-s", "-m", "2", "http://192.168.7.2:8080/api/status"]).output() {
                if api.status.success() && !api.stdout.is_empty() {
                    println!("\x1b[1;32m[+] Painel Web e API REST online em http://192.168.7.2:8080\x1b[0m");
                    println!("\x1b[1;37m    Telemetria:\x1b[0m {}", String::from_utf8_lossy(&api.stdout));
                }
            }
        }
        _ => {
            println!("\x1b[1;33m[!] Pi Zero não respondeu em 192.168.7.2.\x1b[0m");
            println!("    Certifique-se de que o cabo USB está conectado na porta de dados USB (porta central).");
        }
    }

    println!();
    Ok(())
}
