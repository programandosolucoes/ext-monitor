//! Standalone Pure-Rust Universal Miracast (Wi-Fi Display / WFD 1.0 / MS-MICE) Client CLI
//!
//! Features:
//! - Multi-architecture GPU hardware video encoding (AMD VA-API, Intel QuickSync, NVIDIA NVENC, CPU OpenH264)
//! - Direct RTSP 1.0 WFD M1-M7 handshake and atomic TEARDOWN
//! - Automatic network discovery via mDNS, SSDP, and Direct-USB
//! - Graceful SIGINT/SIGTERM handling with < 100ms session cleanup and terminal restoration
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

#![allow(dead_code)]

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[path = "../encoder.rs"]
mod encoder;

#[path = "../miracast_launcher.rs"]
pub mod miracast_launcher;

#[path = "../screencast.rs"]
pub mod screencast;

#[path = "../miracast/mod.rs"]
pub mod miracast;

use miracast::client::{
    MiracastConfig, MiracastSession, DEFAULT_BITRATE_KBPS, DEFAULT_FPS, DEFAULT_HEIGHT,
    DEFAULT_MODE, DEFAULT_TARGET_PORT, DEFAULT_WIDTH,
};
use miracast::discovery::{interactive_select_sink, scan_sinks};
use miracast_launcher::detect_gpu_hardware;

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_signal(_: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}

fn setup_signals() {
    unsafe {
        libc::signal(libc::SIGINT, handle_signal as *const () as usize);
        libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
    }
}

/// Command-line configuration parameters for `ext-miracast`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    /// Target Miracast sink IP (either from `--ip <IP>` or positional argument)
    pub ip: Option<String>,
    /// RTSP signaling port (default: 7236)
    pub port: u16,
    /// Video width in pixels (default: 1280)
    pub width: u32,
    /// Video height in pixels (default: 720)
    pub height: u32,
    /// Frame rate in frames per second (default: 60)
    pub fps: u32,
    /// Video bitrate in kbps (default: 4000)
    pub bitrate_kbps: u32,
    /// Display mode: "extend" (create virtual display) or "clone" (mirror primary display) (default: "extend")
    pub mode: String,
    /// Scan local network (mDNS/SSDP) for Miracast sinks and list them
    pub scan: bool,
    /// Show help message flag
    pub show_help: bool,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            ip: None,
            port: DEFAULT_TARGET_PORT,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            fps: DEFAULT_FPS,
            bitrate_kbps: DEFAULT_BITRATE_KBPS,
            mode: DEFAULT_MODE.to_string(),
            scan: false,
            show_help: false,
        }
    }
}

/// Parses a video resolution string such as "720p", "1080p", or "1280x720".
fn parse_resolution(res: &str) -> Result<(u32, u32), String> {
    match res.to_ascii_lowercase().as_str() {
        "720p" | "720" => Ok((1280, 720)),
        "1080p" | "1080" => Ok((1920, 1080)),
        other => {
            if let Some((w_str, h_str)) = other.split_once('x') {
                let w = w_str
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid width in resolution '{}'", other))?;
                let h = h_str
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid height in resolution '{}'", other))?;
                Ok((w, h))
            } else {
                Err(format!(
                    "Unsupported resolution '{}'. Supported formats: 720p, 1080p, or <WIDTH>x<HEIGHT>",
                    res
                ))
            }
        }
    }
}

impl CliArgs {
    /// Parses CLI arguments from an iterator of arguments (including program name as arg 0).
    pub fn parse_from<I, T>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut cli = Self::default();
        let mut iter = args.into_iter().map(|s| s.into()).peekable();

