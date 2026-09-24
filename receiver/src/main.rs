use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    println!("\x1b[1;32m[ext-receiver]\x1b[0m Starting Raspberry Pi Zero GPU Display Receiver");

    let args: Vec<String> = env::args().collect();
    let port = args
        .get(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5000);

    println!("\x1b[1;34m[ext-receiver]\x1b[0m Listening on UDP port {}", port);

    // Setup signal handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_setup(r);

    // Pipeline: udpsrc -> rtph264depay -> h264parse -> v4l2h264dec (VideoCore IV hardware) -> kmssink (KMS DRM HDMI)
    let caps_arg = format!(
        "caps=application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96"
    );

    while running.load(Ordering::SeqCst) {
        println!("\x1b[1;33m[ext-receiver]\x1b[0m Launching hardware decode pipeline...");

        let mut child = match Command::new("gst-launch-1.0")
            .arg("-v")
            .arg("udpsrc")
            .arg(format!("port={}", port))
            .arg(&caps_arg)
            .arg("!")
            .arg("rtph264depay")
            .arg("!")
            .arg("h264parse")
            .arg("!")
            .arg("v4l2h264dec")
            .arg("capture-io-mode=dmabuf")
            .arg("!")
            .arg("kmssink")
            .arg("sync=false")
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to spawn gst-launch: {}. Retrying in 2s...", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        // Wait for child or termination signal
        while running.load(Ordering::SeqCst) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!(
                        "\x1b[1;33m[ext-receiver]\x1b[0m Pipeline exited with status: {}. Restarting in 1s...",
                        status
                    );
                    break;
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(200));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Error checking child status: {}", e);
                    break;
                }
            }
        }

        if !running.load(Ordering::SeqCst) {
            println!("\x1b[1;33m[ext-receiver]\x1b[0m Shutting down pipeline...");
            let _ = child.kill();
            let _ = child.wait();
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("\x1b[1;32m[ext-receiver]\x1b[0m Receiver terminated cleanly.");
}

fn ctrlc_setup(running: Arc<AtomicBool>) {
    unsafe {
        libc_signal(libc::SIGINT, handle_sig);
        libc_signal(libc::SIGTERM, handle_sig);
    }
    let r = running.clone();
    thread::spawn(move || {
        while RUNNING.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
        r.store(false, Ordering::SeqCst);
    });
}

extern "C" fn handle_sig(_: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}

unsafe fn libc_signal(sig: libc::c_int, handler: extern "C" fn(libc::c_int)) {
    libc::signal(sig, handler as usize);
}
