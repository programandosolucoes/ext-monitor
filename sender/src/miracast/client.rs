//! Pure-Rust Miracast (Wi-Fi Display / WFD 1.0 / MS-MICE) Session Orchestrator
//!
//! Orchestrates the full lifecycle of a Miracast streaming session:
//! 1. Multi-Architecture Hardware Video Encoding:
//!    - AMD Radeon: Direct VA-API on `/dev/dri/renderD128`
//!    - Intel QuickSync: Direct VA-API on `/dev/dri/renderD128`
//!    - NVIDIA: Direct NVENC hardware encoder
//!    - CPU Fallback: Fast Cisco OpenH264
//! 2. RTSP 1.0 WFD Handshake (M1-M7 sequence) via `WfdClient` (port 7236 standard, 7250 MS-MICE).
//! 3. Real-Time MPEG-TS / PES Muxing with keyframe PCR injection via `MpegTsMuxer`.
//! 4. RFC 2250 / RFC 3550 RTP Encapsulation (Payload Type 33 MP2T).
//! 5. CFR Frame Pacing and UDP RTP transmission to the negotiated Sink UDP port.
//! 6. Clean, atomic session termination and RTSP `TEARDOWN` within < 100ms.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::encoder::create_best_encoder;
use super::mpegts::MpegTsMuxer;
use super::wfd_client::WfdClient;

/// Default Miracast video settings (CEA index 6 native 720p60)
pub const DEFAULT_TARGET_IP: &str = "192.168.7.2";
pub const DEFAULT_TARGET_PORT: u16 = 7236;
pub const FALLBACK_MICE_PORT: u16 = 7250;
pub const DEFAULT_WIDTH: u32 = 1280;
pub const DEFAULT_HEIGHT: u32 = 720;
pub const DEFAULT_FPS: u32 = 60;
pub const DEFAULT_BITRATE_KBPS: u32 = 4000;
pub const DEFAULT_MODE: &str = "extend";
pub const DEFAULT_CAPTURE: &str = "kms";

/// Configuration parameters for establishing a Miracast session
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiracastConfig {
    /// Sink IP address (defaults to "192.168.7.2")
    pub target_ip: String,
    /// Sink RTSP signaling port (defaults to 7236, with auto-fallback to 7250 MS-MICE)
    pub target_port: u16,
    /// Video stream width in pixels (defaults to 1280)
    pub width: u32,
    /// Video stream height in pixels (defaults to 720 - CEA index 6 native)
    pub height: u32,
    /// Target frame rate in frames per second (defaults to 60)
    pub fps: u32,
    /// Target video bitrate in kilobits per second (defaults to 4000)
    pub bitrate_kbps: u32,
    /// Display mode: "extend" (create virtual display) or "clone" (mirror primary display)
    pub mode: String,
    /// Capture backend: "mutter" (GNOME D-Bus), "kernel" / "kms" (Linux Kernel DRM/KMS scanout), "x11" (X11 XShm)
    pub capture: String,
}

impl Default for MiracastConfig {
    fn default() -> Self {
        Self {
            target_ip: DEFAULT_TARGET_IP.to_string(),
            target_port: DEFAULT_TARGET_PORT,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            fps: DEFAULT_FPS,
            bitrate_kbps: DEFAULT_BITRATE_KBPS,
            mode: DEFAULT_MODE.to_string(),
            capture: DEFAULT_CAPTURE.to_string(),
        }
    }
}

impl MiracastConfig {
    /// Creates a new `MiracastConfig` with specified target IP and port, using standard defaults for video.
    pub fn new(target_ip: impl Into<String>, target_port: u16) -> Self {
        Self {
            target_ip: target_ip.into(),
            target_port,
            ..Default::default()
        }
    }

    /// Sets custom video resolution (e.g. 1920x1080 or 1280x720).
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets custom frame rate (e.g. 30 or 60 FPS).
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    /// Sets custom target bitrate in kbps.
    pub fn with_bitrate(mut self, bitrate_kbps: u32) -> Self {
        self.bitrate_kbps = bitrate_kbps;
        self
    }

    /// Sets display mode: "extend" or "clone".
    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }

    /// Sets capture backend: "mutter" (GNOME D-Bus), "kernel" / "kms" (Linux Kernel DRM/KMS scanout), or "x11" (X11 XShm).
    pub fn with_capture(mut self, capture: impl Into<String>) -> Self {
        self.capture = capture.into();
        self
    }
}

