//! Appliance IoT Media Renderer, Google Cast / UPnP SSDP & HDMI Visualizer Engine
//!
//! Transforms Raspberry Pi Zero W into an intelligent IoT media device for TV:
//! 1. Animated Spectrum & Cover Art Visualizer on /dev/fb0 (30 FPS VU Meter).
//! 2. UPnP / DLNA MediaRenderer SSDP advertiser on UDP 1900.
//! 3. DIAL (Discovery and Launch) / YouTube Cast descriptor.
//! 4. Background metadata tracking for Bluetooth A2DP (AVRCP) and network audio.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs::OpenOptions;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const FB_WIDTH: usize = 1280;
const FB_HEIGHT: usize = 720;
const FB_SIZE: usize = FB_WIDTH * FB_HEIGHT * 2; // 1,843,200 bytes (RGB565)

// RGB565 Helper Function
#[inline(always)]
pub const fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | ((b as u16) >> 3)
}

#[derive(Debug, Clone)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source: String, // "bluetooth", "upnp", "dial", "network"
    pub state: String,  // "playing", "paused", "idle"
    pub volume: u32,
    pub visualizer_enabled: bool,
}

static GLOBAL_MEDIA_TRACK: OnceLock<Arc<Mutex<MediaTrackInfo>>> = OnceLock::new();

pub fn get_media_state() -> Arc<Mutex<MediaTrackInfo>> {
    GLOBAL_MEDIA_TRACK
        .get_or_init(|| {
            Arc::new(Mutex::new(MediaTrackInfo {
                title: "Ext-Monitor IoT Audio".to_string(),
                artist: "Conecte seu celular via Bluetooth ou UPnP".to_string(),
                album: "Pronto para Transmitir (Cast)".to_string(),
                source: "bluetooth".to_string(),
                state: "idle".to_string(),
                volume: 100,
                visualizer_enabled: true,
            }))
        })
        .clone()
}

/// Start the background SSDP / UPnP multicast responder
/// Start the background SSDP / UPnP multicast responder
pub fn start_ssdp_responder(running: Arc<AtomicBool>, http_port: u16) {
    thread::Builder::new()
        .name("ssdp-responder".to_string())
        .spawn(move || {
            // Ensure multicast route exists on Linux
            let _ = std::process::Command::new("route")
                .args(&["add", "-net", "224.0.0.0", "netmask", "240.0.0.0", "dev", "usb0"])
                .output();

            let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);

            // Bind using libc with SO_REUSEADDR and SO_REUSEPORT to prevent port lockups
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            if fd < 0 {
                eprintln!("\x1b[1;33m[ssdp]\x1b[0m Failed to create raw UDP socket for SSDP.");
                return;
            }

            let opt: libc::c_int = 1;
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEADDR,
                    &opt as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&opt) as libc::socklen_t,
                );
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    &opt as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&opt) as libc::socklen_t,
                );

                let mut addr: libc::sockaddr_in = std::mem::zeroed();
                addr.sin_family = libc::AF_INET as libc::sa_family_t;
                addr.sin_port = (1900u16).to_be();
                addr.sin_addr.s_addr = libc::INADDR_ANY;

                if libc::bind(
                    fd,
                    &addr as *const _ as *const libc::sockaddr,
                    std::mem::size_of_val(&addr) as libc::socklen_t,
                ) != 0
                {
                    libc::close(fd);
                    eprintln!("\x1b[1;33m[ssdp]\x1b[0m Failed to bind UDP 1900. Running without SSDP responder.");
                    return;
                }
            }

            let socket = unsafe { UdpSocket::from_raw_fd(fd) };
            let _ = socket.set_broadcast(true);
            let _ = socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::new(192, 168, 7, 2));
            let _ = socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED);
            let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

            println!("\x1b[1;32m[ssdp]\x1b[0m UPnP / DLNA / DIAL MediaRenderer SSDP daemon active on 239.255.255.250:1900");

            let notify_targets = [
                SocketAddr::from(([239, 255, 255, 250], 1900)),
                SocketAddr::from(([192, 168, 7, 1], 1900)),
                SocketAddr::from(([192, 168, 7, 255], 1900)),
            ];

            let mut buf = [0u8; 2048];
            let mut last_notify = std::time::Instant::now();

            // Initial announcement burst
            send_ssdp_announcements(&socket, &notify_targets, http_port);

            while running.load(Ordering::Relaxed) {
                // 1. Send periodic NOTIFY packets (every 5 seconds)
                if last_notify.elapsed() >= Duration::from_secs(5) {
                    last_notify = std::time::Instant::now();
                    send_ssdp_announcements(&socket, &notify_targets, http_port);
                }

                // 2. Receive and handle M-SEARCH queries
                if let Ok((len, src)) = socket.recv_from(&mut buf) {
                    let req = String::from_utf8_lossy(&buf[..len]);
                    if let Some((loc_path, st)) = parse_ssdp_msearch(&req) {
                        let is_dial = st.contains("dial");
                        let usn = if is_dial {
                            format!("uuid:ext-monitor-dial-device::{}", st)
                        } else {
                            format!("uuid:ext-monitor-bcm2835-renderer::{}", st)
                        };
                        let app_url_header = if is_dial {
                            format!("APPLICATION-URL: http://192.168.7.2:{}/apps/\r\n", http_port)
                        } else {
                            String::new()
                        };
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                             CACHE-CONTROL: max-age=1800\r\n\
                             DATE: Tue, 29 Sep 2026 18:00:00 GMT\r\n\
                             EXT:\r\n\
                             LOCATION: http://192.168.7.2:{}{}\r\n\
                             {}SERVER: Linux/6.6 UPnP/1.0 Ext-Monitor/2.3\r\n\
                             ST: {}\r\n\
                             USN: {}\r\n\
                             BOOTID.UPNP.ORG: 1\r\n\
                             CONFIGID.UPNP.ORG: 1\r\n\
                             SEARCHPORT.UPNP.ORG: 1900\r\n\r\n",
                            http_port, loc_path, app_url_header, st, usn
                        );
                        let _ = socket.send_to(response.as_bytes(), src);
                    }
                }
            }
        })
        .expect("Failed to spawn ssdp thread");
}

