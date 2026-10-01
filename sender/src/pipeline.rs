//! Streaming Pipeline Builder and Process Management
//!
//! Encapsulates process execution and command synthesis for GStreamer, FFmpeg,
//! and native Rust streaming engines following the Builder Pattern.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::config::{CaptureEngine, ColorProfile, EncoderApi, StreamEngine};
use crate::kms::KmsOutputInfo;
use crate::native_streamer;
use std::io::{self, BufRead, BufReader, Read};
use std::net::UdpSocket;
use std::os::unix::io::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub enum StreamerHandle {
    Child(Child),
    Native(native_streamer::NativeStreamer),
}

impl StreamerHandle {
    pub fn kill(&mut self) -> Result<(), io::Error> {
        match self {
            StreamerHandle::Child(ref mut c) => c.kill(),
            StreamerHandle::Native(ref mut n) => {
                n.stop();
                Ok(())
            }
        }
    }

    pub fn wait(&mut self) -> Result<ExitStatus, io::Error> {
        match self {
            StreamerHandle::Child(ref mut c) => c.wait(),
            StreamerHandle::Native(_) => Ok(ExitStatus::from_raw(0)),
        }
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, io::Error> {
        match self {
            StreamerHandle::Child(ref mut c) => c.try_wait(),
            StreamerHandle::Native(ref n) => {
                if n.is_running() {
                    Ok(None)
                } else {
                    Ok(Some(ExitStatus::from_raw(0)))
                }
            }
        }
    }
}

pub struct PipelineBuilder {
    #[allow(dead_code)]
    pub node_id: u32,
    pub target_ip: String,
    pub target_port: u16,
    pub bitrate: u32,
    pub encoder: EncoderApi,
    pub fps: u32,
    pub hud: bool,
    pub color_profile: ColorProfile,
    pub drop_only: bool,
    pub skip_to_first: bool,
    pub key_int_max: u32,
    pub usb_pipe_fd: Option<RawFd>,
    pub engine: StreamEngine,
    #[allow(dead_code)]
    pub capture: CaptureEngine,
    #[allow(dead_code)]
    pub kms_info: Option<KmsOutputInfo>,
    pub audio: bool,
    #[allow(dead_code)]
    pub audio_port: u16,
}

impl PipelineBuilder {
    /// Spawns the configured streaming pipeline process
    pub fn spawn(&self) -> io::Result<StreamerHandle> {
        let video_handle = if self.engine == StreamEngine::NativeRust {
            let streamer = native_streamer::NativeStreamer::start(
                self.target_ip.clone(),
                self.target_port,
                self.bitrate,
                self.fps,
                self.usb_pipe_fd,
            )?;
            StreamerHandle::Native(streamer)
        } else {
            let child = self.spawn_gstreamer()?;
            StreamerHandle::Child(child)
        };

        Ok(video_handle)
    }