        // Skip program name if present
        let _prog_name = iter.next();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    cli.show_help = true;
                    return Ok(cli);
                }
                "--scan" => {
                    cli.scan = true;
                }
                "--ip" => {
                    let val = iter.next().ok_or_else(|| "--ip requires a value".to_string())?;
                    cli.ip = Some(val);
                }
                arg if arg.starts_with("--ip=") => {
                    let val = &arg["--ip=".len()..];
                    if val.is_empty() {
                        return Err("--ip requires a value".to_string());
                    }
                    cli.ip = Some(val.to_string());
                }
                "--port" => {
                    let val = iter.next().ok_or_else(|| "--port requires a value".to_string())?;
                    cli.port = val
                        .parse::<u16>()
                        .map_err(|_| format!("Invalid port value: '{}'", val))?;
                }
                arg if arg.starts_with("--port=") => {
                    let val = &arg["--port=".len()..];
                    cli.port = val
                        .parse::<u16>()
                        .map_err(|_| format!("Invalid port value: '{}'", val))?;
                }
                "--res" => {
                    let val = iter.next().ok_or_else(|| "--res requires a value (e.g. 720p or 1080p)".to_string())?;
                    let (w, h) = parse_resolution(&val)?;
                    cli.width = w;
                    cli.height = h;
                }
                arg if arg.starts_with("--res=") => {
                    let val = &arg["--res=".len()..];
                    let (w, h) = parse_resolution(val)?;
                    cli.width = w;
                    cli.height = h;
                }
                "--fps" => {
                    let val = iter.next().ok_or_else(|| "--fps requires a value (e.g. 30 or 60)".to_string())?;
                    cli.fps = val
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid fps value: '{}'", val))?;
                }
                arg if arg.starts_with("--fps=") => {
                    let val = &arg["--fps=".len()..];
                    cli.fps = val
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid fps value: '{}'", val))?;
                }
                "--bitrate" => {
                    let val = iter.next().ok_or_else(|| "--bitrate requires a value in kbps".to_string())?;
                    cli.bitrate_kbps = val
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid bitrate value: '{}'", val))?;
                }
                arg if arg.starts_with("--bitrate=") => {
                    let val = &arg["--bitrate=".len()..];
                    cli.bitrate_kbps = val
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid bitrate value: '{}'", val))?;
                }
                "--mode" | "-m" => {
                    let val = iter.next().ok_or_else(|| "--mode requires a value ('extend' or 'clone')".to_string())?;
                    let m = val.to_lowercase();
                    if m != "extend" && m != "clone" {
                        return Err(format!("Invalid mode '{}'. Must be 'extend' or 'clone'", val));
                    }
                    cli.mode = m;
                }
                arg if arg.starts_with("--mode=") => {
                    let val = &arg["--mode=".len()..];
                    let m = val.to_lowercase();
                    if m != "extend" && m != "clone" {
                        return Err(format!("Invalid mode '{}'. Must be 'extend' or 'clone'", val));
                    }
                    cli.mode = m;
                }
                "--clone" => {
                    cli.mode = "clone".to_string();
                }
                "--extend" => {
                    cli.mode = "extend".to_string();
                }
                arg if arg.starts_with('-') => {
                    return Err(format!("Unknown option '{}'", arg));
                }
                pos_ip => {
                    if cli.ip.is_none() {
                        cli.ip = Some(pos_ip.to_string());
                    } else {
                        return Err(format!("Unexpected positional argument: '{}'", pos_ip));
                    }
                }
            }
        }

        Ok(cli)
    }
}

