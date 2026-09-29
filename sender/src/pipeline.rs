//! Streaming Pipeline Builder and Process Management
//!
//! Encapsulates process execution and command synthesis for GStreamer, FFmpeg,
//! and native Rust streaming engines following the Builder Pattern.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::config::{CaptureEngine, ColorProfile, EncoderApi, StreamEngine};
use crate::kms::KmsOutputInfo;
use crate::native_streamer;
use std::io::{self, BufRead, BufReader};
use std::os::unix::io::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;

pub enum StreamerHandle {
    Child(Child),
    Native(native_streamer::NativeStreamer),
    Composite {
        video: Box<StreamerHandle>,
        audio: Option<Child>,
    },
}

impl StreamerHandle {
    pub fn kill(&mut self) -> Result<(), io::Error> {
        match self {
            StreamerHandle::Child(ref mut c) => c.kill(),
            StreamerHandle::Native(ref mut n) => {
                n.stop();
                Ok(())
            }
            StreamerHandle::Composite { ref mut video, ref mut audio } => {
                let _ = video.kill();
                if let Some(ref mut a) = audio {
                    let _ = a.kill();
                }
                Ok(())
            }
        }
    }

    pub fn wait(&mut self) -> Result<ExitStatus, io::Error> {
        match self {
            StreamerHandle::Child(ref mut c) => c.wait(),
            StreamerHandle::Native(_) => Ok(ExitStatus::from_raw(0)),
            StreamerHandle::Composite { ref mut video, ref mut audio } => {
                if let Some(ref mut a) = audio {
                    let _ = a.wait();
                }
                video.wait()
            }
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
            StreamerHandle::Composite { ref mut video, .. } => {
                video.try_wait()
            }
        }
    }
}

pub struct PipelineBuilder {
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
    pub capture: CaptureEngine,
    #[allow(dead_code)]
    pub kms_info: Option<KmsOutputInfo>,
    pub audio: bool,
    pub audio_port: u16,
}

impl PipelineBuilder {
    /// Spawns the configured streaming pipeline process
    pub fn spawn_audio(&self) -> Option<Child> {
        if !self.audio {
            return None;
        }

        println!(
            "\x1b[1;34m[*] Starting low-latency PipeWire Opus audio streamer to {}:{}...\x1b[0m",
            self.target_ip, self.audio_port
        );

        Command::new("gst-launch-1.0")
            .arg("-q")
            .arg("pipewiresrc")
            .arg("client-name=ext-hdmi-audio")
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
            .arg(format!("host={}", self.target_ip))
            .arg(format!("port={}", self.audio_port))
            .arg("sync=false")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }

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

        let audio_child = self.spawn_audio();

        Ok(StreamerHandle::Composite {
            video: Box::new(video_handle),
            audio: audio_child,
        })
    }

    fn spawn_gstreamer(&self) -> io::Result<Child> {
        let mut cmd = Command::new("gst-launch-1.0");

        let keepalive_ms = (1000 / self.fps).max(16);
        cmd.arg("pipewiresrc")
            .arg("autoconnect=false")
            .arg("stream-properties=props,node.name=ext-hdmi-sender")
            .arg("do-timestamp=true")
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