/// Transmits periodic SSDP NOTIFY announcements to local network and host
fn send_ssdp_announcements(socket: &UdpSocket, targets: &[SocketAddr], http_port: u16) {
    let notifications = [
        ("urn:dial-multiscreen-org:service:dial:1", "/dial/dd.xml", "uuid:ext-monitor-dial-device", true),
        ("urn:dial-multiscreen-org:device:dial:1", "/dial/dd.xml", "uuid:ext-monitor-dial-device", true),
    ];

    for &(nt, loc_path, udn, is_dial) in &notifications {
        let app_url_header = if is_dial {
            format!("APPLICATION-URL: http://192.168.7.2:{}/apps/\r\n", http_port)
        } else {
            String::new()
        };
        let msg = format!(
            "NOTIFY * HTTP/1.1\r\n\
             HOST: 239.255.255.250:1900\r\n\
             CACHE-CONTROL: max-age=1800\r\n\
             LOCATION: http://192.168.7.2:{}{}\r\n\
             {}NT: {}\r\n\
             NTS: ssdp:alive\r\n\
             SERVER: Linux/6.6 UPnP/1.0 RaspCast/1.56\r\n\
             USN: {}::{}\r\n\
             BOOTID.UPNP.ORG: 1\r\n\
             CONFIGID.UPNP.ORG: 1\r\n\r\n",
            http_port, loc_path, app_url_header, nt, udn, nt
        );
        for target in targets {
            let _ = socket.send_to(msg.as_bytes(), target);
        }
    }
}

/// Parses an incoming SSDP M-SEARCH packet and returns the matching XML endpoint path and ST header
pub fn parse_ssdp_msearch(req: &str) -> Option<(&'static str, &'static str)> {
    let trimmed = req.trim_start();
    if !trimmed.starts_with("M-SEARCH") && !trimmed.starts_with("m-search") {
        return None;
    }
    if req.contains("dial-multiscreen-org") {
        if req.contains("device:dial:1") {
            Some(("/dial/dd.xml", "urn:dial-multiscreen-org:device:dial:1"))
        } else {
            Some(("/dial/dd.xml", "urn:dial-multiscreen-org:service:dial:1"))
        }
    } else if req.contains("MediaRenderer") || req.contains("RenderingControl") || req.contains("AVTransport") {
        Some(("/upnp/desc.xml", "urn:schemas-upnp-org:device:MediaRenderer:1"))
    } else if req.contains("ssdp:all") || req.contains("upnp:rootdevice") {
        Some(("/dial/dd.xml", "urn:dial-multiscreen-org:device:dial:1"))
    } else {
        None
    }
}

