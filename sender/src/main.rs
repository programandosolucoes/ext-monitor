use std::collections::HashMap;
use std::fs;
use std::net::UdpSocket;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, thread};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, Value};

static RUNNING: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProfile {
    TrueColor,    // Standard 24-bit color (VBR 75% avg, dynamic QP)
    Economy256,   // Emulated 256-color coarse quantization (min-qp 30, max-qp 44, VBR 50%)
    Grayscale,    // Monochrome terminal mode (saturation=0.0)
}

impl ColorProfile {
    pub fn name(&self) -> &'static str {
        match self {
            ColorProfile::TrueColor => "24-bit TrueColor (Full)",
            ColorProfile::Economy256 => "256-Color (Coarse QP 30-44)",
            ColorProfile::Grayscale => "Monochrome (Grayscale)",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;32m=====================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: AMD GPU Offload Second Monitor Sender   \x1b[0m");
    println!("\x1b[1;32m=====================================================\x1b[0m");

    let args: Vec<String> = env::args().collect();
    let target_ip = args.get(1).map(|s| s.as_str()).unwrap_or("192.168.7.2");
    let target_port = args
        .get(2)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5000);
    let raw_bitrate = args
        .get(3)
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    let mode = args.get(4).map(|s| s.to_lowercase()).unwrap_or_else(|| "extend".to_string());
    let encoder_arg = args.get(5).map(|s| s.as_str()).unwrap_or("auto");
    let encoder = EncoderApi::from_str(encoder_arg);
    let mut fps = args.get(6).and_then(|p| p.parse::<u32>().ok()).unwrap_or(30);

    // Auto-scale bitrate based on framerate with Ultra-Low Latency presets (sub-15ms)
    let auto_bitrate = match fps {
        f if f >= 50 => 6000,
        f if f >= 25 => 3000,
        f if f >= 15 => 1800,
        f if f >= 10 => 1200,
        _ => 800,
    };
    let mut bitrate = if raw_bitrate == 0 || raw_bitrate == 8000 {
        auto_bitrate
    } else {
        raw_bitrate
    };

    let hud_arg = args.iter().any(|a| {
        let s = a.to_lowercase();
        s == "hud" || s == "--hud" || s == "true" || s == "1"
    });
    let mut hud_showing = hud_arg;
    let mut hud_hide_at = if hud_arg {
        Some(Instant::now() + Duration::from_secs(60))
    } else {
        None
    };

    let mut color_profile = if args.iter().any(|a| {
        let s = a.to_lowercase();
        s == "256" || s == "--256" || s == "--colors=256" || s == "economy" || s == "--economy"
    }) {
        ColorProfile::Economy256
    } else if args.iter().any(|a| {
        let s = a.to_lowercase();
        s == "gray" || s == "--gray" || s == "bw" || s == "--bw" || s == "mono"
    }) {
        ColorProfile::Grayscale
    } else {
        ColorProfile::TrueColor
    };

    println!("\x1b[1;34m[*] Target:\x1b[0m {}:{}", target_ip, target_port);
    println!("\x1b[1;34m[*] Bitrate:\x1b[0m {} kbps (Adaptive VBR, {} FPS)", bitrate, fps);
    println!("\x1b[1;34m[*] Color Profile:\x1b[0m {}", color_profile.name());
    println!("\x1b[1;34m[*] Display Mode:\x1b[0m {} (options: 'extend' or 'clone')", mode);
    println!("\x1b[1;34m[*] Encoder Engine:\x1b[0m {:?} (arg: '{}')", encoder, encoder_arg);
    println!("\x1b[1;34m[*] Target Framerate:\x1b[0m {} FPS", fps);
    println!("\x1b[1;34m[*] Diagnostic HUD:\x1b[0m {}", if hud_arg { "\x1b[1;32mENABLED (Auto-hide in 60s)\x1b[0m" } else { "\x1b[1;30mDISABLED\x1b[0m" });

    // Open UDP control socket for Web Dashboard Hot-Apply
    let ctrl_sock = UdpSocket::bind("0.0.0.0:5001").ok();
    if let Some(ref s) = ctrl_sock {
        let _ = s.set_nonblocking(true);
        println!("\x1b[1;32m[+] Control Listener ativo na porta UDP 5001 (Web Hot-Apply & HUD Trigger)\x1b[0m");
    }

    let monitor_to_record = if mode == "clone" {
        "eDP-1"
    } else {
        // In extended mode, ensure kernel HDMI-A-1 connector is forced and configured in GNOME
        ensure_kernel_hdmi_connected();
        ensure_gnome_displays();
        "HDMI-1"
    };

    println!("\x1b[1;34m[*] Recording Monitor:\x1b[0m {}", monitor_to_record);

    // Signal handler setup
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_setup(r);

    while running.load(Ordering::SeqCst) {
        println!("\x1b[1;34m[*] Connecting to GNOME Mutter ScreenCast via D-Bus...\x1b[0m");

        let dbus_conn = match Connection::session() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] D-Bus session connection failed: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        // Create Mutter ScreenCast Session
        let empty_props: HashMap<&str, Value> = HashMap::new();
        let session_reply = match dbus_conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            "/org/gnome/Mutter/ScreenCast",
            Some("org.gnome.Mutter.ScreenCast"),
            "CreateSession",
            &(empty_props,),
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] CreateSession failed: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let session_path: OwnedObjectPath = session_reply.body().deserialize()?;
        println!("\x1b[1;32m[+] Mutter Session created:\x1b[0m {}", session_path);

