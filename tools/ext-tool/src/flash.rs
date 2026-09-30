//! Native SD Card Flashing Engine for ExtMonitor
//!
//! Flashes the 32MB bootable appliance image directly to an SD card device.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::path::Path;
use std::process::Command;

pub fn run_flash(project_root: &Path, device: &str) -> Result<(), String> {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ExtMonitor SD Card Appliance Flasher                                  \x1b[0m");
    println!("\x1b[1;34m  Target Device: {}                                                     \x1b[0m", device);
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    let img_path = project_root.join("build-appliance/ext-monitor-pi0-appliance.img");
    if !img_path.exists() {
        return Err(format!(
            "Imagem da appliance não encontrada em: {:?}. Execute 'ext-tool build --image' primeiro.",
            img_path
        ));
    }

    println!("\x1b[1;33m[!] ATENÇÃO: Gravando imagem {:?} em {}...\x1b[0m", img_path, device);

    let status = Command::new("sudo")
        .args([
            "dd",
            &format!("if={}", img_path.display()),
            &format!("of={}", device),
            "bs=4M",
            "status=progress",
            "conv=fsync",
        ])
        .status()
        .map_err(|e| format!("Falha ao executar dd: {}", e))?;

    if !status.success() {
        return Err("Falha ao gravar no dispositivo de destino.".to_string());
    }

    println!("\x1b[1;32m[✔] Cartão SD gravado com sucesso! Pronto para boot no Pi Zero.\x1b[0m\n");
    Ok(())
}
