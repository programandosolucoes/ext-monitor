//! Interactive Selection Dialog for Google Chrome Cast
//!
//! Provides a prominent modal selection dialog when Google Cast starts from Chrome:
//! - Extend (Desktop Extension on HDMI-1 TV)
//! - Clone (Screen Mirror on eDP-1 Notebook)
//! - Window (Application Window via Web Caster / GNOME Window Picker)
//! - Tab (Browser Tab via Web Caster /cast)
//! - Cancel (Aborts transmission without modifying existing state)
//!
//! Priority:
//! 1. Uses `zenity` (if installed) to display a focused, persistent modal window with
//!    a 30-second timeout, explicit Cancel button, and priority over background windows.
//! 2. Falls back to FreeDesktop D-Bus notification (`org.freedesktop.Notifications.Notify`)
//!    with critical urgency if zenity is not present.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use zbus::blocking::{Connection, MessageIterator};
use zbus::MatchRule;
use zbus::zvariant::Value;

/// Displays the interactive modal dialog to select Chrome Cast stream mode.
/// Returns Some("extend" | "clone" | "window" | "tab") on success, or None on cancel / timeout.
pub fn prompt_user_mode_selection() -> Option<String> {
    // 1. Try modal dialog via Zenity first (avoids passive notification auto-dismissal in GNOME)
    if let Some(choice) = prompt_user_mode_selection_zenity() {
        return Some(choice);
    }

    // 2. Fall back to native D-Bus FreeDesktop Notification
    prompt_user_mode_selection_native()
}

/// Displays a modal selection dialog using `zenity` with 30s timeout and Cancel button
pub fn prompt_user_mode_selection_zenity() -> Option<String> {
    let output = Command::new("zenity")
        .args([
            "--list",
            "--radiolist",
            "--title=RaspCast: Modo de Transmissão",
            "--text=Google Chrome Cast: Selecione como deseja transmitir para a TV:",
            "--column=",
            "--column=ID",
            "--column=Opção de Transmissão",
            "TRUE", "extend", "🖥️ Estender Área de Trabalho (HDMI-1 TV)",
            "FALSE", "clone", "💻 Espelhar Tela do Laptop (eDP-1)",
            "FALSE", "window", "🪟 Transmitir Janela de Aplicativo",
            "FALSE", "tab", "🌐 Transmitir Aba do Navegador",
            "--hide-column=2",
            "--cancel-label=Cancelar",
            "--ok-label=Confirmar",
            "--timeout=30",
            "--width=500",
            "--height=290",
            "--modal",
        ])
        .env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string()))
        .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()))
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let choice = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !choice.is_empty() {
                    println!("\x1b[1;32m[dialog-zenity]\x1b[0m Usuário selecionou modo: '{}'\x1b[0m", choice);
                    return Some(choice);
                }
            }
            // Status code 1 = user cancelled; Status code 5 = timeout
            println!("\x1b[1;33m[dialog-zenity]\x1b[0m Diálogo cancelado ou timeout de 30s atingido.\x1b[0m");
            None
        }
        Err(e) => {
            eprintln!("\x1b[1;33m[dialog-zenity]\x1b[0m Zenity não disponível ({}). Usando D-Bus...\x1b[0m", e);
            None
        }
    }
}

/// Fallback: FreeDesktop D-Bus Notification with Critical Urgency
pub fn prompt_user_mode_selection_native() -> Option<String> {
    let conn = match Connection::session() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\x1b[1;33m[dialog-rust]\x1b[0m Não foi possível conectar ao D-Bus de sessão: {}\x1b[0m", e);
            return None;
        }
    };

    let actions = vec![
        "extend", "🖥️ Estender Área de Trabalho (HDMI-1)",
        "clone",  "💻 Espelhar Laptop (eDP-1)",
        "window", "🪟 Transmitir Janela de App",
        "tab",    "🌐 Transmitir Aba do Navegador",
        "cancel", "❌ Cancelar / Fechar",
    ];

    let mut hints: HashMap<&str, Value> = HashMap::new();
    hints.insert("urgency", Value::from(2u8));
    hints.insert("resident", Value::from(true));
    hints.insert("transient", Value::from(false));
    let expire_timeout_ms: i32 = 45000;

    let rule = match MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.Notifications")
    {
        Ok(b) => match b.build() {
            r => r,
        },
        Err(e) => {
            eprintln!("\x1b[1;33m[dialog-rust]\x1b[0m Falha ao criar MatchRule D-Bus: {}\x1b[0m", e);
            return None;
        }
    };

    let iter = match MessageIterator::for_match_rule(rule, &conn, Some(16)) {
        Ok(it) => it,
        Err(e) => {
            eprintln!("\x1b[1;33m[dialog-rust]\x1b[0m Falha ao criar MessageIterator: {}\x1b[0m", e);
            return None;
        }
    };

    let reply = match conn.call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        &(
            "RaspCast",
            0u32,
            "video-display",
            "RaspCast: Como deseja transmitir?",
            "Selecione o modo de transmissão para a TV:",
            actions,
            hints,
            expire_timeout_ms,
        ),
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("\x1b[1;33m[dialog-rust]\x1b[0m Falha ao chamar Notify: {}\x1b[0m", e);
            return None;
        }
    };

    let notif_id: u32 = match reply.body().deserialize() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("\x1b[1;33m[dialog-rust]\x1b[0m Falha ao desserializar ID da notificação: {}\x1b[0m", e);
            return None;
        }
    };

    println!("\x1b[1;36m[dialog-rust]\x1b[0m Notificação interativa exibida (ID: {}). Aguardando escolha do usuário...\x1b[0m", notif_id);

    let (tx, rx) = mpsc::channel();
    let notif_target = notif_id;

    thread::spawn(move || {
        for msg_res in iter {
            if let Ok(msg) = msg_res {
                if let Some(member) = msg.header().member() {
                    let member_name = member.as_str();
                    if member_name == "ActionInvoked" {
                        if let Ok((id, action_key)) = msg.body().deserialize::<(u32, String)>() {
                            if id == notif_target {
                                let _ = tx.send(Some(action_key));
                                return;
                            }
                        }
                    } else if member_name == "NotificationClosed" {
                        if let Ok((id, _reason)) = msg.body().deserialize::<(u32, u32)>() {
                            if id == notif_target {
                                let _ = tx.send(None);
                                return;
                            }
                        }
                    }
                }
            }
        }
    });

    match rx.recv_timeout(Duration::from_secs(45)) {
        Ok(Some(action)) => {
            if action == "cancel" {
                println!("\x1b[1;33m[dialog-rust]\x1b[0m Usuário clicou em Cancelar. Transmissão cancelada sem alterações.\x1b[0m");
                None
            } else {
                println!("\x1b[1;32m[dialog-rust]\x1b[0m Usuário selecionou modo: '{}'\x1b[0m", action);
                Some(action)
            }
        }
        Ok(None) => {
            println!("\x1b[1;33m[dialog-rust]\x1b[0m Notificação descartada pelo usuário. Transmissão cancelada.\x1b[0m");
            None
        }
        Err(_) => {
            println!("\x1b[1;33m[dialog-rust]\x1b[0m Tempo limite expirado (45s). Transmissão não iniciada.\x1b[0m");
            None
        }
    }
}