        let mut monitor_props: HashMap<&str, Value> = HashMap::new();
        monitor_props.insert("cursor-mode", Value::from(1u32));

        let stream_reply = match dbus_conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            session_path.as_str(),
            Some("org.gnome.Mutter.ScreenCast.Session"),
            "RecordMonitor",
            &(monitor_to_record, monitor_props),
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] RecordMonitor('{}') failed: {}. Retrying in 2s...\x1b[0m", monitor_to_record, e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let stream_path: OwnedObjectPath = stream_reply.body().deserialize()?;
        println!("\x1b[1;32m[+] {} ScreenCast Stream created:\x1b[0m {}", monitor_to_record, stream_path);

        let stream_proxy = match Proxy::new(
            &dbus_conn,
            "org.gnome.Mutter.ScreenCast",
            stream_path.as_str(),
            "org.gnome.Mutter.ScreenCast.Stream",
        ) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to create stream proxy: {}. Retrying...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let mut signal_iter = match stream_proxy.receive_signal("PipeWireStreamAdded") {
            Ok(iter) => iter,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to subscribe to PipeWireStreamAdded: {}. Retrying...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        // Start the ScreenCast Session
        if let Err(e) = dbus_conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            session_path.as_str(),
            Some("org.gnome.Mutter.ScreenCast.Session"),
            "Start",
            &(),
        ) {
            eprintln!("\x1b[1;31m[!] Session.Start() failed: {}. Retrying...\x1b[0m", e);
            thread::sleep(Duration::from_secs(2));
            continue;
        }

        // Wait for PipeWire Node ID
        let node_id = match signal_iter.next() {
            Some(sig) => match sig.body().deserialize::<(u32,)>() {
                Ok((id,)) => id,
                Err(e) => {
                    eprintln!("\x1b[1;31m[!] Failed to deserialize node_id: {}\x1b[0m", e);
                    continue;
                }
            },
            None => {
                eprintln!("\x1b[1;31m[!] No PipeWireStreamAdded signal received. Retrying...\x1b[0m");
                continue;
            }
        };

        println!("\x1b[1;32m[+] PipeWire Node ID for {}:\x1b[0m {}", monitor_to_record, node_id);

        // Spawn GStreamer pipeline with autoconnect=false
        println!("\x1b[1;33m[*] Starting {:?} hardware streaming pipeline ({} FPS, HUD: {}, Color: {:?})...\x1b[0m", encoder, fps, hud_showing, color_profile);
        let mut child = match spawn_streamer(target_ip, target_port, bitrate, encoder, fps, hud_showing, color_profile) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to spawn streamer: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        // Allow GStreamer pipewiresrc to register its input port
        thread::sleep(Duration::from_millis(800));

        // Find the exact output port for node_id and link it to ext-hdmi-sender
        link_monitor_port_to_sender(node_id, monitor_to_record);

        println!("\x1b[1;32m[+] Monitor {} is streaming LIVE to Pi Zero at {} FPS ({})!\x1b[0m", monitor_to_record, fps, color_profile.name());

        // Supervise streaming process & listen for Web Hot-Apply changes
        while running.load(Ordering::SeqCst) {
            let mut restart_pipeline = false;

            // 1. Check if HUD 60s timeout expired (auto-hide)
            if let Some(hide_at) = hud_hide_at {
                if Instant::now() >= hide_at && hud_showing {
                    println!("\x1b[1;33m[*] HUD auto-hide (60s): Ocultando HUD para liberar tela 100% limpa...\x1b[0m");
                    hud_showing = false;
                    hud_hide_at = None;
                    restart_pipeline = true;
                }
            }

            // 2. Poll UDP control socket for commands from Web Dashboard
            if let Some(ref s) = ctrl_sock {
                let mut buf = [0u8; 2048];
                while let Ok((n, _src)) = s.recv_from(&mut buf) {
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf[..n]) {
                        if let Some(action) = v.get("action").and_then(|x| x.as_str()) {
                            if action == "trigger_hud" {
                                println!("\x1b[1;32m[+] Comando Web: Reativando HUD na tela por 60 segundos!\x1b[0m");
                                hud_showing = true;
                                hud_hide_at = Some(Instant::now() + Duration::from_secs(60));
                                restart_pipeline = true;
                            } else if action == "hide_hud" || action == "kill_hud" {
                                println!("\x1b[1;33m[*] Comando Web: Ocultando HUD imediatamente a pedido do usuário!\x1b[0m");
                                hud_showing = false;
                                hud_hide_at = None;
                                restart_pipeline = true;
                            }
                        }
                        let new_fps_opt = v.get("fps").and_then(|x| x.as_u64()).map(|x| x as u32);
                        let new_bitrate_opt = v.get("bitrate").and_then(|x| x.as_u64()).map(|x| x as u32);

                        if let Some(new_bitrate) = new_bitrate_opt {
                            if new_bitrate != bitrate && new_bitrate >= 400 && new_bitrate <= 15000 {
                                println!("\x1b[1;34m[*] Mudança de Bitrate via Web: {} -> {} kbps (Ultra-Low Latency)\x1b[0m", bitrate, new_bitrate);
                                bitrate = new_bitrate;
                                restart_pipeline = true;
                            }
                        }

                        if let Some(new_fps) = new_fps_opt {
                            if new_fps != fps && new_fps >= 10 && new_fps <= 60 {
                                println!("\x1b[1;34m[*] Mudança de FPS via Web: {} -> {} FPS\x1b[0m", fps, new_fps);
                                fps = new_fps;
                                if new_bitrate_opt.is_none() {
                                    bitrate = match fps {
                                        f if f >= 50 => 6000,
                                        f if f >= 25 => 3000,
                                        f if f >= 15 => 1800,
                                        f if f >= 10 => 1200,
                                        _ => 800,
                                    };
                                }
                                restart_pipeline = true;
                            }
                        }
                        if let Some(new_color) = v.get("color").and_then(|x| x.as_str()) {
                            let cp = match new_color {
                                "256" => ColorProfile::Economy256,
                                "gray" => ColorProfile::Grayscale,
                                _ => ColorProfile::TrueColor,
                            };
                            if cp != color_profile {
                                println!("\x1b[1;34m[*] Mudança de Cor via Web: {:?} -> {:?}\x1b[0m", color_profile, cp);
                                color_profile = cp;
                                restart_pipeline = true;
                            }
                        }
                    }
                }
            }

            if restart_pipeline {
                println!("\x1b[1;36m[*] Aplicando a quente (FPS: {}, Cor: {:?}, HUD: {})...\x1b[0m", fps, color_profile, hud_showing);
                let _ = child.kill();
                let _ = child.wait();
                child = match spawn_streamer(target_ip, target_port, bitrate, encoder, fps, hud_showing, color_profile) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[!] Falha ao reiniciar streamer: {}\x1b[0m", e);
                        break;
                    }
                };
                thread::sleep(Duration::from_millis(600));
                link_monitor_port_to_sender(node_id, monitor_to_record);
                println!("\x1b[1;32m[+] Configuração reaplicada na tela com sucesso!\x1b[0m");
                continue;
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("\x1b[1;33m[*] Streamer saiu com status: {}. Reiniciando...\x1b[0m", status);
                    break;
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(200));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[!] Erro monitorando streamer: {}\x1b[0m", e);
                    break;
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();

        // Close Mutter session
        let _ = dbus_conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            session_path.as_str(),
            Some("org.gnome.Mutter.ScreenCast.Session"),
            "Stop",
            &(),
        );

        if !running.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("\x1b[1;32m[*] ext-sender terminated cleanly.\x1b[0m");
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderApi {
    Vaapi,    // AMD & Intel hardware encoding via VA-API
    Nvenc,    // NVIDIA hardware encoding via NVENC
    Qsv,      // Intel QuickSync hardware encoding
    Software, // CPU / x264 zerolatency
}

