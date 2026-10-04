//! Linux Kernel DRM/KMS Direct Hardware Scanout Capture Module
//!
//! Direct interaction with Linux Kernel DRM/KMS (/dev/dri/card*) subsystem using
//! safe low-level ioctls. Bypasses Wayland, GNOME Mutter, and D-Bus completely,
//! eliminating window occlusion tracking and damage tracking freeze bugs.
//!
//! Supports all GPU vendors: AMD (amdgpu), Intel (i915/xe), NVIDIA (nvidia-drm), and VKMS.
//! Distinguishes between internal laptop panels (eDP/LVDS) and external extended displays (HDMI/DP).
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use drm::control::Device as ControlDevice;
use drm::Device;
use std::fs::{self, File, OpenOptions};
use std::os::fd::{AsFd, BorrowedFd};
use std::path::{Path, PathBuf};

/// Safe wrapper for a Linux DRM device file
pub struct DrmCard(pub File);

impl AsFd for DrmCard {
    /// Executes `as_fd` operational routine.
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for DrmCard {}
impl ControlDevice for DrmCard {}

/// Information regarding an active DRM display output
#[derive(Debug, Clone)]
pub struct KmsOutputInfo {
    pub card_path: PathBuf,
    pub render_node: PathBuf,
    pub connector_name: String,
    pub interface_name: String,
    pub is_internal: bool,
    pub crtc_id: u32,
    pub width: u32,
    pub height: u32,
    pub vrefresh: u32,
}

impl KmsOutputInfo {
    /// Lists all active connected display outputs across all DRM cards
    pub fn list_available() -> Vec<Self> {
        let entries = match fs::read_dir("/dev/dri") {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

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

        let mut all_outputs: Vec<KmsOutputInfo> = Vec::new();

        for card_path in &cards {
            if let Ok(info_list) = Self::inspect_card(card_path) {
                all_outputs.extend(info_list);
            }
        }

        all_outputs
    }

    /// Discovers the target display output across all `/dev/dri/card*` devices.
    ///
    /// If `target_connector` is "auto" or contains "extend"/"hdmi", selects the first active
    /// external display (HDMI, DP). If "clone" or "edp", selects the internal laptop panel.
    pub fn discover(target_connector: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let all_outputs = Self::list_available();

        if all_outputs.is_empty() {
            return Err("Nenhum monitor ativo conectado encontrado no subsistema DRM/KMS".into());
        }

        let target_lower = target_connector.to_lowercase();

        // 1. Numeric index lookup (e.g. "0", "1")
        if let Ok(idx) = target_connector.parse::<usize>() {
            if let Some(out) = all_outputs.get(idx) {
                println!(
                    "\x1b[1;32m[+] DRM/KMS Match por Índice [{}]:\x1b[0m {} [{}] no CRTC {} ({}x{}@{}Hz, Card: {:?}, Render: {:?})",
                    idx, out.connector_name, if out.is_internal { "Interno" } else { "Externo" },
                    out.crtc_id, out.width, out.height, out.vrefresh, out.card_path, out.render_node
                );
                return Ok(out.clone());
            }
        }

        // 2. Se pediu explicitamente HDMI ou modo estendido: BUSCAR EXTERNO HDMI/DP
        if target_lower.contains("hdmi") || target_lower.contains("extend") {
            for out in &all_outputs {
                if !out.is_internal {
                    let name_lower = out.connector_name.to_lowercase();
                    let iface_lower = out.interface_name.to_lowercase();
                    if name_lower.contains("hdmi") || iface_lower.contains("hdmi") || name_lower.contains("dp") || iface_lower.contains("dp") {
                        println!(
                            "\x1b[1;32m[+] DRM/KMS Match Segunda Tela (Estendida):\x1b[0m {} (Iface: {}) no CRTC {} ({}x{}@{}Hz, Card: {:?}, Render: {:?})",
                            out.connector_name, out.interface_name, out.crtc_id, out.width, out.height, out.vrefresh, out.card_path, out.render_node
                        );
                        return Ok(out.clone());
                    }
                }
            }
        }

        // 3. Se pediu explicitamente eDP / Clone da tela interna
        if target_lower.contains("edp") || target_lower.contains("clone") || target_lower.contains("intern") {
            for out in &all_outputs {
                if out.is_internal {
                    println!(
                        "\x1b[1;32m[+] DRM/KMS Match Tela Interna do Notebook (Clone):\x1b[0m {} no CRTC {} ({}x{}@{}Hz, Card: {:?}, Render: {:?})",
                        out.connector_name, out.crtc_id, out.width, out.height, out.vrefresh, out.card_path, out.render_node
                    );
                    return Ok(out.clone());
                }
            }
        }

        // 4. Match por nome específico
        if target_connector != "auto" {
            let norm_target = target_connector.to_lowercase().replace("-", "").replace("_", "");
            for out in &all_outputs {
                let norm_name = out.connector_name.to_lowercase().replace("-", "").replace("_", "");
                let norm_iface = out.interface_name.to_lowercase().replace("-", "").replace("_", "");
                if norm_name.contains(&norm_target) || norm_target.contains(&norm_name)
                    || norm_iface.contains(&norm_target) || norm_target.contains(&norm_iface) {
                    println!(
                        "\x1b[1;32m[+] DRM/KMS Match por Nome:\x1b[0m {} no CRTC {} ({}x{}@{}Hz, Card: {:?}, Render: {:?})",
                        out.connector_name, out.crtc_id, out.width, out.height, out.vrefresh, out.card_path, out.render_node
                    );
                    return Ok(out.clone());
                }
            }
        }

        // 5. Auto selection: SEMPRE prioriza monitor EXTERNO (HDMI, DisplayPort) para extensão
        for out in &all_outputs {
            if !out.is_internal {
                println!(
                    "\x1b[1;32m[+] DRM/KMS Auto-Select (Monitor Externo Estendido):\x1b[0m {} (Iface: {}) no CRTC {} ({}x{}@{}Hz, Card: {:?}, Render: {:?})",
                    out.connector_name, out.interface_name, out.crtc_id, out.width, out.height, out.vrefresh, out.card_path, out.render_node
                );
                return Ok(out.clone());
            }
        }

        // 6. Fallback final para tela interna
        let chosen = all_outputs[0].clone();
        println!(
            "\x1b[1;33m[*] DRM/KMS Fallback (Monitor Primário):\x1b[0m {} no CRTC {} ({}x{}@{}Hz, Card: {:?}, Render: {:?})",
            chosen.connector_name, chosen.crtc_id, chosen.width, chosen.height, chosen.vrefresh, chosen.card_path, chosen.render_node
        );
        Ok(chosen)
    }

    /// Executes `find_render_node` operational routine.
    fn find_render_node(card_path: &Path) -> PathBuf {
        let card_name = card_path.file_name().and_then(|n| n.to_str()).unwrap_or("card0");
        let sys_path = PathBuf::from(format!("/sys/class/drm/{}/device/drm", card_name));
        
        if let Ok(entries) = fs::read_dir(sys_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("renderD") {
                    let candidate = PathBuf::from(format!("/dev/dri/{}", name));
                    if candidate.exists() {
                        return candidate;
                    }
                }
            }
        }

        let default_render = PathBuf::from("/dev/dri/renderD128");
        if default_render.exists() {
            default_render
        } else {
            card_path.to_path_buf()
        }
    }