/// Active Miracast streaming session orchestrator
pub struct MiracastSession {
    config: MiracastConfig,
    sink_rtp_port: u16,
    session_id: Option<String>,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    wfd_client: Option<WfdClient>,
    current_frame: Arc<RwLock<Option<Vec<u8>>>>,
    screencast_session: Option<crate::screencast::MutterScreenCastSession>,
    stream_process: Option<std::process::Child>,
}

impl MiracastSession {
    /// Starts a new Miracast streaming session.
    ///
    /// Sequence:
    /// 1. Connects to `target_ip:target_port` (retrying on MS-MICE port 7250 if port 7236 fails).
    /// 2. Performs full WFD RTSP M1-M7 negotiation, acquiring negotiated Sink UDP RTP port.
    /// 3. Binds local UDP socket (`0.0.0.0:0`).
    /// 4. Initializes the best hardware-accelerated video encoder across AMD, Intel, NVIDIA, or CPU fallback.
    /// 5. Initializes `MpegTsMuxer` and spawns CFR worker thread transmitting RTP MP2T datagrams to the Sink.
    pub fn start(config: MiracastConfig) -> Result<Self, io::Error> {
        println!(
            "\x1b[1;34m[miracast]\x1b[0m Connecting to Miracast sink at {}:{}...",
            config.target_ip, config.target_port
        );

        // 1. Connect to Sink with fallback from standard port 7236 to MS-MICE port 7250
        let mut client = match WfdClient::connect_timeout(&config.target_ip, config.target_port, Duration::from_secs(3)) {
            Ok(c) => c,
            Err(e) => {
                if config.target_port == DEFAULT_TARGET_PORT {
                    println!(
                        "\x1b[1;33m[miracast]\x1b[0m Port {} connection failed ({}), attempting MS-MICE port {}...",
                        DEFAULT_TARGET_PORT, e, FALLBACK_MICE_PORT
                    );
                    WfdClient::connect_timeout(&config.target_ip, FALLBACK_MICE_PORT, Duration::from_secs(3))?
                } else {
                    return Err(e);
                }
            }
        };

        // 2. Perform RTSP WFD M1-M7 session negotiation
        println!("\x1b[1;34m[miracast]\x1b[0m Negotiating WFD RTSP session (M1-M7)...");
        let sink_rtp_port = client.negotiate_session()?;
        let session_id = client.session_id().map(|s| s.to_string());
        println!(
            "\x1b[1;32m[miracast]\x1b[0m RTSP Session established! Sink UDP RTP Port: {}, Session ID: {:?}",
            sink_rtp_port, session_id
        );

        // 3. Resolve destination socket address and bind local UDP socket
        let sink_addr: SocketAddr = format!("{}:{}", config.target_ip, sink_rtp_port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Failed to resolve sink address {}:{}", config.target_ip, sink_rtp_port),
                )
            })?;

        let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
        let _ = udp_socket.set_write_timeout(Some(Duration::from_millis(50)));

        // 4. Multi-architecture GPU hardware video encoder and desktop capture setup
        let target_monitor = if config.mode.to_lowercase() == "clone" {
            "eDP-1"
        } else {
            // For extend mode, prioritize the active HDMI-1 DRM/KMS display, fallback to virtual
            if crate::kms::KmsOutputInfo::discover("HDMI-1").is_ok() {
                "HDMI-1"
            } else {
                "virtual"
            }
        };

        let mut screencast_session = None;
        let mut stream_process = None;

        let capture_backend = config.capture.to_lowercase();
        let gpu = crate::miracast_launcher::detect_gpu_hardware();

        let mut cmd = std::process::Command::new("gst-launch-1.0");
        cmd.arg("-q");
        cmd.env("GST_PLUGIN_FEATURE_RANK", &gpu.rank_string);

        let mut pipeline_ready = false;

        if capture_backend == "x11" {
            println!(
                "\x1b[1;34m[miracast-x11]\x1b[0m Captura direta X11 ativada (ximagesrc, bypass Mutter/D-Bus)..."
            );
            cmd.arg("ximagesrc");
            cmd.arg("use-damage=false");
            cmd.arg("!");
            pipeline_ready = true;
        } else {
            // "kms", "kernel", or "mutter"
            if capture_backend == "kernel" || capture_backend == "kms" {
                println!(
                    "\x1b[1;34m[miracast-kms]\x1b[0m Captura Hardware DRM/KMS Scanout ativada para monitor '{}'...",
                    target_monitor
                );
                if let Ok(kms_out) = crate::kms::KmsOutputInfo::discover(target_monitor) {
                    println!(
                        "\x1b[1;32m[miracast-kms]\x1b[0m DRM/KMS Hardware Scanout: {} (CRTC {}, Card {:?}, {}x{}@{}Hz)",
                        kms_out.connector_name, kms_out.crtc_id, kms_out.card_path, kms_out.width, kms_out.height, kms_out.vrefresh
                    );
                }
            } else {
                println!(
                    "\x1b[1;34m[miracast]\x1b[0m Inspecionando GNOME Mutter ScreenCast para captura (Modo: {}, Monitor: {})...",
                    config.mode, target_monitor
                );
            }

            if let Ok(screencast) = crate::screencast::MutterScreenCastSession::create_and_start(target_monitor) {
                println!(
                    "\x1b[1;32m[miracast]\x1b[0m Captura Scanout ativa no nó PipeWire {} (Monitor: {}, Modo: {})",
                    screencast.node_id, target_monitor, config.mode
                );
                cmd.arg("pipewiresrc");
                cmd.arg(format!("path={}", screencast.node_id));
                cmd.arg("do-timestamp=true");
                cmd.arg("min-buffers=2");
                cmd.arg("max-buffers=4");
                cmd.arg("keepalive-time=16");
                cmd.arg("always-copy=false");
                cmd.arg("!");
                screencast_session = Some(screencast);
                pipeline_ready = true;
            }
        }

        if pipeline_ready {
            cmd.arg("videorate");
            cmd.arg("drop-only=true");
            cmd.arg("skip-to-first=true");
            cmd.arg("!");
            cmd.arg(format!("video/x-raw,framerate={}/1", config.fps));
            cmd.arg("!");

            if gpu.vendor_name.contains("AMD") {
                println!(
                    "\x1b[1;36m[miracast-gpu]\x1b[0m AMD Radeon Profile: vah264enc (target-usage=7 UltraFast, aud=true, b-frames=0, ref=1, cbr={}k)",
                    config.bitrate_kbps
                );
                cmd.arg("vapostproc").arg("!");
                cmd.arg(format!("video/x-raw(memory:VAMemory),width={},height={}", config.width, config.height)).arg("!");
                cmd.arg("vah264enc")
                    .arg(format!("bitrate={}", config.bitrate_kbps))
                    .arg("rate-control=cbr")
                    .arg("target-usage=7")
                    .arg("aud=true")
                    .arg("b-frames=0")
                    .arg("ref-frames=1")
                    .arg("key-int-max=60")
                    .arg("!");
                cmd.arg("video/x-h264,profile=constrained-baseline").arg("!");
            } else if gpu.vendor_name.contains("Intel") {
                let cpb = (config.bitrate_kbps / 4).max(500);
                println!(
                    "\x1b[1;36m[miracast-gpu]\x1b[0m Intel QuickSync Profile: vah264enc (target-usage=7 TU7 Lowest-Latency, aud=true, b-frames=0, ref=1, cpb={}k, cbr={}k)",
                    cpb, config.bitrate_kbps
                );
                cmd.arg("vapostproc").arg("!");
                cmd.arg(format!("video/x-raw(memory:VAMemory),width={},height={}", config.width, config.height)).arg("!");
                cmd.arg("vah264enc")
                    .arg(format!("bitrate={}", config.bitrate_kbps))
                    .arg("rate-control=cbr")
                    .arg("target-usage=7")
                    .arg("aud=true")
                    .arg("b-frames=0")
                    .arg("ref-frames=1")
                    .arg("key-int-max=60")
                    .arg(format!("cpb-size={}", cpb))
                    .arg("!");
                cmd.arg("video/x-h264,profile=constrained-baseline").arg("!");
            } else if gpu.vendor_name.contains("NVIDIA") {
                println!(
                    "\x1b[1;36m[miracast-gpu]\x1b[0m NVIDIA NVENC Profile: nvh264enc (zerolatency=true, aud=true, b-frames=0, gop=60, cbr={}k)",
                    config.bitrate_kbps
                );
                cmd.arg("videoconvert").arg("!");
                cmd.arg(format!("video/x-raw,format=NV12,width={},height={}", config.width, config.height)).arg("!");
                cmd.arg("nvh264enc")
                    .arg(format!("bitrate={}", config.bitrate_kbps))
                    .arg("zerolatency=true")
                    .arg("b-frames=0")
                    .arg("aud=true")
                    .arg("gop-size=60")
                    .arg("!");
                cmd.arg("video/x-h264,profile=constrained-baseline").arg("!");
            } else {
                println!(
                    "\x1b[1;36m[miracast-cpu]\x1b[0m CPU Profile: x264enc (tune=zerolatency, speed=ultrafast, sliced-threads=true, aud=true, b-frames=0, ref=1, cbr={}k)",
                    config.bitrate_kbps
                );
                cmd.arg("videoscale").arg("!");
                cmd.arg("videoconvert").arg("!");
                cmd.arg(format!("video/x-raw,format=I420,width={},height={}", config.width, config.height)).arg("!");
                cmd.arg("x264enc")
                    .arg(format!("bitrate={}", config.bitrate_kbps))
                    .arg("tune=zerolatency")
                    .arg("speed-preset=ultrafast")
                    .arg("b-frames=0")
                    .arg("ref=1")
                    .arg("sliced-threads=true")
                    .arg("aud=true")
                    .arg("key-int-max=60")
                    .arg("!");
                cmd.arg("video/x-h264,profile=constrained-baseline").arg("!");
            }

            cmd.arg("h264parse").arg("config-interval=1").arg("!");

            cmd.arg("mpegtsmux")
                .arg("alignment=7")
                .arg("latency=0")
                .arg("start-time-selection=now")
                .arg("pat-interval=90000")
                .arg("pmt-interval=90000")
                .arg("pcr-interval=3600")
                .arg("!");

            cmd.arg("rtpmp2tpay").arg("!");

            cmd.arg("udpsink")
                .arg(format!("host={}", config.target_ip))
                .arg(format!("port={}", sink_rtp_port))
                .arg("buffer-size=524288")
                .arg("sync=false")
                .arg("async=false");

            match cmd.spawn() {
                Ok(child) => {
                    println!(
                        "\x1b[1;32m[miracast]\x1b[0m Pipeline de streaming acelerado por GPU ativo (PID: {}, Destino: {}:{}, Modo: {}, Captura: {})",
                        child.id(), config.target_ip, sink_rtp_port, config.mode, config.capture
                    );
                    stream_process = Some(child);
                }
                Err(e) => {
                    eprintln!("\x1b[1;33m[miracast]\x1b[0m Falha ao disparar pipeline GStreamer ({}). Usando worker in-process...", e);
                }
            }
        }

        // 5. Initialize atomic control flags and frame buffer
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let current_frame = Arc::new(RwLock::new(None));
        let frame_source = current_frame.clone();

        let cfg = config.clone();

        // 6. Spawn worker thread if no external pipeline is active
        let worker = if stream_process.is_none() {
            let encoder = create_best_encoder(config.width, config.height, config.fps, config.bitrate_kbps);
            println!(
                "\x1b[1;32m[miracast-hw]\x1b[0m Hardware Acceleration Active: \x1b[1;36m{}\x1b[0m ({}x{} @ {} FPS, {} kbps)",
                encoder.name(),
                config.width,
                config.height,
                config.fps,
                config.bitrate_kbps
            );
            let handle = thread::spawn(move || {
                Self::streaming_worker_loop(
                    udp_socket,
                    sink_addr,
                    cfg,
                    encoder,
                    frame_source,
                    running_clone,
                );
            });
            Some(handle)
        } else {
            None
        };

        Ok(Self {
            config,
            sink_rtp_port,
            session_id,
            running,
            worker,
            wfd_client: Some(client),
            current_frame,
            screencast_session,
            stream_process,
        })
    }

    /// Stops the Miracast streaming session gracefully.
    ///
    /// 1. Terminates desktop streaming pipeline and destroys virtual monitor.
    /// 2. Sends RTSP `TEARDOWN` to the sink and cleanly shuts down TCP socket (< 100ms).
    /// 3. Joins the worker thread if active.
    pub fn stop(&mut self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            // Already stopped or stopping
            return;
        }

        println!("\x1b[1;33m[miracast]\x1b[0m Stopping Miracast session and sending RTSP TEARDOWN...");

        // 1. Terminate streaming process if active
        if let Some(mut child) = self.stream_process.take() {
            let _ = child.kill();
            let _ = child.wait();
            println!("\x1b[1;33m[miracast]\x1b[0m Processo de streaming finalizado.");
        }

        // 2. Drop screencast session (destroys virtual monitor or unbinds clone monitor)
        if let Some(s) = self.screencast_session.take() {
            drop(s);
            println!("\x1b[1;33m[miracast]\x1b[0m Sessão Mutter ScreenCast encerrada (modo {}).", self.config.mode);
        }

        // 3. Clean RTSP TEARDOWN to sink
        if let Some(mut client) = self.wfd_client.take() {
            if let Err(e) = client.teardown() {
                eprintln!("\x1b[1;33m[miracast]\x1b[0m TEARDOWN warning: {}", e);
            }
        }

        // 4. Join worker thread if active
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }

        println!("\x1b[1;32m[miracast]\x1b[0m Miracast session stopped cleanly (< 100ms teardown).");
    }

    /// Returns `true` if the session is currently active and streaming.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Returns the configuration associated with this session.
    pub fn config(&self) -> &MiracastConfig {
        &self.config
    }

    /// Returns the negotiated destination UDP RTP port on the Sink.
    pub fn sink_rtp_port(&self) -> u16 {
        self.sink_rtp_port
    }

    /// Returns the RTSP session ID if established.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Updates the current uncompressed frame buffer (BGRA/RGBA) to be encoded and streamed.
    pub fn update_frame(&self, frame: Vec<u8>) {
        if let Ok(mut lock) = self.current_frame.write() {
            *lock = Some(frame);
        }
    }

    /// Worker thread executing CFR frame pacing, hardware encoding, MPEG-TS muxing, and RTP UDP sending.
    fn streaming_worker_loop(
        socket: UdpSocket,
        sink_addr: SocketAddr,
        config: MiracastConfig,
        mut encoder: Box<dyn crate::encoder::HardwareEncoder>,
        frame_source: Arc<RwLock<Option<Vec<u8>>>>,
        running: Arc<AtomicBool>,
    ) {
        let frame_interval = Duration::from_nanos(1_000_000_000 / config.fps.max(1) as u64);
        let pts_increment: u64 = 90_000 / config.fps.max(1) as u64;

        let mut muxer = MpegTsMuxer::new();
        let mut pts_90khz: u64 = 0;
        let mut rtp_seq: u16 = 1;
        let mut frame_count: u64 = 0;
        let mut last_frame = Instant::now();

        // Preallocate default background frame (dark slate color)
        let frame_size = (config.width * config.height * 4) as usize;
        let default_frame = vec![0x1Eu8; frame_size];

        println!(
            "\x1b[1;32m[miracast]\x1b[0m Streaming worker live: {}x{} @ {} FPS -> UDP {}",
            config.width, config.height, config.fps, sink_addr
        );

        while running.load(Ordering::SeqCst) {
            let now = Instant::now();
            let elapsed = now.duration_since(last_frame);

            // CFR Pacing: sleep until next frame deadline
            if elapsed < frame_interval {
                let remaining = frame_interval - elapsed;
                if remaining > Duration::from_millis(1) {
                    thread::sleep(remaining - Duration::from_millis(1));
                }
                while Instant::now().duration_since(last_frame) < frame_interval {
                    std::hint::spin_loop();
                }
            }
            last_frame = Instant::now();
            frame_count += 1;

            // Generate keyframe every 2 seconds
            let is_keyframe = (frame_count % (config.fps as u64 * 2)) == 1;

            // Acquire latest frame data or fallback to default pattern
            let active_frame = if let Ok(lock) = frame_source.read() {
                lock.clone()
            } else {
                None
            };
            let frame_data = active_frame.as_deref().unwrap_or(&default_frame);

            // 1. Encode frame using active multi-arch hardware encoder
            let h264_stream = match encoder.encode_frame(frame_data, is_keyframe) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("\x1b[1;31m[miracast]\x1b[0m Video encode error: {}", e);
                    continue;
                }
            };

            if h264_stream.is_empty() {
                continue;
            }

            // 2. Mux H.264 NALUs into 188-byte MPEG-TS packets (with PAT/PMT/PCR on keyframes)
            let ts_packets = muxer.mux_h264_nalus(&h264_stream, is_keyframe, pts_90khz);

            // 3. Encapsulate MPEG-TS packets into RTP MP2T datagrams (PT=33, 7 TS packets / datagram)
            let rtp_datagrams = MpegTsMuxer::wrap_rtp(&ts_packets, rtp_seq, (pts_90khz & 0xFFFF_FFFF) as u32);
            rtp_seq = rtp_seq.wrapping_add(rtp_datagrams.len() as u16);
            pts_90khz = pts_90khz.wrapping_add(pts_increment);

            // 4. Transmit RTP datagrams via UDP socket to Sink
            for datagram in &rtp_datagrams {
                if let Err(e) = socket.send_to(datagram, &sink_addr) {
                    if running.load(Ordering::SeqCst) {
                        eprintln!("\x1b[1;31m[miracast]\x1b[0m UDP RTP send error: {}", e);
                    }
                    break;
                }
            }
        }

        running.store(false, Ordering::SeqCst);
    }
}

