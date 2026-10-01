//! Systemd User Service and Single-Instance Lock Manager for ext-receiver
//!
//! Enables ext-receiver to run on non-Raspberry Pi machines (x86 Linux desktops/laptops)
//! as a background daemon, preventing multiple instances and port conflicts.
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

pub fn get_pid_file() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("ext-monitor-receiver.pid")
    } else {
        let uid = unsafe { libc::getuid() };
        std::env::temp_dir().join(format!("ext-monitor-receiver-{}.pid", uid))
    }
}

pub fn get_service_file() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("systemd")
            .join("user")
            .join("ext-monitor-receiver.service")
    })
}

pub fn get_running_pid() -> Option<u32> {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        return None;
    }

    let content = fs::read_to_string(&pid_file).ok()?;
    let pid: u32 = content.trim().parse().ok()?;

    let is_alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
    if !is_alive {
        let _ = fs::remove_file(&pid_file);
        return None;
    }

    let cmdline_path = format!("/proc/{}/cmdline", pid);
    if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
        if cmdline.contains("ext-receiver") {
            return Some(pid);
        }
    }

    let _ = fs::remove_file(&pid_file);
    None
}

pub fn acquire_lock() -> Result<PidLock, String> {
    if let Some(existing_pid) = get_running_pid() {
        let my_pid = std::process::id();
        if existing_pid != my_pid {
            return Err(format!("Instância do ext-receiver já em execução com PID {}", existing_pid));
        }
    }

    let pid_file = get_pid_file();
    let my_pid = std::process::id();
    if let Err(e) = fs::write(&pid_file, format!("{}\n", my_pid)) {
        return Err(format!("Falha ao gravar PID lockfile {:?}: {}", pid_file, e));
    }

    Ok(PidLock { path: pid_file })
}

pub fn is_service_installed() -> bool {
    get_service_file().map(|p| p.exists()).unwrap_or(false)
}

pub fn install_service() -> Result<PathBuf, String> {
    let service_file = get_service_file().ok_or("Variável $HOME não definida.")?;
    let parent = service_file.parent().ok_or("Caminho inválido para systemd user.")?;
    fs::create_dir_all(parent).map_err(|e| format!("Falha ao criar diretório {:?}: {}", parent, e))?;

    let binary_path = if PathBuf::from("/usr/local/bin/ext-receiver").exists() {
        "/usr/local/bin/ext-receiver".to_string()
    } else if let Ok(current) = std::env::current_exe() {
        current.to_string_lossy().to_string()
    } else {
        "/usr/local/bin/ext-receiver".to_string()
    };

    let unit_content = format!(
        r#"[Unit]
Description=Ext-Monitor Universal Display Receiver
Documentation=https://github.com/carlosalberto4ti/ext-monitor
After=network.target sound.target

[Service]
Type=simple
ExecStart={} --service-daemon
Restart=on-failure
RestartSec=2s
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
"#,
        binary_path
    );

    fs::write(&service_file, unit_content)
        .map_err(|e| format!("Falha ao escrever arquivo de serviço {:?}: {}", service_file, e))?;

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).output();
    let _ = Command::new("systemctl").args(["--user", "enable", "ext-monitor-receiver.service"]).output();

    Ok(service_file)
}

pub fn start_service() -> Result<(), String> {
    if !is_service_installed() {
        println!("\x1b[1;34m[*] Serviço local de usuário não encontrado. Instalando automaticamente...\x1b[0m");
        let path = install_service()?;
        println!("\x1b[1;32m[+] Serviço criado em {:?}\x1b[0m", path);
    }

    let out = Command::new("systemctl")
        .args(["--user", "restart", "ext-monitor-receiver.service"])
        .output()
        .map_err(|e| format!("Falha ao invocar systemctl: {}", e))?;

    if out.status.success() {
        println!("\x1b[1;32m[+] ext-receiver iniciado como serviço em background (systemd --user)!\x1b[0m");
        println!("\x1b[1;36m    Painel Web: http://localhost:8080\x1b[0m");
        Ok(())
    } else {
        Err(format!("Erro ao iniciar serviço: {}", String::from_utf8_lossy(&out.stderr)))
    }
}

pub fn stop_all() -> Result<(), String> {
    let _ = Command::new("systemctl").args(["--user", "stop", "ext-monitor-receiver.service"]).output();

    if let Some(pid) = get_running_pid() {
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

    let pid_file = get_pid_file();
    if pid_file.exists() {
        let _ = fs::remove_file(&pid_file);
    }

    println!("\x1b[1;32m[+] ext-receiver finalizado com sucesso.\x1b[0m");
    Ok(())
}
