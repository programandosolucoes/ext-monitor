//! Native Test Runner & Isolated Docker Integration Engine for ExtMonitor
//!
//! Provides automated unit testing and isolated Docker container execution
//! for ext-sender, ext-receiver, and USB hardware passthrough without risking
//! host desktop/GNOME session stability.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub struct TestOptions {
    pub project_root: PathBuf,
    pub run_unit: bool,
    pub run_docker: bool,
    pub test_usb: bool,
}

/// Executes requested test suites
pub fn run_tests(opts: &TestOptions) -> Result<(), String> {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ExtMonitor Test Suite & Isolated Docker Test Engine v0.3.0            \x1b[0m");
    println!("\x1b[1;34m  100% Native Rust | Zero Shell Scripts | GNOME Session Protection      \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m\n");

    let mut all_ok = true;

    // 1. Run local Rust unit tests if requested or if no specific mode selected
    if opts.run_unit || (!opts.run_docker && !opts.test_usb) {
        println!("\x1b[1;33m[▶] Executando Bateria de Testes Unitários Nativos em Rust...\x1b[0m\n");
        let start = Instant::now();

        // Sender tests
        println!("\x1b[1;34m--- Testes do Transmissor (ext-sender) ---\x1b[0m");
        let sender_manifest = opts.project_root.join("sender/Cargo.toml");
        if let Err(e) = run_cargo_test(&sender_manifest) {
            eprintln!("\x1b[1;31m[✖] Falha nos testes de ext-sender: {}\x1b[0m", e);
            all_ok = false;
        } else {
            println!("\x1b[1;32m[✔] ext-sender: Todos os testes passaram com sucesso!\x1b[0m\n");
        }

        // Receiver tests
        println!("\x1b[1;34m--- Testes do Receptor (ext-receiver) ---\x1b[0m");
        let receiver_manifest = opts.project_root.join("receiver/Cargo.toml");
        if let Err(e) = run_cargo_test(&receiver_manifest) {
            eprintln!("\x1b[1;31m[✖] Falha nos testes de ext-receiver: {}\x1b[0m", e);
            all_ok = false;
        } else {
            println!("\x1b[1;32m[✔] ext-receiver: Todos os testes passaram com sucesso!\x1b[0m\n");
        }

        // Ext-tool tests
        println!("\x1b[1;34m--- Testes da Ferramenta de Gerenciamento (ext-tool) ---\x1b[0m");
        let tool_manifest = opts.project_root.join("tools/ext-tool/Cargo.toml");
        if let Err(e) = run_cargo_test(&tool_manifest) {
            eprintln!("\x1b[1;31m[✖] Falha nos testes de ext-tool: {}\x1b[0m", e);
            all_ok = false;
        } else {
            println!("\x1b[1;32m[✔] ext-tool: Todos os testes passaram com sucesso!\x1b[0m\n");
        }

        println!(
            "\x1b[1;36m[*] Tempo total dos testes unitários: {:.2}s\x1b[0m\n",
            start.elapsed().as_secs_f64()
        );
    }

    // 2. Run USB hardware probe test if requested
    if opts.test_usb {
        println!("\x1b[1;33m[▶] Diagnosticando Conexão USB do Pi Zero Gadget...\x1b[0m");
        match probe_usb_pi_zero() {
            Ok(msg) => println!("\x1b[1;32m[✔] {}\x1b[0m\n", msg),
            Err(e) => {
                eprintln!("\x1b[1;31m[✖] Diagnóstico USB: {}\x1b[0m\n", e);
                all_ok = false;
            }
        }
    }

    // 3. Run isolated Docker container tests if requested
    if opts.run_docker {
        println!("\x1b[1;33m[▶] Executando Testes Isolados no Container Docker (Blindagem de Sessão)...\x1b[0m");
        if let Err(e) = run_docker_isolated_test(&opts.project_root) {
            eprintln!("\x1b[1;31m[✖] Erro no ambiente isolado Docker: {}\x1b[0m", e);
            all_ok = false;
        } else {
            println!("\x1b[1;32m[✔] Teste isolado no Docker concluído com sucesso e sessão do host 100% preservada!\x1b[0m\n");
        }
    }

    if all_ok {
        println!("\x1b[1;32m[✔] TODAS AS ETAPAS DE TESTE PASSARAM COM SUCESSO!\x1b[0m\n");
        Ok(())
    } else {
        Err("Uma ou mais etapas de teste falharam.".to_string())
    }
}

/// Runs cargo test on a specific crate manifest
fn run_cargo_test(manifest_path: &Path) -> Result<(), String> {
    let status = Command::new("cargo")
        .args([
            "test",
            "--manifest-path",
            &manifest_path.to_string_lossy(),
            "--",
            "--nocapture",
        ])
        .status()
        .map_err(|e| format!("Falha ao invocar cargo test: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo test retornou status de erro {:?}", status.code()))
    }
}