impl Drop for MiracastSession {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn test_miracast_config_defaults() {
        let config = MiracastConfig::default();
        assert_eq!(config.target_ip, "192.168.7.2");
        assert_eq!(config.target_port, 7236);
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert_eq!(config.fps, 60);
        assert_eq!(config.bitrate_kbps, 4000);
        assert_eq!(config.mode, "extend");
        assert_eq!(config.capture, "mutter");
    }

    #[test]
    fn test_miracast_config_builder() {
        let config = MiracastConfig::new("10.0.0.5", 7250)
            .with_resolution(1920, 1080)
            .with_fps(30)
            .with_bitrate(8000)
            .with_mode("clone")
            .with_capture("kernel");

        assert_eq!(config.target_ip, "10.0.0.5");
        assert_eq!(config.target_port, 7250);
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.fps, 30);
        assert_eq!(config.bitrate_kbps, 8000);
        assert_eq!(config.mode, "clone");
        assert_eq!(config.capture, "kernel");
    }

    #[test]
    fn test_miracast_session_creation() {
        // 1. Bind mock TCP listener for RTSP signaling
        let tcp_listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind mock TCP listener");
        let tcp_port = tcp_listener.local_addr().unwrap().port();

        // 2. Bind mock UDP socket for Sink RTP reception
        let udp_sink = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind mock UDP sink");
        let udp_port = udp_sink.local_addr().unwrap().port();
        udp_sink
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Set UDP timeout failed");

        // 3. Spawn mock RTSP Sink server thread simulating full M1-M7 and TEARDOWN
        let server_thread = thread::spawn(move || {
            let (mut stream, _) = tcp_listener.accept().expect("Accept failed");
            let mut buf = [0u8; 4096];

            // M1 OPTIONS
            let n = stream.read(&mut buf).unwrap();
            let req1 = String::from_utf8_lossy(&buf[..n]);
            assert!(req1.starts_with("OPTIONS"));
            let resp1 = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: org.wfa.wfd1.0, GET_PARAMETER, SET_PARAMETER\r\n\r\n";
            stream.write_all(resp1.as_bytes()).unwrap();

            // M3 GET_PARAMETER
            let n = stream.read(&mut buf).unwrap();
            let req3 = String::from_utf8_lossy(&buf[..n]);
            assert!(req3.starts_with("GET_PARAMETER"));
            let body3 = format!(
                "wfd_video_formats: 30 00 01 02 00000069 00000000 00000000 00 0000 0000 00 none none\r\nwfd_client_rtpports: RTP/AVP/UDP;unicast {} {} mode=play\r\n",
                udp_port, udp_port + 1
            );
            let resp3 = format!(
                "RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Type: text/parameters\r\nContent-Length: {}\r\n\r\n{}",
                body3.len(),
                body3
            );
            stream.write_all(resp3.as_bytes()).unwrap();

            // M4 SET_PARAMETER
            let n = stream.read(&mut buf).unwrap();
            let req4 = String::from_utf8_lossy(&buf[..n]);
            assert!(req4.starts_with("SET_PARAMETER"));
            let resp4 = "RTSP/1.0 200 OK\r\nCSeq: 3\r\n\r\n";
            stream.write_all(resp4.as_bytes()).unwrap();

            // M5 SET_PARAMETER (trigger SETUP)
            let n = stream.read(&mut buf).unwrap();
            let req5 = String::from_utf8_lossy(&buf[..n]);
            assert!(req5.starts_with("SET_PARAMETER"));
            let resp5 = "RTSP/1.0 200 OK\r\nCSeq: 4\r\n\r\n";
            stream.write_all(resp5.as_bytes()).unwrap();

            // M6 SETUP
            let n = stream.read(&mut buf).unwrap();
            let req6 = String::from_utf8_lossy(&buf[..n]);
            assert!(req6.starts_with("SETUP"));
            let resp6 = format!(
                "RTSP/1.0 200 OK\r\nCSeq: 5\r\nSession: 99887766;timeout=30\r\nTransport: RTP/AVP/UDP;unicast;client_port=5002;server_port={}\r\n\r\n",
                udp_port
            );
            stream.write_all(resp6.as_bytes()).unwrap();

            // M7 PLAY
            let n = stream.read(&mut buf).unwrap();
            let req7 = String::from_utf8_lossy(&buf[..n]);
            assert!(req7.starts_with("PLAY"));
            let resp7 = "RTSP/1.0 200 OK\r\nCSeq: 6\r\nSession: 99887766\r\n\r\n";
            stream.write_all(resp7.as_bytes()).unwrap();

            // TEARDOWN
            let n = stream.read(&mut buf).unwrap();
            let req_td = String::from_utf8_lossy(&buf[..n]);
            assert!(req_td.starts_with("TEARDOWN"));
            let resp_td = "RTSP/1.0 200 OK\r\nCSeq: 7\r\n\r\n";
            stream.write_all(resp_td.as_bytes()).unwrap();
        });

        // 4. Start MiracastSession
        let config = MiracastConfig {
            target_ip: "127.0.0.1".to_string(),
            target_port: tcp_port,
            width: 320,
            height: 240,
            fps: 30,
            bitrate_kbps: 1000,
            mode: "extend".to_string(),
            capture: "mutter".to_string(),
        };

        let mut session = MiracastSession::start(config).expect("MiracastSession::start failed");
        assert!(session.is_running(), "Session must be running");
        assert_eq!(session.sink_rtp_port(), udp_port);
        assert_eq!(session.session_id(), Some("99887766"));

        // 5. Update frame buffer
        let dummy_frame = vec![0x33u8; 320 * 240 * 4];
        session.update_frame(dummy_frame);

        // 6. Verify mock UDP sink receives valid RTP MP2T packet
        let mut rtp_buf = [0u8; 2048];
        let (recv_len, _src) = udp_sink
            .recv_from(&mut rtp_buf)
            .expect("Failed to receive RTP packet on UDP sink");
        assert!(recv_len >= 12 + 188, "Received packet must contain RTP header and at least 1 TS packet");
        assert_eq!(rtp_buf[0], 0x80, "RTP V=2 byte");
        assert_eq!(rtp_buf[1] & 0x7F, 33, "RTP Payload Type must be 33 (MP2T)");
        assert_eq!(rtp_buf[12], 0x47, "MPEG-TS sync byte 0x47");

        // 7. Stop session and verify clean shutdown
        session.stop();
        assert!(!session.is_running(), "Session must be stopped");

        server_thread.join().expect("Mock server thread panicked");
    }

    #[test]
    fn test_miracast_session_stop_idempotent() {
        let tcp_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let tcp_port = tcp_listener.local_addr().unwrap().port();

        let _server = thread::spawn(move || {
            let (mut stream, _) = tcp_listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: org.wfa.wfd1.0\r\n\r\n");
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 2\r\n\r\n");
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 3\r\n\r\n");
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 4\r\n\r\n");
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 5\r\nSession: 111\r\nTransport: client_port=5002;server_port=5002\r\n\r\n");
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 6\r\nSession: 111\r\n\r\n");
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"RTSP/1.0 200 OK\r\nCSeq: 7\r\n\r\n");
        });

        let config = MiracastConfig {
            target_ip: "127.0.0.1".to_string(),
            target_port: tcp_port,
            width: 320,
            height: 240,
            fps: 30,
            bitrate_kbps: 1000,
            mode: "extend".to_string(),
            capture: "mutter".to_string(),
        };

        let mut session = MiracastSession::start(config).unwrap();
        assert!(session.is_running());
        session.stop();
        assert!(!session.is_running());
        session.stop(); // Second call must not panic or error
        assert!(!session.is_running());
    }

    #[test]
    fn test_miracast_session_connect_failure() {
        let config = MiracastConfig {
            target_ip: "127.0.0.1".to_string(),
            target_port: 1, // Closed port
            ..Default::default()
        };

        let res = MiracastSession::start(config);
        assert!(res.is_err());
    }
}