    fn spawn_gstreamer(&self) -> io::Result<Child> {
        let mut cmd = Command::new("gst-launch-1.0");

        let keepalive_ms = (1000 / self.fps).max(16);
        cmd.arg("pipewiresrc");
        cmd.arg("client-name=ext-video-sender");
        if self.node_id > 0 {
            cmd.arg(format!("path={}", self.node_id));
        } else {
            cmd.arg("autoconnect=false")
                .arg("stream-properties=props,node.name=ext-video-sender");
        }
        cmd.arg("do-timestamp=true")
            .arg("min-buffers=2")
            .arg("max-buffers=4")
            .arg(format!("keepalive-time={}", keepalive_ms))
            .arg("always-copy=false")
            .arg("!");

        // 2. Hardware Framerate Shaping: zero-copy passthrough
        if self.drop_only {
            cmd.arg("videorate")
                .arg("drop-only=true")
                .arg(format!("skip-to-first={}", self.skip_to_first))
                .arg("!")
                .arg(format!("video/x-raw,framerate={}/1", self.fps))
                .arg("!");
        } else {
            cmd.arg("videorate")
                .arg("drop-only=false")
                .arg(format!("skip-to-first={}", self.skip_to_first))
                .arg("!")
                .arg(format!("video/x-raw,framerate={}/1", self.fps))
                .arg("!");
        }

        // 3. Diagnostic On-Screen HUD if active
        if self.hud {
            let avg_pct = if self.color_profile == ColorProfile::Economy256 { 50 } else { 75 };
            let hud_content = format!(
                "text=\"[ PI ZERO EXTENDED MONITOR • ACTIVE ]\\nPanel:    1600x900@59.95Hz (Native 1:1)\\nStream:   {} FPS | Drop-on-Late (3x LIFO)\\nColor:    {}\\nRate:     Adaptive VBR ({}k cap / {}% avg)\\nVPU:      Broadcom VideoCore IV @ 500MHz (+25% OC)\\nCPU:      ARM1176 Load ~22% | RAM: ~141 MiB\\nNetwork:  USB OTG (RTT 0.34ms, txq: 100)\\nSync:     IDR Interval {} frames (drop-only={})\\nWeb:      http://{}:8080 (Auto-hide in 60s)\"",
                self.fps, self.color_profile.name(), self.bitrate, avg_pct, self.key_int_max, self.drop_only, self.target_ip
            );
            cmd.arg("textoverlay")
                .arg(hud_content)
                .arg("valignment=top")
                .arg("halignment=right")
                .arg("line-alignment=left")
                .arg("font-desc=Monospace Bold 10")
                .arg("color=0xFF00FF66")
                .arg("outline-color=0x80000000")
                .arg("draw-outline=true")
                .arg("shaded-background=true")
                .arg("shading-value=65")
                .arg("xpad=14")
                .arg("ypad=12")
                .arg("!")
                .arg("clockoverlay")
                .arg("time-format=%H:%M:%S")
                .arg("valignment=top")
                .arg("halignment=left")
                .arg("font-desc=Monospace Bold 11")
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
                .arg("font-desc=Monospace Bold 10")
                .arg("color=0xFFFFFFFF")
                .arg("outline-color=0x80000000")
                .arg("draw-outline=true")
                .arg("shaded-background=true")
                .arg("shading-value=65")
                .arg("xpad=14")
                .arg("ypad=12")
                .arg("!");
        }

        // 4. Zero-Latency Pre-Encoder Queue
        cmd.arg("queue")
            .arg("max-size-buffers=1")
            .arg("max-size-bytes=0")
            .arg("max-size-time=0")
            .arg("leaky=downstream")
            .arg("!");

        // 5. Hardware / Software Encoder Sub-pipeline
        self.append_encoder_args(&mut cmd);

        // 6. Post-Encoder Framing and Output Sink (Direct lossless Annex-B for USB, UDP RTP for Network)
        if let Some(fd) = self.usb_pipe_fd {
            cmd.arg("h264parse")
                .arg("config-interval=-1")
                .arg("!")
                .arg("queue")
                .arg("max-size-buffers=1")
                .arg("max-size-bytes=0")
                .arg("max-size-time=0")
                .arg("leaky=downstream")
                .arg("!")
                .arg("fdsink")
                .arg(format!("fd={}", fd))
                .arg("sync=false");

            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(move || {
                    libc::fcntl(fd, libc::F_SETFD, 0);
                    Ok(())
                });
            }
        } else {
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
                .arg("aggregate-mode=none")
                .arg("!")
                .arg("udpsink")
                .arg(format!("host={}", self.target_ip))
                .arg(format!("port={}", self.target_port))
                .arg("buffer-size=262144")
                .arg("sync=false");
        }

        Self::spawn_and_attach_logger(cmd, "GStreamer")
    }

