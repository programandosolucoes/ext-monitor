use std::collections::HashMap;
use std::fs;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, Value};

static RUNNING: AtomicBool = AtomicBool::new(true);

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
    let bitrate = args
        .get(3)
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(8000);

    println!("\x1b[1;34m[*] Target:\x1b[0m {}:{}", target_ip, target_port);
    println!("\x1b[1;34m[*] Bitrate:\x1b[0m {} kbps (VA-API CBR, 60 FPS)", bitrate);

    // 1. Ensure kernel HDMI-A-1 connector is forced connected
    ensure_kernel_hdmi_connected();

    // 2. Ensure GNOME Mutter has HDMI-1 active in side-by-side extended layout
    ensure_gnome_displays();

    // 3. Setup signal handler
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

        // RecordMonitor for HDMI-1 (the dedicated extended second monitor)
        let empty_props: HashMap<&str, Value> = HashMap::new();
        let stream_reply = match dbus_conn.call_method(
            Some("org.gnome.Mutter.ScreenCast"),
            session_path.as_str(),
            Some("org.gnome.Mutter.ScreenCast.Session"),
            "RecordMonitor",
            &("HDMI-1", empty_props),
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] RecordMonitor('HDMI-1') failed: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let stream_path: OwnedObjectPath = stream_reply.body().deserialize()?;
        println!("\x1b[1;32m[+] HDMI-1 ScreenCast Stream created:\x1b[0m {}", stream_path);

        // Setup signal listener for PipeWireStreamAdded
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

        println!("\x1b[1;32m[+] PipeWire Node ID for HDMI-1:\x1b[0m {}", node_id);

        // Spawn GStreamer pipeline with autoconnect=false
        println!("\x1b[1;33m[*] Starting AMD GPU H.264 VA-API streaming pipeline...\x1b[0m");
        let mut child = match spawn_streamer(target_ip, target_port, bitrate) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to spawn streamer: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        // Allow GStreamer pipewiresrc to register its input port
        thread::sleep(Duration::from_millis(800));

        // Find the exact output port for HDMI-1 node_id and link it to ext-hdmi-sender
        link_hdmi_port_to_sender(node_id);

        println!("\x1b[1;32m[+] Second monitor HDMI-1 is streaming LIVE to Pi Zero at 60 FPS!\x1b[0m");

        // Supervise streaming process
        while running.load(Ordering::SeqCst) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("\x1b[1;33m[*] Streamer exited with status: {}. Restarting...\x1b[0m", status);
                    break;
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(250));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[!] Error monitoring streamer: {}\x1b[0m", e);
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

fn link_hdmi_port_to_sender(node_id: u32) {
    for _ in 0..10 {
        if let Ok(output) = Command::new("pw-dump").output() {
            if let Ok(dump) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(items) = dump.as_array() {
                    let mut hdmi_out_port = None;

                    for item in items {
                        if item["type"] == "PipeWire:Interface:Port" {
                            let props = &item["info"]["props"];
                            if props["node.id"] == node_id && props["port.direction"] == "out" {
                                hdmi_out_port = item["id"].as_u64();
                                break;
                            }
                        }
                    }

                    if let Some(out_port) = hdmi_out_port {
                        println!("\x1b[1;34m[*] Found HDMI-1 Output Port: {}\x1b[0m", out_port);
                        let status = Command::new("pw-link")
                            .arg(out_port.to_string())
                            .arg("ext-hdmi-sender:input_1")
                            .status();

                        if let Ok(s) = status {
                            if s.success() {
                                println!("\x1b[1;32m[+] Successfully linked HDMI-1 (port {}) -> ext-hdmi-sender!\x1b[0m", out_port);
                                return;
                            }
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(300));
    }

    eprintln!("\x1b[1;31m[!] Warning: Failed to link HDMI-1 port automatically after 3s.\x1b[0m");
}

fn spawn_streamer(target_ip: &str, target_port: u16, bitrate: u32) -> Result<Child, std::io::Error> {
    Command::new("gst-launch-1.0")
        .arg("-v")
        .arg("pipewiresrc")
        .arg("autoconnect=false")
        .arg("stream-properties=props,node.name=ext-hdmi-sender")
        .arg("do-timestamp=true")
        .arg("!")
        .arg("videoconvert")
        .arg("!")
        .arg("video/x-raw,format=NV12")
        .arg("!")
        .arg("vapostproc")
        .arg("!")
        .arg("vah264enc")
        .arg(format!("bitrate={}", bitrate))
        .arg("rate-control=cbr")
        .arg("b-frames=0")
        .arg("ref-frames=1")
        .arg("!")
        .arg("h264parse")
        .arg("!")
        .arg("rtph264pay")
        .arg("config-interval=1")
        .arg("pt=96")
        .arg("!")
        .arg("udpsink")
        .arg(format!("host={}", target_ip))
        .arg(format!("port={}", target_port))
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

    if let Ok(out) = check {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("HDMI-1") {
            println!("\x1b[1;32m[+] GNOME Mutter displays already configured with HDMI-1.\x1b[0m");
            return;
        }
    }

    println!("\x1b[1;33m[*] Applying GNOME extended display layout (side-by-side)...\x1b[0m");
    let apply_cmd = r#"gdbus call --session --dest org.gnome.Mutter.DisplayConfig --object-path /org/gnome/Mutter/DisplayConfig --method org.gnome.Mutter.DisplayConfig.ApplyMonitorsConfig 4 1 "[(0, 0, 1.0, 0, true, [('eDP-1', '1920x1080@60.003', @a{sv} {})]), (1920, 0, 1.0, 0, false, [('HDMI-1', '1280x720@60.000', @a{sv} {})])]" "@a{sv} {}""#;
    let _ = Command::new("bash").arg("-c").arg(apply_cmd).status();
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