    /// Executes `inspect_card` operational routine.
    fn inspect_card(path: &Path) -> Result<Vec<KmsOutputInfo>, Box<dyn std::error::Error>> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let card = DrmCard(file);

        let res = card.resource_handles()?;
        let render_node = Self::find_render_node(path);
        let mut outputs = Vec::new();

        for &conn_handle in res.connectors() {
            if let Ok(conn) = card.get_connector(conn_handle, true) {
                if conn.state() == drm::control::connector::State::Connected {
                    let iface = conn.interface();
                    
                    // Classifica se é conector interno (tela do laptop) ou externo (HDMI, DP, etc.)
                    let (friendly_name, is_internal) = match iface {
                        drm::control::connector::Interface::EmbeddedDisplayPort => ("eDP-1".to_string(), true),
                        drm::control::connector::Interface::LVDS => ("LVDS-1".to_string(), true),
                        drm::control::connector::Interface::DSI => ("DSI-1".to_string(), true),
                        drm::control::connector::Interface::HDMIA => ("HDMI-1".to_string(), false),
                        drm::control::connector::Interface::HDMIB => ("HDMI-2".to_string(), false),
                        drm::control::connector::Interface::DisplayPort => ("DP-1".to_string(), false),
                        _ => (format!("{:?}-{:?}", iface, conn_handle), false),
                    };

                    let raw_iface_name = format!("{:?}", iface);

                    if let Some(enc_handle) = conn.current_encoder() {
                        if let Ok(enc) = card.get_encoder(enc_handle) {
                            if let Some(crtc_handle) = enc.crtc() {
                                if let Ok(crtc) = card.get_crtc(crtc_handle) {
                                    let (w, h, rate) = if let Some(mode) = crtc.mode() {
                                        (mode.size().0 as u32, mode.size().1 as u32, mode.vrefresh())
                                    } else {
                                        (1280, 720, 60)
                                    };

                                    let crtc_raw_id = unsafe {
                                        std::mem::transmute::<drm::control::crtc::Handle, u32>(crtc_handle)
                                    };

                                    outputs.push(KmsOutputInfo {
                                        card_path: path.to_path_buf(),
                                        render_node: render_node.clone(),
                                        connector_name: friendly_name,
                                        interface_name: raw_iface_name,
                                        is_internal,
                                        crtc_id: crtc_raw_id,
                                        width: w,
                                        height: h,
                                        vrefresh: rate,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(outputs)
    }
}