/// Generates UPnP MediaRenderer XML description
pub fn get_upnp_desc_xml(host_ip: &str, http_port: u16) -> String {
    format!(
        r#"<?xml version="1.0"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
    <friendlyName>RaspCast DLNA ({})</friendlyName>
    <manufacturer>Carlos Alberto / Ext-Monitor Project</manufacturer>
    <manufacturerURL>https://github.com/programandosolucoes/ext-monitor</manufacturerURL>
    <modelDescription>Smart IoT Display &amp; Audio Renderer</modelDescription>
    <modelName>Ext-Monitor IoT Appliance</modelName>
    <modelNumber>v2.3.0</modelNumber>
    <modelURL>http://{}:{}/</modelURL>
    <UDN>uuid:ext-monitor-bcm2835-renderer</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:RenderingControl</serviceId>
        <SCPDURL>/upnp/RenderingControl.xml</SCPDURL>
        <controlURL>/upnp/control/RenderingControl</controlURL>
        <eventSubURL>/upnp/event/RenderingControl</eventSubURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:AVTransport</serviceId>
        <SCPDURL>/upnp/AVTransport.xml</SCPDURL>
        <controlURL>/upnp/control/AVTransport</controlURL>
        <eventSubURL>/upnp/event/AVTransport</eventSubURL>
      </service>
    </serviceList>
  </device>
</root>"#,
        host_ip, host_ip, http_port
    )
}

/// Generates DIAL (Discovery and Launch) XML description (for YouTube Cast)
pub fn get_dial_dd_xml(host_ip: &str, http_port: u16) -> String {
    format!(
        r#"<?xml version="1.0"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:dial-multiscreen-org:device:dial:1</deviceType>
    <friendlyName>RaspCast ({})</friendlyName>
    <manufacturer>Carlos Alberto / Ext-Monitor Project</manufacturer>
    <modelName>Eureka Dongle</modelName>
    <modelURL>http://{}:{}/</modelURL>
    <UDN>uuid:ext-monitor-dial-device</UDN>
    <iconList>
      <icon>
        <mimetype>image/png</mimetype>
        <width>96</width>
        <height>96</height>
        <depth>32</depth>
        <url>/setup/icon.png</url>
      </icon>
    </iconList>
    <serviceList>
      <service>
        <serviceType>urn:dial-multiscreen-org:service:dial:1</serviceType>
        <serviceId>urn:dial-multiscreen-org:serviceId:dial</serviceId>
        <controlURL>/apps</controlURL>
        <eventSubURL></eventSubURL>
        <SCPDURL>/dial/dial.xml</SCPDURL>
      </service>
    </serviceList>
  </device>
</root>"#,
        host_ip, host_ip, http_port
    )
}

#[derive(Debug, Clone)]
pub struct AudioSpectrumState {
    pub bands: [f32; 24],
    pub peaks: [f32; 24],
    pub rms_db: f32,
    pub is_active: bool,
    pub last_update: std::time::Instant,
}

static GLOBAL_SPECTRUM: OnceLock<Arc<Mutex<AudioSpectrumState>>> = OnceLock::new();

pub fn get_audio_spectrum() -> Arc<Mutex<AudioSpectrumState>> {
    GLOBAL_SPECTRUM
        .get_or_init(|| {
            Arc::new(Mutex::new(AudioSpectrumState {
                bands: [0.0; 24],
                peaks: [0.0; 24],
                rms_db: -96.0,
                is_active: false,
                last_update: std::time::Instant::now(),
            }))
        })
        .clone()
}

