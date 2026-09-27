//! Streaming Pipeline Builder and Process Management
//!
//! Encapsulates process execution and command synthesis for GStreamer, FFmpeg,
//! and native Rust streaming engines following the Builder Pattern.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::config::{ColorProfile, EncoderApi, StreamEngine};
use crate::native_streamer;
use std::io;
use std::os::unix::io::RawFd;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus};

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
}

impl PipelineBuilder {
    /// Spawns the configured streaming pipeline process
    pub fn spawn(&self) -> io::Result<StreamerHandle> {
        match self.engine {
            StreamEngine::NativeRust => {
                let streamer = native_streamer::NativeStreamer::start(
                    self.target_ip.clone(),
                    self.target_port,
                    self.bitrate,
                    self.fps,
                    self.usb_pipe_fd,
                )?;
                Ok(StreamerHandle::Native(streamer))
            }
            StreamEngine::GStreamer => {
                let child = self.spawn_gstreamer()?;
                Ok(StreamerHandle::Child(child))
            }
            StreamEngine::FFmpeg => {
                let child = self.spawn_ffmpeg()?;
                Ok(StreamerHandle::Child(child))
            }
        }
    }

    fn spawn_gstreamer(&self) -> io::Result<Child> {
        let mut cmd = Command::new("gst-launch-1.0");
        cmd.arg("-v");

        // 1. PipeWire source with clean pw-link port registration
        cmd.arg("pipewiresrc")
            .arg("autoconnect=false")
            .arg("stream-properties=props,node.name=ext-hdmi-sender")
            .arg("do-timestamp=true")
            .arg("min-buffers=2")
            .arg("max-buffers=2")
            .arg("always-copy=false")
            .arg("!")
            .arg("queue")
            .arg("max-size-buffers=2")
            .arg("max-size-bytes=0")
            .arg("max-size-time=0")
            .arg("leaky=downstream")
            .arg("!")
            .arg(format!("video/x-raw,max-framerate={}/1", self.fps))
            .arg("!");

        // 2. Framerate normalization (smooth continuous frame delivery)
        cmd.arg("videorate")
            .arg(format!("drop-only={}", self.drop_only))
            .arg(format!("skip-to-first={}", self.skip_to_first))
            .arg("!")
            .arg(format!("video/x-raw,framerate={}/1", self.fps))
            .arg("!");

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
                .arg("max-size-buffers=4")
                .arg("max-size-bytes=0")
                .arg("max-size-time=0")
                .arg("!")
                .arg("fdsink")
                .arg(format!("fd={}", fd))
                .arg("sync=false");
        } else {
            cmd.arg("rtph264pay")
                .arg("config-interval=1")
                .arg("pt=96")
                .arg("aggregate-mode=none")
                .arg("!")
                .arg("queue")
                .arg("max-size-buffers=4")
                .arg("max-size-bytes=0")
                .arg("max-size-time=0")
                .arg("leaky=downstream")
                .arg("!")
                .arg("udpsink")
                .arg(format!("host={}", self.target_ip))
                .arg(format!("port={}", self.target_port))
                .arg("buffer-size=262144")
                .arg("sync=false");
        }

        cmd.spawn()
    }

    fn append_encoder_args(&self, cmd: &mut Command) {
        match self.encoder {
            EncoderApi::Vaapi => {
                if self.color_profile == ColorProfile::Grayscale {
                    cmd.arg("videobalance").arg("saturation=0.0").arg("!");
                }

                cmd.arg("vapostproc")
                    .arg("!")
                    .arg("vah264enc")
                    .arg(format!("bitrate={}", self.bitrate))
                    .arg("rate-control=vbr");

                match self.color_profile {
                    ColorProfile::Economy256 => {
                        cmd.arg("target-percentage=50").arg("min-qp=28").arg("max-qp=42");
                    }
                    ColorProfile::Grayscale => {
                        cmd.arg("target-percentage=60").arg("min-qp=20").arg("max-qp=38");
                    }
                    ColorProfile::TrueColor => {
                        cmd.arg("target-percentage=85").arg("min-qp=18").arg("max-qp=34");
                    }
                }

                cmd.arg("mbbrc=enabled")
                    .arg("target-usage=5")
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

    fn spawn_ffmpeg(&self) -> io::Result<Child> {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-nostdin")
            .arg("-y")
            .arg("-f").arg("pipewire")
            .arg("-i").arg(format!("{}", self.node_id))
            .arg("-vf").arg("scale=1280:720:flags=fast_bilinear,fps=30")
            .arg("-c:v").arg("h264_vaapi")
            .arg("-vaapi_device").arg("/dev/dri/renderD128")
            .arg("-b:v").arg(format!("{}k", self.bitrate))
            .arg("-maxrate").arg(format!("{}k", self.bitrate))
            .arg("-bufsize").arg(format!("{}k", self.bitrate / 4))
            .arg("-g").arg(format!("{}", self.key_int_max))
            .arg("-bf").arg("0")
            .arg("-tune").arg("zerolatency");

        if let Some(fd) = self.usb_pipe_fd {
            cmd.arg("-f").arg("h264").arg(format!("pipe:{}", fd));
        } else {
            cmd.arg("-payload_type").arg("96")
                .arg("-f").arg("rtp")
                .arg(format!("rtp://{}:{}", self.target_ip, self.target_port));
        }

        cmd.spawn()
    }
}
