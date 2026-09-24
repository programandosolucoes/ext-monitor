use std::fs;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
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
    println!("\x1b[1;34m[*] Bitrate:\x1b[0m {} kbps (VA-API CBR)", bitrate);

    // 1. Ensure kernel HDMI-A-1 connector is connected
    ensure_kernel_hdmi_connected();

    // 2. Ensure GNOME DisplayConfig has HDMI-1 enabled side-by-side
    ensure_gnome_displays();

    // 3. Setup signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_setup(r);

    // 4. Run streamer loop
    while running.load(Ordering::SeqCst) {
        println!("\x1b[1;33m[*] Starting AMD GPU H.264 streaming pipeline (60 FPS)...\x1b[0m");

        let mut child = match spawn_streamer(target_ip, target_port, bitrate) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to spawn streamer: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        while running.load(Ordering::SeqCst) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("\x1b[1;33m[*] Streamer exited with status: {}. Restarting in 1s...\x1b[0m", status);
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

        if !running.load(Ordering::SeqCst) {
            println!("\x1b[1;33m[*] Shutting down streamer...\x1b[0m");
            let _ = child.kill();
            let _ = child.wait();
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("\x1b[1;32m[*] ext-sender terminated cleanly.\x1b[0m");
}

fn ensure_kernel_hdmi_connected() {
    let status_path = "/sys/class/drm/card1-HDMI-A-1/status";
    let is_connected = fs::read_to_string(status_path)
        .map(|s| s.trim() == "connected")
        .unwrap_or(false);

    if is_connected {
        println!("\x1b[1;32m[+] Kernel HDMI-A-1 is already connected.\x1b[0m");
        return;
    }

    println!("\x1b[1;33m[*] Forcing kernel HDMI-A-1 connected with Pi monitor EDID...\x1b[0m");
    let cmd = "sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/edid_override < /tmp/pi-monitor.edid > /dev/null && \
               echo 'on' | sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/force > /dev/null && \
               echo 1 | sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/trigger_hotplug > /dev/null";
    let _ = Command::new("bash").arg("-c").arg(cmd).status();
    thread::sleep(Duration::from_millis(500));
}

fn ensure_gnome_displays() {
    // Check if HDMI-1 is present in GetCurrentState
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
        if stdout.contains("HDMI-1") && stdout.contains("1600") {
            println!("\x1b[1;32m[+] GNOME Mutter displays already configured with HDMI-1.\x1b[0m");
            return;
        }
    }

    println!("\x1b[1;33m[*] Applying GNOME side-by-side display layout (1920x1080 + 1600x900)...\x1b[0m");
    let apply_cmd = r#"gdbus call --session --dest org.gnome.Mutter.DisplayConfig --object-path /org/gnome/Mutter/DisplayConfig --method org.gnome.Mutter.DisplayConfig.ApplyMonitorsConfig 4 1 "[(0, 0, 1.0, 0, true, [('eDP-1', '1920x1080@60.003', @a{sv} {})]), (1920, 0, 1.0, 0, false, [('HDMI-1', '1600x900@59.946', @a{sv} {})])]" "@a{sv} {}""#;
    let _ = Command::new("bash").arg("-c").arg(apply_cmd).status();
    thread::sleep(Duration::from_millis(500));
}

fn spawn_streamer(target_ip: &str, target_port: u16, bitrate: u32) -> Result<Child, std::io::Error> {
    // Pipeline:
    // ximagesrc startx=1920 endx=3519 starty=0 endy=899 (Rectangle of HDMI-1)
    // -> videoconvert -> NV12
    // -> vah264enc (AMD Radeon 610M Hardware Encoder)
    // -> rtph264pay -> udpsink
    Command::new("gst-launch-1.0")
        .arg("-v")
        .arg("ximagesrc")
        .arg("startx=1920")
        .arg("endx=3519")
        .arg("starty=0")
        .arg("endy=899")
        .arg("use-damage=0")
        .arg("!")
        .arg("videoconvert")
        .arg("!")
        .arg("video/x-raw,format=NV12,framerate=60/1")
        .arg("!")
        .arg("vah264enc")
        .arg("rate-control=cbr")
        .arg(format!("bitrate={}", bitrate))
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