pub fn update_audio_spectrum(bands: &[f32; 24], rms_db: f32) {
    let has_signal = rms_db > -55.0 || bands.iter().any(|&b| b > 0.02);
    let spec_arc = get_audio_spectrum();
    if let Ok(mut spec) = spec_arc.lock() {
        if has_signal {
            spec.rms_db = rms_db;
            spec.is_active = true;
            spec.last_update = std::time::Instant::now();
            for i in 0..24 {
                let val = bands[i].clamp(0.0, 1.0);
                spec.bands[i] = val;
                if val > spec.peaks[i] {
                    spec.peaks[i] = val;
                } else {
                    spec.peaks[i] = (spec.peaks[i] - 0.02).max(0.0);
                }
            }
        } else {
            // Signal is silence / inactive: immediately zero out values to eliminate false/stale data
            spec.is_active = false;
            spec.rms_db = -60.0;
            spec.bands = [0.0; 24];
            spec.peaks = [0.0; 24];
        }
    }

    if let Ok(mut trk) = get_media_state().lock() {
        if has_signal {
            if trk.state != "playing" {
                trk.state = "playing".to_string();
                trk.source = "pc_audio".to_string();
                trk.title = "HDMI Digital Audio (48kHz)".to_string();
                trk.artist = "Real-time Host PC Signal (PCM PipeWire)".to_string();
                trk.album = "Ext-Monitor Low-Latency Audio".to_string();
            }
        } else if trk.state == "playing" && trk.source == "pc_audio" {
            trk.state = "idle".to_string();
        }
    }
}

/// Start background UDP listener on port 5006 for real-time audio spectrum telemetry
pub fn start_audio_telemetry_listener(running: Arc<AtomicBool>) {
    thread::Builder::new()
        .name("audio-telemetry".to_string())
        .spawn(move || {
            let bind_addr = SocketAddr::from(([0, 0, 0, 0], 5006));
            let socket = match UdpSocket::bind(bind_addr) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("\x1b[1;33m[audio-telemetry]\x1b[0m Failed to bind UDP 5006: {}. Running without direct UDP telemetry.", e);
                    return;
                }
            };
            let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));
            println!("\x1b[1;32m[audio-telemetry]\x1b[0m Real-time audio spectrum UDP listener active on port 5006");

            let mut buf = [0u8; 1024];
            while running.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((len, _)) if len >= 24 => {
                        let mut bands = [0.0f32; 24];
                        for i in 0..24 {
                            bands[i] = (buf[i] as f32) / 255.0f32;
                        }
                        let rms_byte = if len >= 25 { buf[24] } else { 128 };
                        let rms_db = ((rms_byte as f32) / 255.0f32 * 60.0) - 60.0;
                        update_audio_spectrum(&bands, rms_db);
                    }
                    _ => {
                        // Socket timeout (500ms): zero out spectrum so stale/false data does not persist
                        update_audio_spectrum(&[0.0; 24], -60.0);
                    }
                }
            }
        })
        .expect("Failed to spawn audio telemetry thread");
}

// -----------------------------------------------------------------------------
// Visualizer Engine on /dev/fb0 (30 FPS Dynamic Animated Spectrum & Metadata)
// -----------------------------------------------------------------------------

pub struct VisualizerEngine;