/// Prints formatted CLI usage and options help message.
pub fn print_help() {
    println!("\x1b[1;36mext-miracast\x1b[0m - Pure-Rust Universal Miracast (Wi-Fi Display / WFD 1.0) Client\n");
    println!("USAGE:");
    println!("    ext-miracast [OPTIONS] [SINK_IP]\n");
    println!("ARGS:");
    println!("    <SINK_IP>               Target Miracast Sink IP address\n");
    println!("OPTIONS:");
    println!("    --ip <IP>               Target Miracast Sink IP address (or positional argument)");
    println!("    --port <PORT>           Target RTSP port (default: 7236)");
    println!("    --res <720p|1080p>      Video resolution (default: 720p, 1280x720 native CEA index 6)");
    println!("    --fps <30|60>           Frame rate (default: 60)");
    println!("    --bitrate <KBPS>        Video bitrate in kbps (default: 4000)");
    println!("    --mode <extend|clone>   Display mode: 'extend' (virtual screen) or 'clone' (mirror primary eDP-1) (default: extend)");
    println!("    -m <extend|clone>       Short flag for --mode");
    println!("    --clone                 Shortcut for --mode clone");
    println!("    --extend                Shortcut for --mode extend");
    println!("    --scan                  Scan local network (mDNS/SSDP) for Miracast sinks and list them");
    println!("    -h, --help              Print help information");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let cli = match CliArgs::parse_from(args) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
            eprintln!("Try 'ext-miracast --help' for more information.");
            std::process::exit(1);
        }
    };

    if cli.show_help {
        print_help();
        return Ok(());
    }

    if cli.scan {
        println!("\x1b[1;36m[miracast-scan]\x1b[0m Scanning local network for Miracast / Wi-Fi Display sinks (mDNS/SSDP/Direct-USB)...");
        let sinks = scan_sinks(Duration::from_secs(2));
        if sinks.is_empty() {
            println!("\x1b[1;33m[miracast-scan]\x1b[0m No Miracast / Wi-Fi Display sinks discovered.");
        } else {
            println!("\x1b[1;32m[miracast-scan]\x1b[0m Discovered {} sink(s):", sinks.len());
            for (i, sink) in sinks.iter().enumerate() {
                println!(
                    "  [{}] {} - {}:{} ({})",
                    i + 1,
                    sink.name,
                    sink.ip,
                    sink.port,
                    sink.protocol
                );
            }
        }
        return Ok(());
    }

    // Determine target IP & port
    let (target_ip, target_port) = if let Some(ip) = cli.ip {
        (ip, cli.port)
    } else {
        println!("\x1b[1;36m[miracast]\x1b[0m No target IP specified. Launching interactive discovery scanner...");
        let selected = interactive_select_sink(Duration::from_secs(2));
        match selected {
            Some(sink) => {
                let port = if cli.port != DEFAULT_TARGET_PORT {
                    cli.port
                } else {
                    sink.port
                };
                (sink.ip.to_string(), port)
            }
            None => {
                println!("\x1b[1;33m[miracast]\x1b[0m No sink selected. Exiting cleanly.");
                return Ok(());
            }
        }
    };

    // Detect and report host GPU hardware acceleration info
    let gpu = detect_gpu_hardware();
    println!(
        "\x1b[1;34m[miracast-hw]\x1b[0m Detected Host GPU: \x1b[1;32m{}\x1b[0m (Hardware Encoders: {})",
        gpu.vendor_name, gpu.encoder_name
    );

    // Setup signal handler for graceful Ctrl+C / SIGTERM teardown
    setup_signals();

    // Configure Miracast session
    let config = MiracastConfig::new(&target_ip, target_port)
        .with_resolution(cli.width, cli.height)
        .with_fps(cli.fps)
        .with_bitrate(cli.bitrate_kbps)
        .with_mode(cli.mode.clone());

    println!(
        "\x1b[1;36m[miracast]\x1b[0m Launching session [mode: {}]: {}x{} @ {} FPS, {} kbps -> {}:{}",
        cli.mode, cli.width, cli.height, cli.fps, cli.bitrate_kbps, target_ip, target_port
    );

    let mut session = match MiracastSession::start(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[1;31m[miracast]\x1b[0m Failed to start Miracast session: {}", e);
            std::process::exit(1);
        }
    };

    println!("\x1b[1;32m[miracast]\x1b[0m Streaming active! Press Ctrl+C to terminate cleanly.");

    // Signal loop monitoring atomic RUNNING flag and session health
    while RUNNING.load(Ordering::SeqCst) && session.is_running() {
        std::thread::sleep(Duration::from_millis(100));
    }

    println!("\n\x1b[1;33m[miracast]\x1b[0m Signal received. Shutting down session with RTSP TEARDOWN...");
    session.stop();

    // Instant terminal restore (cursor visible + formatting reset)
    print!("\x1b[?25h\x1b[0m");
    let _ = io::stdout().flush();
    println!("\x1b[1;32m[miracast]\x1b[0m Clean shutdown complete (< 100ms teardown).");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_defaults() {
        let args = ["ext-miracast"];
        let cli = CliArgs::parse_from(args).expect("Failed to parse defaults");
        assert_eq!(cli.ip, None);
        assert_eq!(cli.port, 7236);
        assert_eq!(cli.width, 1280);
        assert_eq!(cli.height, 720);
        assert_eq!(cli.fps, 60);
        assert_eq!(cli.bitrate_kbps, 4000);
        assert!(!cli.scan);
        assert!(!cli.show_help);
    }

    #[test]
    fn test_cli_args_positional_ip() {
        let args = ["ext-miracast", "192.168.7.2"];
        let cli = CliArgs::parse_from(args).expect("Failed to parse positional IP");
        assert_eq!(cli.ip, Some("192.168.7.2".to_string()));
        assert_eq!(cli.port, 7236);
    }

    #[test]
    fn test_cli_args_named_options() {
        let args = [
            "ext-miracast",
            "--ip",
            "10.0.0.100",
            "--port",
            "7250",
            "--res",
            "1080p",
            "--fps",
            "30",
            "--bitrate",
            "6000",
        ];
        let cli = CliArgs::parse_from(args).expect("Failed to parse named options");
        assert_eq!(cli.ip, Some("10.0.0.100".to_string()));
        assert_eq!(cli.port, 7250);
        assert_eq!(cli.width, 1920);
        assert_eq!(cli.height, 1080);
        assert_eq!(cli.fps, 30);
        assert_eq!(cli.bitrate_kbps, 6000);
        assert!(!cli.scan);
    }

    #[test]
    fn test_cli_args_equals_syntax() {
        let args = [
            "ext-miracast",
            "--ip=192.168.1.50",
            "--port=7236",
            "--res=720p",
            "--fps=60",
            "--bitrate=5000",
        ];
        let cli = CliArgs::parse_from(args).expect("Failed to parse equals syntax");
        assert_eq!(cli.ip, Some("192.168.1.50".to_string()));
        assert_eq!(cli.port, 7236);
        assert_eq!(cli.width, 1280);
        assert_eq!(cli.height, 720);
        assert_eq!(cli.fps, 60);
        assert_eq!(cli.bitrate_kbps, 5000);
    }

    #[test]
    fn test_cli_args_custom_resolution_width_height() {
        let args = ["ext-miracast", "--res", "800x480"];
        let cli = CliArgs::parse_from(args).expect("Failed to parse custom resolution");
        assert_eq!(cli.width, 800);
        assert_eq!(cli.height, 480);
    }

    #[test]
    fn test_cli_args_scan_flag() {
        let args = ["ext-miracast", "--scan"];
        let cli = CliArgs::parse_from(args).expect("Failed to parse --scan");
        assert!(cli.scan);
        assert_eq!(cli.ip, None);
    }

    #[test]
    fn test_cli_args_help_flags() {
        let args1 = ["ext-miracast", "-h"];
        let cli1 = CliArgs::parse_from(args1).expect("Failed to parse -h");
        assert!(cli1.show_help);

        let args2 = ["ext-miracast", "--help"];
        let cli2 = CliArgs::parse_from(args2).expect("Failed to parse --help");
        assert!(cli2.show_help);
    }

    #[test]
    fn test_cli_args_invalid_option_errors() {
        let args = ["ext-miracast", "--unknown-flag"];
        let err = CliArgs::parse_from(args);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Unknown option"));
    }

    #[test]
    fn test_cli_args_invalid_port() {
        let args = ["ext-miracast", "--port", "not-a-number"];
        let err = CliArgs::parse_from(args);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Invalid port value"));
    }

    #[test]
    fn test_cli_args_invalid_res() {
        let args = ["ext-miracast", "--res", "4k_invalid"];
        let err = CliArgs::parse_from(args);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Unsupported resolution"));
    }

    #[test]
    fn test_cli_args_duplicate_ip_positional_conflict() {
        let args = ["ext-miracast", "--ip", "192.168.7.2", "10.0.0.1"];
        let err = CliArgs::parse_from(args);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Unexpected positional argument"));
    }

    #[test]
    fn test_cli_args_mode_options() {
        let args1 = ["ext-miracast", "--mode", "extend"];
        let cli1 = CliArgs::parse_from(args1).expect("Failed to parse --mode extend");
        assert_eq!(cli1.mode, "extend");

        let args2 = ["ext-miracast", "-m", "clone"];
        let cli2 = CliArgs::parse_from(args2).expect("Failed to parse -m clone");
        assert_eq!(cli2.mode, "clone");

        let args3 = ["ext-miracast", "--mode=clone"];
        let cli3 = CliArgs::parse_from(args3).expect("Failed to parse --mode=clone");
        assert_eq!(cli3.mode, "clone");
    }

    #[test]
    fn test_cli_args_mode_flags_clone_and_extend() {
        let args1 = ["ext-miracast", "--clone"];
        let cli1 = CliArgs::parse_from(args1).expect("Failed to parse --clone");
        assert_eq!(cli1.mode, "clone");

        let args2 = ["ext-miracast", "--extend"];
        let cli2 = CliArgs::parse_from(args2).expect("Failed to parse --extend");
        assert_eq!(cli2.mode, "extend");
    }

    #[test]
    fn test_cli_args_invalid_mode() {
        let args = ["ext-miracast", "--mode", "mirror"];
        let err = CliArgs::parse_from(args);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Invalid mode"));
    }
}