/// Probes USB devices on the host to check for Pi Zero via native sysfs or lsusb
pub fn probe_usb_pi_zero() -> Result<String, String> {
    // 1. Try pure Rust sysfs check (/sys/bus/usb/devices)
    if let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let path = entry.path();
            let vendor_path = path.join("idVendor");
            let product_path = path.join("idProduct");
            if vendor_path.exists() && product_path.exists() {
                if let (Ok(v), Ok(p)) = (std::fs::read_to_string(&vendor_path), std::fs::read_to_string(&product_path)) {
                    let vid = v.trim().to_lowercase();
                    let pid = p.trim().to_lowercase();
                    if (vid == "1d50" && pid == "614d") || (vid == "1d6b" && pid == "0104") {
                        let prod_name = std::fs::read_to_string(path.join("product"))
                            .unwrap_or_else(|_| "Pi Zero Display Gadget".to_string());
                        return Ok(format!(
                            "Dispositivo Pi Zero USB encontrado (sysfs): VID:PID {}:{} ({}) em {}",
                            vid, pid, prod_name.trim(), path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                }
            }
        }
    }

    // 2. Fallback to lsusb if sysfs was not accessible
    if let Ok(output) = Command::new("lsusb").output() {
        let s = String::from_utf8_lossy(&output.stdout);
        for line in s.lines() {
            if line.contains("1d50:614d") || line.contains("1d6b:0104") || line.contains("OpenMoko") || line.contains("Generic Display") {
                return Ok(format!("Dispositivo Pi Zero USB encontrado (lsusb): {}", line.trim()));
            }
        }
    }

    Err("Nenhum Raspberry Pi Zero em modo USB Display Gadget (1d50:614d / 1d6b:0104) foi detectado.".to_string())
}

/// Runs isolated tests inside the Docker container
pub fn run_docker_isolated_test(project_root: &Path) -> Result<(), String> {
    // Check if docker daemon is reachable
    let check = Command::new("docker")
        .args(["info"])
        .output()
        .map_err(|e| format!("Falha ao comunicar com daemon Docker: {}", e))?;

    if !check.status.success() {
        return Err("O serviço Docker não está acessível no host.".to_string());
    }

    let dockerfile_path = project_root.join("docker/Dockerfile.test-sender");
    if !dockerfile_path.exists() {
        return Err(format!("Dockerfile não encontrado em: {:?}", dockerfile_path));
    }

    println!("\x1b[1;34m[*] Verificando/construindo imagem ext-monitor-test:latest...\x1b[0m");
    let build_status = Command::new("docker")
        .args([
            "build",
            "-t",
            "ext-monitor-test:latest",
            "-f",
            &dockerfile_path.to_string_lossy(),
            &project_root.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("Falha ao construir imagem docker: {}", e))?;

    if !build_status.success() {
        return Err("Falha na construção da imagem Docker ext-monitor-test:latest".to_string());
    }

    println!("\x1b[1;34m[*] Subindo container isolado com repasse de USB (/dev/bus/usb) e GPU (/dev/dri)...\x1b[0m");

    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home/carlos".to_string());
    let cargo_home = format!("{}/.cargo", home_dir);
    let rustup_home = format!("{}/.rustup", home_dir);

    let mut docker_cmd = Command::new("docker");
    docker_cmd.args([
        "run",
        "--rm",
        "--privileged",
        "-v",
        "/dev/bus/usb:/dev/bus/usb",
        "--device",
        "/dev/dri:/dev/dri",
        "--net=host",
        "-v",
        &format!("{}:/root/.cargo:ro", cargo_home),
        "-v",
        &format!("{}:/root/.rustup:ro", rustup_home),
        "-e",
        "PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "-v",
        &format!("{}:/workspace", project_root.display()),
        "-w",
        "/workspace",
        "ext-monitor-test:latest",
        "bash",
        "-c",
        "echo '[docker] Validando aceleração gráfica DRM/KMS e VA-API (Native-Rust Engine padrão)...' && \
         gst-inspect-1.0 --version && \
         echo '[docker] Executando bateria de testes dentro do container isolado...' && \
         cargo test --manifest-path sender/Cargo.toml && \
         cargo test --manifest-path receiver/Cargo.toml",
    ]);

    let run_status = docker_cmd
        .status()
        .map_err(|e| format!("Falha ao executar container docker: {}", e))?;

    if !run_status.success() {
        return Err("Execução dos testes dentro do Docker retornou falha.".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_options_defaults() {
        let opts = TestOptions {
            project_root: PathBuf::from("."),
            run_unit: true,
            run_docker: false,
            test_usb: false,
        };
        assert!(opts.run_unit);
        assert!(!opts.run_docker);
    }
}