impl VisualizerEngine {
    /// Launches the HDMI visualizer daemon. Runs only when audio is playing and video is idle.
    pub fn start(running: Arc<AtomicBool>, pipeline_mgr: Arc<crate::pipeline::PipelineManager>) {
        thread::Builder::new()
            .name("hdmi-visualizer".to_string())
            .spawn(move || {
                let state_arc = get_media_state();
                let spec_arc = get_audio_spectrum();
                let mut frame_buf = vec![0u8; FB_SIZE];
                let mut was_drawing = false;

                println!("\x1b[1;36m[visualizer]\x1b[0m HDMI Dynamic Audio Visualizer Engine ready (Single-HDMI Multiplexed).");

                while running.load(Ordering::Relaxed) {
                    // 1. Dependency Rule: Single HDMI Port on Raspberry Pi Zero (Blueprint 34 / Level 0 Arbiter)
                    // If Desktop Video streaming is active (Level 0), video owns 100% of HDMI scanout.
                    // Visualizer must NOT write to /dev/fb0 and must NEVER trigger splash screen.
                    if pipeline_mgr.current_kind().is_some() || crate::flow::ARBITER.is_level0_active() {
                        if was_drawing {
                            was_drawing = false;
                        }
                        thread::sleep(Duration::from_millis(150));
                        continue;
                    }

                    // 2. Audio Spectrum Telemetry Evaluation (800ms silence/inactivity cutoff)
                    let (is_audio_active, bands, peaks, rms_db) = {
                        let mut s = spec_arc.lock().unwrap();
                        let active = s.is_active && s.last_update.elapsed() < Duration::from_millis(800);
                        if !active {
                            s.is_active = false;
                            s.rms_db = -60.0;
                            s.bands = [0.0; 24];
                            s.peaks = [0.0; 24];
                        }
                        (active, s.bands, s.peaks, s.rms_db)
                    };

                    let (vis_enabled, title, artist, album) = {
                        let mut st = state_arc.lock().unwrap();
                        if !is_audio_active && st.source == "pc_audio" {
                            st.state = "idle".to_string();
                        }
                        // Level 2 Arbiter Rule: only render visualizer if Arbiter explicitly grants Level 2 with audio active
                        let should_render = crate::flow::ARBITER.can_visualizer_render() && is_audio_active;
                        (
                            should_render,
                            st.title.clone(),
                            st.artist.clone(),
                            st.album.clone(),
                        )
                    };

                    // 3. If audio has stopped and visualizer was drawing, restore Splash screen ONLY if Arbiter permits
                    if !vis_enabled {
                        if was_drawing {
                            if crate::flow::ARBITER.can_splash_render() {
                                println!("\x1b[1;33m[visualizer]\x1b[0m Audio stopped/silenced. Restoring Ready Splash screen...");
                                crate::display::SplashEngine::show_ready();
                            }
                            was_drawing = false;
                        }
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }

                    was_drawing = true;

                    // Render background gradient (Deep Night Blue to Charcoal)
                    render_gradient_background(&mut frame_buf);

                    // Render Glassmorphic Media Player Card with live metadata & VU meter
                    render_player_card(&mut frame_buf, &title, &artist, &album, rms_db);

                    // Render Dynamic Equalizer Spectrum Bars from REAL AUDIO TELEMETRY
                    render_spectrum_bars(&mut frame_buf, &bands, &peaks);

                    // Blit buffer to /dev/fb0
                    blit_to_fb0(&frame_buf);

                    thread::sleep(Duration::from_millis(33)); // ~30 FPS
                }
            })
            .expect("Failed to spawn visualizer thread");
    }
}

/// Render dark gradient background in RGB565
fn render_gradient_background(buf: &mut [u8]) {
    for y in 0..FB_HEIGHT {
        let t = y as f32 / FB_HEIGHT as f32;
        let r = ((12.0 * (1.0 - t)) + (4.0 * t)) as u8;
        let g = ((14.0 * (1.0 - t)) + (5.0 * t)) as u8;
        let b = ((28.0 * (1.0 - t)) + (12.0 * t)) as u8;
        let c = rgb565(r, g, b);
        let c_low = (c & 0xFF) as u8;
        let c_high = ((c >> 8) & 0xFF) as u8;

        let row_start = y * FB_WIDTH * 2;
        for x in 0..FB_WIDTH {
            let idx = row_start + x * 2;
            buf[idx] = c_low;
            buf[idx + 1] = c_high;
        }
    }
}