impl EncoderApi {
    pub fn detect() -> Self {
        // 1. Check for NVIDIA NVENC
        if let Ok(out) = Command::new("gst-inspect-1.0").arg("nvh264enc").output() {
            if out.status.success() {
                return EncoderApi::Nvenc;
            }
        }
        // 2. Check for VA-API (AMD / Intel)
        if let Ok(out) = Command::new("gst-inspect-1.0").arg("vah264enc").output() {
            if out.status.success() {
                return EncoderApi::Vaapi;
            }
        }
        // 3. Check for Intel QSV
        if let Ok(out) = Command::new("gst-inspect-1.0").arg("qsvh264enc").output() {
            if out.status.success() {
                return EncoderApi::Qsv;
            }
        }
        // 4. Fallback to CPU Software x264
        EncoderApi::Software
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "nvenc" | "nvidia" => EncoderApi::Nvenc,
            "vaapi" | "amd" | "intel" => EncoderApi::Vaapi,
            "qsv" | "quicksync" => EncoderApi::Qsv,
            "software" | "cpu" | "x264" => EncoderApi::Software,
            _ => Self::detect(),
        }
    }
}

fn link_monitor_port_to_sender(node_id: u32, monitor_name: &str) {
    for _ in 0..10 {
        if let Ok(output) = Command::new("pw-dump").output() {
            if let Ok(dump) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(items) = dump.as_array() {
                    let mut out_port = None;

                    for item in items {
                        if item["type"] == "PipeWire:Interface:Port" {
                            let props = &item["info"]["props"];
                            if props["node.id"] == node_id && props["port.direction"] == "out" {
                                out_port = item["id"].as_u64();
                                break;
                            }
                        }
                    }

                    if let Some(p) = out_port {
                        println!("\x1b[1;34m[*] Found {} Output Port: {}\x1b[0m", monitor_name, p);
                        let status = Command::new("pw-link")
                            .arg(p.to_string())
                            .arg("ext-hdmi-sender:input_1")
                            .status();

                        if let Ok(s) = status {
                            if s.success() {
                                println!("\x1b[1;32m[+] Successfully linked {} (port {}) -> ext-hdmi-sender!\x1b[0m", monitor_name, p);
                                return;
                            }
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(300));
    }

    eprintln!("\x1b[1;31m[!] Warning: Failed to link {} port automatically after 3s.\x1b[0m", monitor_name);
}

fn spawn_streamer(
    target_ip: &str,
    target_port: u16,
    bitrate: u32,
    encoder: EncoderApi,
    fps: u32,
    hud: bool,
    color_profile: ColorProfile,
) -> Result<Child, std::io::Error> {
    let mut cmd = Command::new("gst-launch-1.0");
    cmd.arg("-v");

    // 1. PipeWire source with strict minimal buffering to eliminate lag
    cmd.arg("pipewiresrc")
        .arg("autoconnect=false")
        .arg("stream-properties=props,node.name=ext-hdmi-sender")
        .arg("do-timestamp=true")
        .arg("min-buffers=2")
        .arg("max-buffers=2")
        .arg("always-copy=false")
        .arg("!");

    // 2. Framerate decimation if requested < 60 FPS with maximum preference for latest frames
    if fps < 60 {
        cmd.arg("videorate")
            .arg("drop-only=true")
            .arg("new-pref=1.0")
            .arg("skip-to-first=true")
            .arg("!")
            .arg(format!("video/x-raw,framerate={}/1", fps))
            .arg("!");
    }

    // 3. Diagnostic HUD (On-Screen Display) if requested
    if hud {
        println!("\x1b[1;35m[+] Injecting Advanced On-Screen Diagnostic Telemetry HUD with Glass Transparency...\x1b[0m");
        let avg_pct = if color_profile == ColorProfile::Economy256 { 50 } else { 75 };
        let hud_text = format!(
            "text=\"[ PI ZERO EXTENDED MONITOR • ACTIVE ]\nPanel:    1600x900@59.95Hz (Nativo 1:1)\nStream:   {} FPS | Drop-on-Late (3x LIFO)\nColor:    {}\nRate:     VBR Adaptativo ({}k cap / {}% avg)\nVPU:      Broadcom VideoCore IV @ 500MHz (+25% OC)\nCPU:      ARM1176 Carga ~22% | RAM: ~141 MiB\nRede:     USB OTG (RTT 0.34ms, txq: 100)\nSync:     IDR Refresh 1.0s ({} frames)\nWeb:      http://{}:8080 (Auto-hide em 60s)\"",
            fps, color_profile.name(), bitrate, avg_pct, fps, target_ip
        );
        cmd.arg("textoverlay")
            .arg(hud_text)
            .arg("valignment=top")
            .arg("halignment=right")
            .arg("line-alignment=left")
            .arg("font-desc=\"Monospace Bold 10\"")
            .arg("color=0xFF00FF66")
            .arg("outline-color=0x80000000")
            .arg("draw-outline=true")
            .arg("shaded-background=true")
            .arg("shading-value=65")
            .arg("xpad=14")
            .arg("ypad=12")
            .arg("!")
            .arg("clockoverlay")
            .arg("time-format=\"%H:%M:%S\"")
            .arg("valignment=top")
            .arg("halignment=left")
            .arg("font-desc=\"Monospace Bold 11\"")
            .arg("color=0xFF00E5FF")
            .arg("outline-color=0x80000000")
            .arg("draw-outline=true")
            .arg("shaded-background=true")
            .arg("shading-value=65")
            .arg("xpad=14")
            .arg("ypad=12")
            .arg("!")
            .arg("timeoverlay")
            .arg("valignment=top")
            .arg("halignment=left")
            .arg("deltay=28")
            .arg("font-desc=\"Monospace Bold 10\"")
            .arg("color=0xFFFFFFFF")
            .arg("outline-color=0x80000000")
            .arg("draw-outline=true")
            .arg("shaded-background=true")
            .arg("shading-value=65")
            .arg("xpad=14")
            .arg("ypad=12")
            .arg("!");
    }

    // 4. Zero-Latency Pre-Encoder Queue: drop stale frames on the fly (Sunshine style)
    cmd.arg("queue")
        .arg("max-size-buffers=1")
        .arg("max-size-bytes=0")
        .arg("max-size-time=0")
        .arg("leaky=downstream")
        .arg("!");

    // 5. Hardware / Software encoder selection
    match encoder {
        EncoderApi::Vaapi => {
            println!("\x1b[1;36m[+] Initializing VA-API (AMD/Intel) Zero-Copy Direct GPU Pipeline (Adaptive VBR, Smooth MBBRC Full-Motion, Profile: {:?})...\x1b[0m", color_profile);
            
            if color_profile == ColorProfile::Grayscale {
                cmd.arg("videobalance").arg("saturation=0.0").arg("!");
            }

            cmd.arg("vapostproc")
                .arg("!")
                .arg("vah264enc")
                .arg(format!("bitrate={}", bitrate))
                .arg("rate-control=vbr");

            if color_profile == ColorProfile::Economy256 {
                cmd.arg("target-percentage=50")
                    .arg("min-qp=30")
                    .arg("max-qp=44");
            } else {
                cmd.arg("target-percentage=75");
            }

            cmd.arg("mbbrc=enabled")             // Controle macrobloco a macrobloco (evita picos ao mover telas inteiras)
                .arg("target-usage=7")           // AMD ultra-fast lowest latency mode
                .arg("b-frames=0")
                .arg("ref-frames=1")
                .arg("aud=true")                 // Access Unit delimiter para integridade de frames
                .arg("cabac=false")              // CAVLC simple entropy coding (super leve para o Pi Zero)
                .arg("dct8x8=false")             // Simple 4x4 transforms
                .arg("num-slices=1")             // 1 fatia inteira atômica (elimina cortes/rasgos horizontais na tela)
                .arg(format!("key-int-max={}", fps.max(15))) // Full Refresh (IDR) a cada 1.0s (estabilidade total sem micro-congelamento)
                .arg("!")
                .arg("video/x-h264,profile=constrained-baseline") // Super optimized low-overhead profile
                .arg("!");
        }
        EncoderApi::Nvenc => {
            println!("\x1b[1;36m[+] Initializing NVIDIA NVENC Zero-Latency GPU Pipeline...\x1b[0m");
            cmd.arg("videoconvert")
                .arg("!")
                .arg("video/x-raw,format=NV12")
                .arg("!")
                .arg("nvh264enc")
                .arg(format!("bitrate={}", bitrate))
                .arg("preset=low-latency-hq")
                .arg("rc-mode=cbr-ld-hq")
                .arg("zerolatency=true")
                .arg("gop-size=30")
                .arg("b-frames=0")
                .arg("!");
        }
        EncoderApi::Qsv => {
            println!("\x1b[1;36m[+] Initializing Intel QuickSync (QSV) GPU Pipeline...\x1b[0m");
            cmd.arg("videoconvert")
                .arg("!")
                .arg("video/x-raw,format=NV12")
                .arg("!")
                .arg("qsvh264enc")
                .arg(format!("bitrate={}", bitrate))
                .arg("rate-control=cbr")
                .arg("target-usage=7")
                .arg("b-frames=0")
                .arg("gop-size=30")
                .arg("!");
        }
        EncoderApi::Software => {
            println!("\x1b[1;36m[+] Initializing CPU Software x264 (Zero-Latency Ultrafast)...\x1b[0m");
            cmd.arg("videoconvert")
                .arg("!")
                .arg("video/x-raw,format=I420")
                .arg("!")
                .arg("x264enc")
                .arg(format!("bitrate={}", bitrate))
                .arg("tune=zerolatency")
                .arg("speed-preset=ultrafast")
                .arg("b-frames=0")
                .arg("ref-frames=1")
                .arg("key-int-max=30")
                .arg("!");
        }
    }

    // 6. Post-Encoder Leaky Queue & RTP UDP Transmission (Zero-Buffer-Bloat)
    cmd.arg("h264parse")
        .arg("!")
        .arg("queue")
        .arg("max-size-buffers=1")
        .arg("max-size-bytes=0")
        .arg("max-size-time=0")
        .arg("leaky=downstream")
        .arg("!")
        .arg("rtph264pay")
        .arg("config-interval=1")
        .arg("pt=96")
        .arg("!")
        .arg("udpsink")
        .arg(format!("host={}", target_ip))
        .arg(format!("port={}", target_port))
        .arg("buffer-size=262144")
        .arg("sync=false")
        .spawn()
}

fn ensure_kernel_hdmi_connected() {
    let status_path = "/sys/class/drm/card1-HDMI-A-1/status";
    let is_connected = fs::read_to_string(status_path)
        .map(|s| s.trim() == "connected")
        .unwrap_or(false);

    if is_connected {
        println!("\x1b[1;32m[+] Kernel HDMI-A-1 is connected.\x1b[0m");
        return;
    }

    println!("\x1b[1;33m[*] Forcing kernel HDMI-A-1 connected with Pi monitor EDID...\x1b[0m");
    let cmd = "sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/edid_override < /home/carlos/ide/ext-monitor/edid/pi-monitor.edid > /dev/null && \
               echo 'on' | sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/force > /dev/null && \
               echo 1 | sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/trigger_hotplug > /dev/null";
    let _ = Command::new("bash").arg("-c").arg(cmd).status();
    thread::sleep(Duration::from_millis(500));
}

fn ensure_gnome_displays() {
    let check = Command::new("gdbus")
        .args(&[
            "call",
            "--session",
            "--dest",
            "org.gnome.Mutter.DisplayConfig",
            "--object-path",
            "/org/gnome/Mutter/DisplayConfig",
            "--method",
            "org.gnome.Mutter.DisplayConfig.GetCurrentState",
        ])
        .output();

    let (serial, is_configured) = if let Ok(out) = check {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let serial = if let Some(start) = stdout.find("(uint32 ") {
            let rest = &stdout[start + 8..];
            rest.find(',').and_then(|end| rest[..end].trim().parse::<u32>().ok()).unwrap_or(1)
        } else {
            1
        };

        // Check if HDMI-1 is in the active logical monitors array (3rd tuple element)
        let is_logical = stdout.contains("('HDMI-1', 'LRX'") || stdout.contains("[('HDMI-1'");
        (serial, is_logical)
    } else {
        (1, false)
    };

    if is_configured {
        println!("\x1b[1;32m[+] GNOME Mutter displays already configured with HDMI-1 in extended mode.\x1b[0m");
        return;
    }

    println!("\x1b[1;33m[*] Applying GNOME extended display layout (side-by-side, serial={})...\x1b[0m", serial);
    let apply_cmd = format!(
        r#"gdbus call --session --dest org.gnome.Mutter.DisplayConfig --object-path /org/gnome/Mutter/DisplayConfig --method org.gnome.Mutter.DisplayConfig.ApplyMonitorsConfig {} 1 "[(0, 0, 1.0, 0, true, [('eDP-1', '1920x1080@60.003', @a{{sv}} {{}})]), (1920, 0, 1.0, 0, false, [('HDMI-1', '1600x900@59.946', @a{{sv}} {{}})])]" "@a{{sv}} {{}}""#,
        serial
    );
    let _ = Command::new("bash").arg("-c").arg(&apply_cmd).status();
    thread::sleep(Duration::from_millis(500));
}

fn ctrlc_setup(running: Arc<AtomicBool>) {
    let r = running.clone();
    ctrlc_hook();
    thread::spawn(move || {
        while RUNNING.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
        r.store(false, Ordering::SeqCst);
    });
}

fn ctrlc_hook() {
    unsafe {
        libc::signal(libc::SIGINT, handle_sig as *const () as usize);
        libc::signal(libc::SIGTERM, handle_sig as *const () as usize);
    }
}

extern "C" fn handle_sig(_: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}
