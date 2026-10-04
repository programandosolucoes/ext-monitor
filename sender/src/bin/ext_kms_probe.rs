//! Diagnostic CLI tool to probe Linux Kernel DRM/KMS outputs and PRIME DMA-BUF export capability
//!
//! Scans all /dev/dri/card* devices, resolves active connectors, CRTCs, framebuffers,
//! and tests hardware scanout PRIME DMA-BUF export.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use drm::control::Device as ControlDevice;
use drm::Device;
use std::fs::{self, OpenOptions};
use std::os::fd::{AsFd, BorrowedFd};
use std::path::PathBuf;

#[path = "../kms.rs"]
mod kms;

use kms::KmsOutputInfo;

/// Represents Card configuration and operational state.
struct Card(std::fs::File);

impl AsFd for Card {
    /// Executes `as_fd` operational routine.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

/// Application entrypoint initializing the runtime environment and dispatching execution.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m========================================================================\x1b[0m");
    println!("\x1b[1;36m  ext-kms-probe: Scanner de Diagnóstico de Hardware DRM/KMS Linux       \x1b[0m");
    println!("\x1b[1;36m========================================================================\x1b[0m");

    let entries = fs::read_dir("/dev/dri")?;
    let mut cards: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with("card") && s[4..].chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        })
        .collect();

    cards.sort();

    if cards.is_empty() {
        println!("\x1b[1;31m[!] Nenhum dispositivo /dev/dri/card* encontrado.\x1b[0m");
        return Ok(());
    }

    for card_path in &cards {
        println!("\n\x1b[1;34m[*] Inspecionando:\x1b[0m {:?}", card_path);
        let file = match OpenOptions::new().read(true).write(true).open(card_path) {
            Ok(f) => f,
            Err(e) => {
                println!("   \x1b[1;31m[!] Falha ao abrir: {}\x1b[0m", e);
                continue;
            }
        };

        let card = Card(file);
        let res = match card.resource_handles() {
            Ok(r) => r,
            Err(e) => {
                println!("   \x1b[1;31m[!] Falha ao ler resource handles: {}\x1b[0m", e);
                continue;
            }
        };

        for &conn_handle in res.connectors() {
            if let Ok(conn) = card.get_connector(conn_handle, true) {
                let state_str = if conn.state() == drm::control::connector::State::Connected {
                    "\x1b[1;32mCONECTADO\x1b[0m"
                } else {
                    "\x1b[1;30mDESCONECTADO\x1b[0m"
                };

                println!("   - Conector: {:?}-{:?} [{}]", conn.interface(), conn_handle, state_str);

                if conn.state() == drm::control::connector::State::Connected {
                    if let Some(enc_handle) = conn.current_encoder() {
                        if let Ok(enc) = card.get_encoder(enc_handle) {
                            if let Some(crtc_handle) = enc.crtc() {
                                if let Ok(crtc) = card.get_crtc(crtc_handle) {
                                    let (w, h, rate) = if let Some(mode) = crtc.mode() {
                                        (mode.size().0, mode.size().1, mode.vrefresh())
                                    } else {
                                        (0, 0, 0)
                                    };
                                    println!("     CRTC: {:?} | Modo: {}x{} @ {}Hz", crtc_handle, w, h, rate);

                                    if let Some(fb_handle) = crtc.framebuffer() {
                                        match card.get_planar_framebuffer(fb_handle) {
                                            Ok(planar) => {
                                                println!("     FB Handle: {:?} ({}x{}, Formato: {:?}, Buffers len: {})", fb_handle, planar.size().0, planar.size().1, planar.pixel_format(), planar.buffers().len());
                                                for (i, buf_opt) in planar.buffers().iter().enumerate() {
                                                    println!("       Buffer [{}]: {:?}", i, buf_opt);
                                                    if let Some(buf_handle) = buf_opt {
                                                        match card.buffer_to_prime_fd(*buf_handle, 0) {
                                                            Ok(fd) => {
                                                                println!("     \x1b[1;32m[+] Plano {} PRIME DMA-BUF FD: {:?}\x1b[0m", i, fd);
                                                            }
                                                            Err(e) => {
                                                                println!("     \x1b[1;33m[-] Plano {} buffer_to_prime_fd: {}\x1b[0m", i, e);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                println!("     \x1b[1;33m[-] get_planar_framebuffer falhou: {}\x1b[0m", e);
                                                if let Ok(legacy) = card.get_framebuffer(fb_handle) {
                                                    println!("     FB Legacy: {:?} ({}x{}, pitch: {}, bpp: {})", fb_handle, legacy.size().0, legacy.size().1, legacy.pitch(), legacy.bpp());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\n\x1b[1;32m[+] Varredura DRM/KMS concluída.\x1b[0m");

    println!("\n\x1b[1;36m=== Teste de Resolução de Monitores ===\x1b[0m");
    println!("1. Teste para Segunda Tela Estendida ('HDMI-1'):");
    let _ = KmsOutputInfo::discover("HDMI-1");
    println!("\n2. Teste para Tela Clonada ('eDP-1'):");
    let _ = KmsOutputInfo::discover("eDP-1");
    println!("\n3. Teste para Auto-Select ('auto'):");
    let _ = KmsOutputInfo::discover("auto");

    Ok(())
}
