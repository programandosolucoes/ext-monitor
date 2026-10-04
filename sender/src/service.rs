//! Systemd User Service and Single-Instance Lock Manager for ext-sender
//!
//! Provides daemonization, automatic background execution via `systemd --user`,
//! PID file locking, and single-instance process enforcement without root.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct PidLock {
    path: PathBuf,
}

impl Drop for PidLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Returns the path to the user-scoped PID lockfile
pub fn get_pid_file() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("ext-monitor-sender.pid")
    } else {
        let uid = unsafe { libc::getuid() };
        std::env::temp_dir().join(format!("ext-monitor-sender-{}.pid", uid))
    }
}

/// Returns the path to the systemd user service file (~/.config/systemd/user/ext-monitor-sender.service)
pub fn get_service_file() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("systemd")
            .join("user")
            .join("ext-monitor-sender.service")
    })
}

/// Checks if an active ext-sender process is currently running
pub fn get_running_pid() -> Option<u32> {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        return None;
    }

    let content = fs::read_to_string(&pid_file).ok()?;
    let pid: u32 = content.trim().parse().ok()?;

    // Check if process exists via kill(pid, 0)
    let is_alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
    if !is_alive {
        let _ = fs::remove_file(&pid_file);
        return None;
    }

    // Verify process is actually ext-sender
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
        if cmdline.contains("ext-sender") {
            return Some(pid);
        }
    }

    // Stale PID or different process
    let _ = fs::remove_file(&pid_file);
    None
}

/// Acquires single-instance lock file for the current process
pub fn acquire_lock() -> Result<PidLock, String> {
    if let Some(existing_pid) = get_running_pid() {
        let my_pid = std::process::id();
        if existing_pid != my_pid {
            return Err(format!("Instância já em execução com PID {}", existing_pid));
        }
    }

    let pid_file = get_pid_file();
    let my_pid = std::process::id();
    if let Err(e) = fs::write(&pid_file, format!("{}\n", my_pid)) {
        return Err(format!("Falha ao gravar PID lockfile {:?}: {}", pid_file, e));
    }

    Ok(PidLock { path: pid_file })
}

/// Checks if systemd user service unit exists
pub fn is_service_installed() -> bool {
    get_service_file().map(|p| p.exists()).unwrap_or(false)
}

/// Installs the ext-monitor-sender service for the current user (systemd --user)
pub fn install_service() -> Result<PathBuf, String> {
    let service_file = get_service_file().ok_or("Variável $HOME não definida.")?;
    let parent = service_file.parent().ok_or("Caminho inválido para systemd user.")?;
    fs::create_dir_all(parent).map_err(|e| format!("Falha ao criar diretório {:?}: {}", parent, e))?;

    // Determine binary path: prefer /usr/local/bin/ext-sender if available
    let binary_path = if PathBuf::from("/usr/local/bin/ext-sender").exists() {
        "/usr/local/bin/ext-sender".to_string()
    } else if let Ok(current) = std::env::current_exe() {
        current.to_string_lossy().to_string()
    } else {
        "/usr/local/bin/ext-sender".to_string()
    };

    let unit_content = format!(
        r#"[Unit]
Description=Ext-Monitor Universal Virtual Second Screen Sender
Documentation=https://github.com/carlosalberto4ti/ext-monitor
After=graphical-session.target pipewire.service
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart={} --service-daemon
Restart=on-failure
RestartSec=2s
StandardOutput=journal
StandardError=journal
Environment=RUST_LOG=info
Environment=WAYLAND_DISPLAY=wayland-0
Environment=DISPLAY=:0
Environment=XDG_RUNTIME_DIR=%t

[Install]
WantedBy=graphical-session.target
"#,
        binary_path
    );

    fs::write(&service_file, unit_content)
        .map_err(|e| format!("Falha ao escrever arquivo de serviço {:?}: {}", service_file, e))?;

    // Reload systemd user daemon and enable service
    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).output();
    let _ = Command::new("systemctl").args(["--user", "enable", "ext-monitor-sender.service"]).output();

    Ok(service_file)
}

/// Uninstalls the systemd user service
pub fn uninstall_service() -> Result<(), String> {
    let _ = Command::new("systemctl").args(["--user", "stop", "ext-monitor-sender.service"]).output();
    let _ = Command::new("systemctl").args(["--user", "disable", "ext-monitor-sender.service"]).output();

    if let Some(service_file) = get_service_file() {
        if service_file.exists() {
            let _ = fs::remove_file(&service_file);
        }
    }

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).output();
    Ok(())
}

