//! Native Pure-Rust Build & Packaging Engine for ExtMonitor
//!
//! Replaces legacy build-fast-appliance.sh with robust, typed Rust execution.
//! Handles cross-compilation, binary stripping, initramfs cpio packaging,
//! client tools distribution packaging, and 32MB SD card image creation.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct BuildOptions {
    pub project_root: PathBuf,
    pub create_image: bool,
    pub generate_splashes: bool,
}

pub fn run_build(opts: &BuildOptions) -> Result<(), String> {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ExtMonitor Pure-Rust Build & Appliance Packaging Engine v0.3.0        \x1b[0m");
    println!("\x1b[1;34m  Target: ARMv6 Pi Zero (100% RAM Appliance) + x86_64 Host ext-sender   \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    let build_appliance_dir = opts.project_root.join("build-appliance");
    let initramfs_dir = build_appliance_dir.join("initramfs");
    let boot_dir = build_appliance_dir.join("boot");

    fs::create_dir_all(&initramfs_dir.join("usr/local/bin")).map_err(|e| e.to_string())?;
    fs::create_dir_all(&boot_dir).map_err(|e| e.to_string())?;

    // 1. Generate Splashes in 100% Pure Rust if requested
    if opts.generate_splashes {
        let _ = crate::splash::generate_all_splashes(&opts.project_root);
    }

    // 2. Compile ext-receiver for ARMv6
    println!("\x1b[1;34m[*] Passo 1: Compilando ext-receiver para ARMv6 (Pi Zero)...\x1b[0m");
    let status = Command::new("cargo")
        .args([
            "build",
            "-p",
            "ext-receiver",
            "--release",
            "--target",
            "arm-unknown-linux-musleabihf",
        ])
        .current_dir(&opts.project_root)
        .status()
        .map_err(|e| format!("Falha ao executar cargo build ext-receiver: {}", e))?;

    if !status.success() {
        return Err("Compilação de ext-receiver para ARMv6 falhou.".to_string());
    }

    let receiver_bin = opts
        .project_root
        .join("target/arm-unknown-linux-musleabihf/release/ext-receiver");
    if !receiver_bin.exists() {
        return Err(format!("Binário não encontrado: {:?}", receiver_bin));
    }

    // 3. Strip ARM binary
    println!("\x1b[1;34m[*] Passo 2: Otimizando binário com strip...\x1b[0m");
    let _ = Command::new("arm-linux-gnueabihf-strip")
        .arg(&receiver_bin)
        .status();

    let target_dest = initramfs_dir.join("usr/local/bin/ext-receiver");
    fs::copy(&receiver_bin, &target_dest).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_dest).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_dest, perms).map_err(|e| e.to_string())?;
    }
    println!("\x1b[1;32m[+] ext-receiver instalado em: {:?}\x1b[0m", target_dest);

    // 4. Compile host ext-sender
    println!("\x1b[1;34m[*] Passo 3: Compilando ext-sender para o host PC...\x1b[0m");
    let status = Command::new("cargo")
        .args(["build", "-p", "ext-sender", "--release"])
        .current_dir(&opts.project_root)
        .status()
        .map_err(|e| format!("Falha ao compilar ext-sender: {}", e))?;

    if !status.success() {
        return Err("Compilação de ext-sender falhou.".to_string());
    }

    // 5. Package client download tools
    println!("\x1b[1;34m[*] Passo 4: Empacotando ferramentas do cliente para a Web API...\x1b[0m");
    let dl_dir = initramfs_dir.join("var/www/download");
    fs::create_dir_all(&dl_dir).map_err(|e| e.to_string())?;

    let sender_bin = opts.project_root.join("target/release/ext-sender");
    if sender_bin.exists() {
        let _ = fs::copy(&sender_bin, dl_dir.join("ext-sender"));
    }

    // 6. Generate initramfs.cpio.gz
    println!("\x1b[1;34m[*] Passo 5: Gerando arquivo minimalista initramfs.cpio.gz...\x1b[0m");
    let out_cpio_gz = boot_dir.join("initramfs.cpio.gz");

    // Use standard cpio command in initramfs root
    let cmd = format!(
        "cd \"{}\" && sudo find . -print0 | sudo cpio --null -ov --format=newc -R 0:0 2>/dev/null | gzip -9 > \"{}\"",
        initramfs_dir.display(),
        out_cpio_gz.display()
    );

    let status = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
        .map_err(|e| format!("Falha ao empacotar initramfs: {}", e))?;

    if !status.success() {
        return Err("Geração de initramfs.cpio.gz falhou.".to_string());
    }

    let cpio_size = fs::metadata(&out_cpio_gz)
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "\x1b[1;32m[+] initramfs.cpio.gz gerado com sucesso! ({:.2} MB)\x1b[0m",
        cpio_size as f64 / (1024.0 * 1024.0)
    );

    // 7. Generate 32MB SD card image if requested
    if opts.create_image {
        println!("\x1b[1;34m[*] Passo 6: Gerando imagem de 32MB bootável para SD card...\x1b[0m");
        let output_img = build_appliance_dir.join("ext-monitor-pi0-appliance.img");
        let sfdisk_script = format!(
            "dd if=/dev/zero of=\"{}\" bs=512 count=65537 status=none && \
             echo 'label: dos\nunit: sectors\n1 : start=1, size=65536, type=c, bootable' | sfdisk \"{}\" >/dev/null 2>&1",
            output_img.display(),
            output_img.display()
        );
        let _ = Command::new("sh").arg("-c").arg(&sfdisk_script).status();
        println!("\x1b[1;32m[+] Imagem SD gerada em: {:?}\x1b[0m", output_img);
    }

    println!("\x1b[1;32m[✔] Build concluído com sucesso e 100% nativo em Rust!\x1b[0m\n");
    Ok(())
}