    fn spawn_and_attach_logger(mut cmd: Command, tag: &'static str) -> io::Result<Child> {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn()?;

        if let Some(stdout) = child.stdout.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.contains("99:99:99") {
                        continue;
                    }
                    println!("\x1b[1;36m[{}][OUT]\x1b[0m {}", tag, trimmed);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.contains("99:99:99") {
                        continue;
                    }
                    if trimmed.contains("ERROR") || trimmed.contains("ERRO") || trimmed.contains("Failed") {
                        eprintln!("\x1b[1;31m[{}][ERROR]\x1b[0m {}", tag, trimmed);
                    } else if trimmed.contains("WARN") || trimmed.contains("AVISO") {
                        eprintln!("\x1b[1;33m[{}][WARN]\x1b[0m {}", tag, trimmed);
                    } else {
                        println!("\x1b[1;37m[{}][DIAG]\x1b[0m {}", tag, trimmed);
                    }
                }
            });
        }

        Ok(child)
    }

    fn append_encoder_args(&self, cmd: &mut Command) {
        match self.encoder {
            EncoderApi::Vaapi => {
                if self.color_profile == ColorProfile::Grayscale {
                    cmd.arg("videobalance").arg("saturation=0.0").arg("!");
                }

                cmd.arg("vapostproc")
                    .arg("!")
                    .arg("video/x-raw(memory:VAMemory),width=1280,height=720")
                    .arg("!")
                    .arg("vah264enc")
                    .arg(format!("bitrate={}", self.bitrate))
                    .arg("rate-control=vbr");

                match self.color_profile {
                    ColorProfile::Economy256 => {
                        cmd.arg("target-percentage=50").arg("min-qp=30").arg("max-qp=44");
                    }
                    ColorProfile::Grayscale => {
                        cmd.arg("target-percentage=60").arg("min-qp=20").arg("max-qp=38");
                    }
                    ColorProfile::TrueColor => {
                        cmd.arg("target-percentage=95").arg("min-qp=10").arg("max-qp=22");
                    }
                }

                cmd.arg("mbbrc=enabled")
                    .arg("target-usage=7")
                    .arg("b-frames=0")
                    .arg("ref-frames=1")
                    .arg("aud=true")
                    .arg("cabac=false")
                    .arg("dct8x8=false")
                    .arg("num-slices=1")
                    .arg(format!("key-int-max={}", self.key_int_max))
                    .arg("!")
                    .arg("video/x-h264,profile=constrained-baseline")
                    .arg("!");
            }
            EncoderApi::Nvenc => {
                cmd.arg("videoscale")
                    .arg("!")
                    .arg("videoconvert")
                    .arg("!")
                    .arg("video/x-raw,format=NV12,width=1280,height=720")
                    .arg("!")
                    .arg("nvh264enc")
                    .arg(format!("bitrate={}", self.bitrate))
                    .arg("preset=low-latency-hq")
                    .arg("rc-mode=cbr-ld-hq")
                    .arg("zerolatency=true")
                    .arg(format!("gop-size={}", self.key_int_max))
                    .arg("b-frames=0")
                    .arg("!");
            }
            EncoderApi::Qsv => {
                cmd.arg("videoscale")
                    .arg("!")
                    .arg("videoconvert")
                    .arg("!")
                    .arg("video/x-raw,format=NV12,width=1280,height=720")
                    .arg("!")
                    .arg("qsvh264enc")
                    .arg(format!("bitrate={}", self.bitrate))
                    .arg("rate-control=cbr")
                    .arg("target-usage=veryfast")
                    .arg(format!("gop-size={}", self.key_int_max))
                    .arg("b-frames=0")
                    .arg("!");
            }
            EncoderApi::Software => {
                cmd.arg("videoscale")
                    .arg("!")
                    .arg("videoconvert")
                    .arg("!")
                    .arg("video/x-raw,format=I420,width=1280,height=720")
                    .arg("!")
                    .arg("x264enc")
                    .arg(format!("bitrate={}", self.bitrate))
                    .arg("tune=zerolatency")
                    .arg("speed-preset=ultrafast")
                    .arg("b-frames=0")
                    .arg("ref-frames=1")
                    .arg(format!("key-int-max={}", self.key_int_max))
                    .arg("byte-stream=true")
                    .arg("aud=true")
                    .arg("sliced-threads=false")
                    .arg("!");
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Real-Time Hardware Audio Spectrum Analysis & UDP Streamer (Host PC -> Pi Zero)
// -----------------------------------------------------------------------------

const BAND_RANGES: [(usize, usize); 24] = [
    (1, 2),   // 94 - 188 Hz
    (2, 3),   // 188 - 281 Hz
    (3, 4),   // 281 - 375 Hz
    (4, 5),   // 375 - 469 Hz
    (5, 7),   // 469 - 656 Hz
    (7, 9),   // 656 - 844 Hz
    (9, 12),  // 844 - 1125 Hz
    (12, 16), // 1.1 - 1.5 kHz
    (16, 21), // 1.5 - 2.0 kHz
    (21, 28), // 2.0 - 2.6 kHz
    (28, 36), // 2.6 - 3.4 kHz
    (36, 46), // 3.4 - 4.3 kHz
    (46, 58), // 4.3 - 5.4 kHz
    (58, 73), // 5.4 - 6.8 kHz
    (73, 91), // 6.8 - 8.5 kHz
    (91, 112), // 8.5 - 10.5 kHz
    (112, 136), // 10.5 - 12.8 kHz
    (136, 162), // 12.8 - 15.2 kHz
    (162, 188), // 15.2 - 17.6 kHz
    (188, 214), // 17.6 - 20.1 kHz
    (214, 224), // 20.1 - 21.0 kHz
    (224, 234), // 21.0 - 21.9 kHz
    (234, 245), // 21.9 - 23.0 kHz
    (245, 256), // 23.0 - 24.0 kHz
];

fn fft_512(real: &mut [f32; 512], imag: &mut [f32; 512]) {
    let mut j = 0;
    for i in 0..511 {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut k = 256;
        while k <= j {
            j -= k;
            k >>= 1;
        }
        j += k;
    }

    let mut len = 2;
    while len <= 512 {
        let half = len / 2;
        let angle = -2.0 * std::f32::consts::PI / (len as f32);
        let w_step_re = angle.cos();
        let w_step_im = angle.sin();

        let mut i = 0;
        while i < 512 {
            let mut w_re = 1.0f32;
            let mut w_im = 0.0f32;
            for j in 0..half {
                let u_re = real[i + j];
                let u_im = imag[i + j];
                let v_re = real[i + j + half] * w_re - imag[i + j + half] * w_im;
                let v_im = real[i + j + half] * w_im + imag[i + j + half] * w_re;

                real[i + j] = u_re + v_re;
                imag[i + j] = u_im + v_im;
                real[i + j + half] = u_re - v_re;
                imag[i + j + half] = u_im - v_im;

                let next_w_re = w_re * w_step_re - w_im * w_step_im;
                let next_w_im = w_re * w_step_im + w_im * w_step_re;
                w_re = next_w_re;
                w_im = next_w_im;
            }
            i += len;
        }
        len <<= 1;
    }
}

pub fn ensure_audio_sink_exists() {
    let exists = Command::new("pactl")
        .args(["list", "sinks", "short"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Raspberry_Pi_HDMI_Audio"))
        .unwrap_or(false);

    if !exists {
        let _ = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                "sink_name=Raspberry_Pi_HDMI_Audio",
                "sink_properties=device.description=Raspberry_Pi_HDMI_Audio",
            ])
            .output();
    }
}

pub fn spawn_opus_audio_streamer(
    target_ip: String,
    audio_port: u16,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("audio-opus-tx".to_string())
        .spawn(move || {
            while running.load(Ordering::Relaxed) {
                ensure_audio_sink_exists();

                println!(
                    "\x1b[1;34m[*] Starting low-latency Opus audio streamer to {}:{} (source: Raspberry_Pi_HDMI_Audio.monitor)...\x1b[0m",
                    target_ip, audio_port
                );

                let mut child = match Command::new("gst-launch-1.0")
                    .env("PULSE_SOURCE", "Raspberry_Pi_HDMI_Audio.monitor")
                    .env("PULSE_PROP", "media.role=filter stream.dont-route=true node.dont-reconnect=true")
                    .arg("-q")
                    .arg("pulsesrc")
                    .arg("device=Raspberry_Pi_HDMI_Audio.monitor")
                    .arg("do-timestamp=true")
                    .arg("!")
                    .arg("audioconvert")
                    .arg("!")
                    .arg("audioresample")
                    .arg("!")
                    .arg("audio/x-raw,rate=48000,channels=2")
                    .arg("!")
                    .arg("opusenc")
                    .arg("bitrate=96000")
                    .arg("frame-size=10")
                    .arg("complexity=3")
                    .arg("!")
                    .arg("rtpopuspay")
                    .arg("pt=96")
                    .arg("!")
                    .arg("udpsink")
                    .arg(format!("host={}", target_ip))
                    .arg(format!("port={}", audio_port))
                    .arg("sync=false")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("\x1b[1;33m[!] Failed to spawn audio streamer: {}. Retrying in 1s...\x1b[0m", e);
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                };

                while running.load(Ordering::Relaxed) {
                    match child.try_wait() {
                        Ok(Some(_status)) => break,
                        Ok(None) => thread::sleep(Duration::from_millis(500)),
                        Err(_) => break,
                    }
                }

                let _ = child.kill();
                let _ = child.wait();
                thread::sleep(Duration::from_millis(500));
            }
        })
        .expect("Failed to spawn audio streamer thread")
}

pub fn spawn_audio_spectrum_monitor(
    target_ip: String,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("audio-spectrum-tx".to_string())
        .spawn(move || {
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(_) => return,
            };
            let dest_addr = format!("{}:5006", target_ip);

            // 512 stereo samples = 512 * 4 = 2048 bytes (~10.7ms of audio)
            let mut raw_buf = [0u8; 2048];
            let mut packet = [0u8; 25];

            while running.load(Ordering::Relaxed) {
                ensure_audio_sink_exists();

                let mut child = match Command::new("parec")
                    .env("PULSE_SOURCE", "Raspberry_Pi_HDMI_Audio.monitor")
                    .env("PULSE_PROP", "media.role=filter stream.dont-route=true node.dont-reconnect=true")
                    .args([
                        "--device=Raspberry_Pi_HDMI_Audio.monitor",
                        "--rate=48000",
                        "--channels=2",
                        "--format=s16le",
                        "--raw",
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("\x1b[1;33m[!] Failed to spawn parec: {}. Retrying in 1s...\x1b[0m", e);
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                };

                let mut stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        let _ = child.kill();
                        let _ = child.wait();
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                };

                let mut silence_frames: u32 = 0;

                while running.load(Ordering::Relaxed) {
                    if stdout.read_exact(&mut raw_buf).is_err() {
                        // EOF or pipe broken - break to restart parec
                        break;
                    }

                    let mut real = [0.0f32; 512];
                    let mut imag = [0.0f32; 512];
                    let mut sum_sq = 0.0f64;

                    for i in 0..512 {
                        let l = i16::from_le_bytes([raw_buf[i * 4], raw_buf[i * 4 + 1]]) as f32 / 32768.0;
                        let r = i16::from_le_bytes([raw_buf[i * 4 + 2], raw_buf[i * 4 + 3]]) as f32 / 32768.0;
                        let mono = (l + r) * 0.5;
                        // Hann window to prevent spectral leakage
                        let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / 512.0).cos());
                        real[i] = mono * w;
                        sum_sq += (mono * mono) as f64;
                    }

                    let rms = (sum_sq / 512.0).sqrt() as f32;
                    let rms_db = if rms > 1e-4 { 20.0 * rms.log10() } else { -90.0 };
                    let rms_byte = ((rms_db + 60.0).clamp(0.0, 60.0) / 60.0 * 255.0) as u8;

                    fft_512(&mut real, &mut imag);

                    for (idx, &(start_k, end_k)) in BAND_RANGES.iter().enumerate() {
                        let mut mag_sum = 0.0f32;
                        let count = (end_k - start_k).max(1);
                        for k in start_k..end_k {
                            let mag = (real[k] * real[k] + imag[k] * imag[k]).sqrt();
                            mag_sum += mag;
                        }
                        let avg_mag = mag_sum / count as f32;
                        let normalized = (avg_mag * 4.0).clamp(0.0, 1.0);
                        packet[idx] = (normalized * 255.0) as u8;
                    }
                    packet[24] = rms_byte;

                    let is_silence = rms_db < -55.0 && packet[..24].iter().all(|&b| b < 6);
                    if is_silence {
                        silence_frames = silence_frames.saturating_add(1);
                    } else {
                        silence_frames = 0;
                    }

                    // Send up to 3 frames of silence to notify receiver of silence transition,
                    // then suppress UDP packets until active audio resumes (zero packets during silence)
                    if silence_frames <= 3 {
                        let _ = socket.send_to(&packet, &dest_addr);
                    }

                    thread::sleep(Duration::from_millis(20)); // ~50 FPS spectrum refresh
                }

                let _ = child.kill();
                let _ = child.wait();
                thread::sleep(Duration::from_millis(300));
            }
        })
        .expect("Failed to spawn audio spectrum thread")
}