/// Starts the background user service
pub fn start_service() -> Result<(), String> {
    if !is_service_installed() {
        println!("\x1b[1;34m[*] Serviço local de usuário não encontrado. Instalando automaticamente...\x1b[0m");
        let path = install_service()?;
        println!("\x1b[1;32m[+] Serviço criado em {:?}\x1b[0m", path);
    }

    let out = Command::new("systemctl")
        .args(["--user", "restart", "ext-monitor-sender.service"])
        .output()
        .map_err(|e| format!("Falha ao invocar systemctl: {}", e))?;

    if out.status.success() {
        println!("\x1b[1;32m[+] ext-sender iniciado com sucesso como serviço em background (systemd --user)!\x1b[0m");
        println!("\x1b[1;36m    Controle Web: http://192.168.7.2:8080\x1b[0m");
        println!("\x1b[1;36m    Ver logs:     ext-sender logs\x1b[0m");
        println!("\x1b[1;36m    Status:       ext-sender status\x1b[0m");
        println!("\x1b[1;36m    Parar:        ext-sender stop\x1b[0m");
        Ok(())
    } else {
        Err(format!(
            "Erro ao iniciar serviço: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

/// Stops the service and any orphan streaming instances
pub fn stop_all() -> Result<(), String> {
    println!("\x1b[1;33m[*] Parando serviço e processos do ext-sender...\x1b[0m");

    // 1. Stop systemd service if active
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "ext-monitor-sender.service"])
        .output();

    // 2. Kill PID lock if still running
    if let Some(pid) = get_running_pid() {
        println!("\x1b[1;33m[*] Encerrando processo ativo PID {}...\x1b[0m", pid);
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGTERM);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        if unsafe { libc::kill(pid as libc::pid_t, 0) == 0 } {
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
    }

    // 3. Remove PID file
    let pid_file = get_pid_file();
    if pid_file.exists() {
        let _ = fs::remove_file(&pid_file);
    }

    // 4. Clean up any leftover gst-launch-1.0 from our session
    let _ = Command::new("killall").args(["-9", "gst-launch-1.0"]).output();

    println!("\x1b[1;32m[+] ext-sender finalizado com sucesso.\x1b[0m");
    Ok(())
}

/// Prints current service and process status
pub fn show_status() {
    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  Status do Transmissor ext-sender (Host Linux Wayland)                 \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m");

    // Process status
    if let Some(pid) = get_running_pid() {
        println!("\x1b[1;32m  [●] Processo: ATIVO (PID: {})\x1b[0m", pid);
    } else {
        println!("\x1b[1;33m  [○] Processo: INATIVO (nenhuma instância em execução)\x1b[0m");
    }

    // Systemd service status
    let is_installed = is_service_installed();
    print!("  Serviço systemd --user: ");
    if is_installed {
        let out = Command::new("systemctl")
            .args(["--user", "is-active", "ext-monitor-sender.service"])
            .output();
        let state = out
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "desconhecido".to_string());

        if state == "active" {
            println!("\x1b[1;32mATIVO (rodando em background)\x1b[0m");
        } else {
            println!("\x1b[1;33mINSTALADO ({})\x1b[0m", state);
        }
    } else {
        println!("\x1b[1;31mNÃO INSTALADO\x1b[0m (será instalado automaticamente no primeiro start)");
    }

    // Appliance endpoint telemetry
    println!("\n\x1b[1;34m[*] Verificando telemetria do Pi Zero em http://192.168.7.2:8080/api/status...\x1b[0m");
    match crate::http_client::get("http://192.168.7.2:8080/api/status") {
        Ok(body) if !body.is_empty() => {
            println!("\x1b[1;32m[+] Conexão com Pi Zero OK:\x1b[0m\n{}", body);
        }
        _ => {
            println!("\x1b[1;33m[!] Pi Zero não respondeu em 192.168.7.2:8080 (verifique se o cabo USB está conectado)\x1b[0m");
        }
    }
}

/// Follows live logs of the user service via journalctl
pub fn follow_logs() {
    println!("\x1b[1;34m[*] Acompanhando logs em tempo real (Ctrl+C para sair)...\x1b[0m");
    let _ = Command::new("journalctl")
        .args(["--user", "-u", "ext-monitor-sender.service", "-f", "-n", "50"])
        .status();
}