/// Render modern player card with title, artist, album and live stereo VU meter
fn render_player_card(buf: &mut [u8], title: &str, artist: &str, album: &str, rms_db: f32) {
    let card_x = 240;
    let card_y = 120;
    let card_w = 800;
    let card_h = 320;

    let card_bg = rgb565(20, 24, 42);
    let border_cyan = rgb565(0, 229, 255);
    let c_low = (card_bg & 0xFF) as u8;
    let c_high = ((card_bg >> 8) & 0xFF) as u8;

    let b_low = (border_cyan & 0xFF) as u8;
    let b_high = ((border_cyan >> 8) & 0xFF) as u8;

    // Fill card interior
    for y in card_y..(card_y + card_h) {
        let row_start = y * FB_WIDTH * 2;
        for x in card_x..(card_x + card_w) {
            let idx = row_start + x * 2;
            if x == card_x || x == card_x + card_w - 1 || y == card_y || y == card_y + card_h - 1 {
                buf[idx] = b_low;
                buf[idx + 1] = b_high;
            } else {
                buf[idx] = c_low;
                buf[idx + 1] = c_high;
            }
        }
    }

    // Draw album art placeholder square (180x180)
    let art_x = card_x + 40;
    let art_y = card_y + 40;
    let art_sz = 180;
    let art_color = rgb565(32, 40, 70);
    let art_low = (art_color & 0xFF) as u8;
    let art_high = ((art_color >> 8) & 0xFF) as u8;

    for y in art_y..(art_y + art_sz) {
        let row_start = y * FB_WIDTH * 2;
        for x in art_x..(art_x + art_sz) {
            let idx = row_start + x * 2;
            buf[idx] = art_low;
            buf[idx + 1] = art_high;
        }
    }

    // Draw decorative musical notes icon inside art square
    let note_c = rgb565(0, 229, 255);
    draw_rect(buf, art_x + 70, art_y + 60, 12, 60, note_c);
    draw_rect(buf, art_x + 105, art_y + 50, 12, 70, note_c);
    draw_rect(buf, art_x + 70, art_y + 50, 47, 14, note_c);
    draw_rect(buf, art_x + 55, art_y + 105, 27, 20, note_c);
    draw_rect(buf, art_x + 90, art_y + 105, 27, 20, note_c);

    // Text rendering: Badge, Title, Artist, Album
    let text_x = card_x + 250;
    let badge_color = rgb565(126, 231, 135); // Accent Green
    let white = rgb565(255, 255, 255);
    let cyan = rgb565(0, 229, 255);
    let gray = rgb565(180, 190, 210);

    draw_text_bitmap(buf, text_x, card_y + 45, "● TOCANDO AGORA - IOT MEDIA RENDERER", badge_color, 2);
    draw_text_bitmap(buf, text_x, card_y + 85, title, white, 3);
    draw_text_bitmap(buf, text_x, card_y + 140, artist, cyan, 2);
    draw_text_bitmap(buf, text_x, card_y + 175, album, gray, 2);

    // Live Stereo VU Meter Bar (Green -> Amber -> Red Peak)
    let vu_y = card_y + 215;
    let vu_w = 480;
    let vu_h = 14;
    let normalized = ((rms_db + 60.0) / 60.0).clamp(0.0, 1.0);
    let fill_w = (normalized * vu_w as f32) as usize;

    // Meter background
    draw_rect(buf, text_x, vu_y, vu_w, vu_h, rgb565(15, 20, 35));
    // Meter fill
    for bx in 0..fill_w {
        let ratio = bx as f32 / vu_w as f32;
        let c = if ratio < 0.65 {
            rgb565(126, 231, 135) // Green
        } else if ratio < 0.88 {
            rgb565(255, 171, 64) // Amber
        } else {
            rgb565(255, 82, 82) // Red
        };
        draw_rect(buf, text_x + bx, vu_y, 1, vu_h, c);
    }
    // Meter border
    draw_rect(buf, text_x, vu_y, vu_w, 1, border_cyan);
    draw_rect(buf, text_x, vu_y + vu_h - 1, vu_w, 1, border_cyan);

    // Decorative soundwave horizontal accent line
    let line_y = card_y + card_h - 35;
    let line_c = rgb565(126, 231, 135);
    draw_rect(buf, card_x + 40, line_y, card_w - 80, 3, line_c);
}

/// Render 24 animated frequency bars at the bottom
fn render_spectrum_bars(buf: &mut [u8], heights: &[f32; 24], peaks: &[f32; 24]) {
    let start_x = 240;
    let base_y = 660;
    let max_h = 160;
    let bar_w = 26;
    let gap = 8;

    for i in 0..24 {
        let x = start_x + i * (bar_w + gap);
        let h = ((heights[i] * max_h as f32) as usize).min(max_h).max(4);
        let peak_h = ((peaks[i] * max_h as f32) as usize).min(max_h).max(4);

        // Bar body with vertical gradient (Cyan -> Neon Green -> Amber Peak)
        for y_off in 0..h {
            let y = base_y - y_off;
            let ratio = y_off as f32 / max_h as f32;
            let color = if ratio < 0.6 {
                rgb565(0, 229, 255) // Cyan
            } else if ratio < 0.85 {
                rgb565(126, 231, 135) // Neon Green
            } else {
                rgb565(255, 171, 64) // Amber Peak
            };

            let c_low = (color & 0xFF) as u8;
            let c_high = ((color >> 8) & 0xFF) as u8;
            let row_start = y * FB_WIDTH * 2;
            for bx in 0..bar_w {
                let idx = row_start + (x + bx) * 2;
                buf[idx] = c_low;
                buf[idx + 1] = c_high;
            }
        }

        // Floating peak indicator dot
        let peak_y = base_y - peak_h;
        let peak_color = rgb565(255, 82, 82); // Bright Red/Pink
        draw_rect(buf, x, peak_y, bar_w, 3, peak_color);
    }
}

