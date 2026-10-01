//! ExtMonitor Pure-Rust Swiss Army Knife: Build, Package, Deploy, Splash & Tools
//!
//! Replaces all legacy .sh and .py build/deploy/packaging scripts with a single
//! fast, typed, and dependency-free native Rust tool.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

mod builder;
mod deploy;
mod flash;
mod scan;
mod splash;
mod test;

use std::env;
use std::path::PathBuf;

fn print_usage() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-tool: ExtMonitor Native Rust Build, Deploy & Appliance Manager    \x1b[0m");
    println!("\x1b[1;34m  100% Pure Rust | Zero Script Dependencies (.sh / .py eliminated)      \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    println!("\x1b[1;33mUSO:\x1b[0m");
    println!("  ext-tool <SUBCOMANDO> [OPÇÕES]\n");

    println!("\x1b[1;33mSUBCOMANDOS DISPONÍVEIS:\x1b[0m");
    println!("  \x1b[1;32mtest\x1b[0m [--unit] [--docker] [--usb] Executa bateria de testes unitários ou container isolado Docker");
    println!("  \x1b[1;32mbuild\x1b[0m [--image] [--splash]  Compila ext-receiver (ARMv6), ext-sender e gera initramfs.cpio.gz");
    println!("  \x1b[1;32mdeploy\x1b[0m [--ip=<IP>]          Envia atualização OTA ao vivo para o Pi Zero (http://IP:8080)");
    println!("  \x1b[1;32msplash\x1b[0m                      Gera telas de splash (loading/ready) em RGB565 raw.gz em Rust");
    println!("  \x1b[1;32mflash\x1b[0m <DISPOSITIVO>         Grava a imagem da appliance no cartão SD (ex: /dev/sdb)");
    println!("  \x1b[1;32mscan\x1b[0m                        Descobre placas Pi Zero conectadas via USB/Rede");
    println!("  \x1b[1;32mhelp\x1b[0m, -h, --help            Exibe esta mensagem de ajuda\n");

    println!("\x1b[1;33mEXEMPLOS RÁPIDOS:\x1b[0m");
    println!("  ext-tool test               # Executa todos os testes unitários nativos em Rust");
    println!("  ext-tool test --docker      # Executa testes isolados no Docker (blindagem de GNOME)");
    println!("  ext-tool test --usb         # Diagnostica a comunicação direta com o gadget USB");
    println!("  ext-tool build              # Compila e empacota initramfs.cpio.gz");
    println!("  ext-tool deploy             # Envia o novo initramfs para o Pi Zero via OTA");
    println!("  ext-tool splash             # Regenera as telas de inicialização em RGB565");
    println!("  ext-tool scan               # Diagnostica a conexão do Pi Zero\n");
}

fn find_project_root() -> PathBuf {
    let mut cur = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..5 {
        if cur.join("Cargo.toml").exists() && cur.join("receiver").exists() && cur.join("sender").exists() {
            return cur;
        }
        if let Some(parent) = cur.parent() {
            cur = parent.to_path_buf();
        } else {
            break;
        }
    }
    PathBuf::from(".")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" || args[1] == "help" {
        print_usage();
        return Ok(());
    }

    let project_root = find_project_root();

    match args[1].as_str() {
        "test" => {
            let run_docker = args.iter().any(|a| a == "--docker" || a == "--isolated");
            let test_usb = args.iter().any(|a| a == "--usb");
            let run_unit = args.iter().any(|a| a == "--unit") || (!run_docker && !test_usb);
            let opts = test::TestOptions {
                project_root,
                run_unit,
                run_docker,
                test_usb,
            };
            if let Err(e) = test::run_tests(&opts) {
                eprintln!("\x1b[1;31m[!] Erro na execução dos testes: {}\x1b[0m", e);
                std::process::exit(1);
            }
        }
        "build" => {
            let create_image = args.iter().any(|a| a == "--image");
            let generate_splashes = args.iter().any(|a| a == "--splash" || a == "--splashes");
            let opts = builder::BuildOptions {
                project_root,
                create_image,
                generate_splashes,
            };
            if let Err(e) = builder::run_build(&opts) {
                eprintln!("\x1b[1;31m[!] Erro no build: {}\x1b[0m", e);
                std::process::exit(1);
            }
        }
        "deploy" => {
            let ip = args
                .iter()
                .find_map(|a| a.strip_prefix("--ip="))
                .unwrap_or("192.168.7.2");
            if let Err(e) = deploy::run_deploy(&project_root, ip) {
                eprintln!("\x1b[1;31m[!] Erro no deploy: {}\x1b[0m", e);
                std::process::exit(1);
            }
        }
        "splash" => {
            let out_dir = project_root.join("build-appliance/initramfs/etc");
            let _ = splash::generate_loading_splash(&out_dir);
            let _ = splash::generate_ready_splash(&out_dir);
            let artifacts_dir = project_root.join("receiver/splash");
            if artifacts_dir.exists() {
                let _ = splash::generate_loading_splash(&artifacts_dir);
                let _ = splash::generate_ready_splash(&artifacts_dir);
            }
            println!("\x1b[1;32m[✔] Todas as telas de splash foram geradas em Rust nativo!\x1b[0m");
        }
        "flash" => {
            if args.len() < 3 {
                eprintln!("\x1b[1;31m[!] Informe o dispositivo de destino (ex: ext-tool flash /dev/sdb)\x1b[0m");
                std::process::exit(1);
            }
            let dev = &args[2];
            if let Err(e) = flash::run_flash(&project_root, dev) {
                eprintln!("\x1b[1;31m[!] Erro ao gravar SD: {}\x1b[0m", e);
                std::process::exit(1);
            }
        }
        "scan" => {
            if let Err(e) = scan::run_scan() {
                eprintln!("\x1b[1;31m[!] Erro no scan: {}\x1b[0m", e);
                std::process::exit(1);
            }
        }
        unknown => {
            eprintln!("\x1b[1;31m[!] Subcomando desconhecido: '{}'\x1b[0m\n", unknown);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}