/// Simple rectangle draw helper in RGB565
fn draw_rect(buf: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: u16) {
    let c_low = (color & 0xFF) as u8;
    let c_high = ((color >> 8) & 0xFF) as u8;

    for cy in y..(y + h).min(FB_HEIGHT) {
        let row_start = cy * FB_WIDTH * 2;
        for cx in x..(x + w).min(FB_WIDTH) {
            let idx = row_start + cx * 2;
            buf[idx] = c_low;
            buf[idx + 1] = c_high;
        }
    }
}

/// Minimalist 5x7 ASCII Bitmap Font renderer with scale factor
fn draw_text_bitmap(buf: &mut [u8], start_x: usize, start_y: usize, text: &str, color: u16, scale: usize) {
    let mut cur_x = start_x;
    for ch in text.chars().take(45) {
        if cur_x + 6 * scale >= FB_WIDTH {
            break;
        }
        let glyph = get_glyph_5x7(ch);
        for col in 0..5 {
            let col_bits = glyph[col];
            for row in 0..7 {
                if (col_bits & (1 << row)) != 0 {
                    draw_rect(buf, cur_x + col * scale, start_y + row * scale, scale, scale, color);
                }
            }
        }
        cur_x += 6 * scale;
    }
}

/// 5x7 ASCII glyph patterns for common characters
fn get_glyph_5x7(c: char) -> [u8; 5] {
    match c {
        'A' | 'a' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        'B' | 'b' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        'C' | 'c' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        'D' | 'd' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        'E' | 'e' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        'F' | 'f' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        'G' | 'g' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        'H' | 'h' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        'I' | 'i' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        'J' | 'j' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        'K' | 'k' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        'L' | 'l' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        'M' | 'm' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        'N' | 'n' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        'O' | 'o' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        'P' | 'p' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        'Q' | 'q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        'R' | 'r' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        'S' | 's' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' | 't' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        'U' | 'u' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        'V' | 'v' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        'W' | 'w' => [0x7F, 0x20, 0x18, 0x20, 0x7F],
        'X' | 'x' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' | 'y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' | 'z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        '0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1E],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        ':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        '.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        '/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        '(' => [0x00, 0x1C, 0x22, 0x41, 0x00],
        ')' => [0x00, 0x41, 0x22, 0x1C, 0x00],
        '●' | '*' => [0x1C, 0x3E, 0x3E, 0x3E, 0x1C],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

/// Blits frame directly to /dev/fb0 with unblanking
fn blit_to_fb0(buffer: &[u8]) {
    if buffer.len() < FB_SIZE {
        return;
    }

    if let Ok(mut file) = OpenOptions::new().read(true).write(true).open("/dev/fb0") {
        let fd = file.as_raw_fd();
        const FBIOBLANK: libc::c_ulong = 0x4611;
        unsafe {
            libc::ioctl(fd, FBIOBLANK as _, 0 as libc::c_int);
        }

        if let Ok(tty1) = OpenOptions::new().read(true).write(true).open("/dev/tty1") {
            const KDSETMODE: libc::c_ulong = 0x4B3A;
            const KD_GRAPHICS: libc::c_ulong = 0x01;
            unsafe {
                libc::ioctl(tty1.as_raw_fd(), KDSETMODE as _, KD_GRAPHICS);
            }
        }

        use std::io::{Seek, SeekFrom, Write};
        let _ = file.seek(SeekFrom::Start(0));
        let _ = file.write_all(&buffer[..FB_SIZE]);
        let _ = file.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb565_color_conversions() {
        assert_eq!(rgb565(0, 0, 0), 0x0000); // Black
        assert_eq!(rgb565(255, 255, 255), 0xFFFF); // White
        assert_eq!(rgb565(255, 0, 0), 0xF800); // Red (5 bits)
        assert_eq!(rgb565(0, 255, 0), 0x07E0); // Green (6 bits)
        assert_eq!(rgb565(0, 0, 255), 0x001F); // Blue (5 bits)
    }

    #[test]
    fn test_audio_spectrum_silence_and_active() {
        // Active audio signal
        let active_bands = [0.5f32; 24];
        update_audio_spectrum(&active_bands, -20.0);
        {
            let spec = get_audio_spectrum();
            let s = spec.lock().unwrap();
            assert!(s.is_active);
            assert_eq!(s.rms_db, -20.0);
            assert_eq!(s.bands[0], 0.5);
        }

        // Silence signal below threshold (-55 dB)
        let silence_bands = [0.0f32; 24];
        update_audio_spectrum(&silence_bands, -65.0);
        {
            let spec = get_audio_spectrum();
            let s = spec.lock().unwrap();
            assert!(!s.is_active);
            assert_eq!(s.rms_db, -60.0);
            assert_eq!(s.bands[0], 0.0);
        }
    }

    #[test]
    fn test_glyph_pattern_dimensions() {
        let glyph_a = get_glyph_5x7('A');
        assert_eq!(glyph_a.len(), 5);
        let glyph_0 = get_glyph_5x7('0');
        assert_eq!(glyph_0.len(), 5);
    }

    #[test]
    fn test_dial_youtube_google_home_descriptor() {
        let xml = get_dial_dd_xml("192.168.7.2", 8080);
        assert!(xml.contains("urn:dial-multiscreen-org:device:dial:1"));
        assert!(xml.contains("RaspCast (192.168.7.2)"));
        assert!(xml.contains("urn:dial-multiscreen-org:service:dial:1"));
        assert!(xml.contains("<controlURL>/apps</controlURL>"));
        assert!(xml.contains("<SCPDURL>/dial/dial.xml</SCPDURL>"));
    }

    #[test]
    fn test_upnp_media_renderer_descriptor() {
        let xml = get_upnp_desc_xml("192.168.7.2", 8080);
        assert!(xml.contains("urn:schemas-upnp-org:device:MediaRenderer:1"));
        assert!(xml.contains("RaspCast DLNA (192.168.7.2)"));
        assert!(xml.contains("urn:schemas-upnp-org:service:RenderingControl:1"));
        assert!(xml.contains("urn:schemas-upnp-org:service:AVTransport:1"));
        assert!(xml.contains("<modelName>Ext-Monitor IoT Appliance</modelName>"));
    }

    #[test]
    fn test_ssdp_msearch_dial_and_upnp_detection() {
        // DIAL (YouTube / Google Cast) query
        let dial_req = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:dial-multiscreen-org:service:dial:1\r\n\r\n";
        let res = parse_ssdp_msearch(dial_req);
        assert_eq!(res, Some(("/dial/dd.xml", "urn:dial-multiscreen-org:service:dial:1")));

        // UPnP MediaRenderer query
        let upnp_req = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n";
        let res = parse_ssdp_msearch(upnp_req);
        assert_eq!(res, Some(("/upnp/desc.xml", "urn:schemas-upnp-org:device:MediaRenderer:1")));

        // Broad ssdp:all query routes to DIAL to avoid duplicate ghost MediaRenderer in Chrome
        let all_req = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: ssdp:all\r\n\r\n";
        let res = parse_ssdp_msearch(all_req);
        assert_eq!(res, Some(("/dial/dd.xml", "urn:dial-multiscreen-org:device:dial:1")));

        // Non-SSDP or unrelated packet
        let invalid_req = "GET /index.html HTTP/1.1\r\n\r\n";
        assert_eq!(parse_ssdp_msearch(invalid_req), None);
    }

    #[test]
    fn test_media_track_state_transitions() {
        let track_arc = get_media_state();
        {
            let mut trk = track_arc.lock().unwrap();
            trk.state = "playing".to_string();
            trk.title = "Test Audio Track".to_string();
            trk.artist = "Ext-Monitor Artist".to_string();
            trk.volume = 85;
        }

        {
            let trk = track_arc.lock().unwrap();
            assert_eq!(trk.state, "playing");
            assert_eq!(trk.title, "Test Audio Track");
            assert_eq!(trk.artist, "Ext-Monitor Artist");
            assert_eq!(trk.volume, 85);
        }
    }
}
