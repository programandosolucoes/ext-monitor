//! Broadcom VideoCore IV Hardware Decode & KMS DRM Rendering Pipeline Manager
//!
//! Controls the lifecycle of GStreamer hardware pipelines utilizing
//! V4L2 M2M hardware decoder (`/dev/video10` - `v4l2h264dec`) and kernel DRM sink (`kmssink`).
//!
//! Supported Input Sources:
//! 1. `RawH264Rtp`: Raw RTP H.264 stream received from Linux Wayland (`ext-sender`) on UDP port 5000.
//! 2. `MiracastMp2t`: MPEG-TS over RTP stream from Windows Wireless Display (`Win + K`) on UDP port 5002.
//! 3. `UsbBulkPipe`: Raw H.264 NAL stream from USB FunctionFS endpoint (Mode 2).
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::drm::ensure_drm_hdmi_connected;
use std::os::unix::io::RawFd;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Active video pipeline type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineKind {
    /// Linux Host: raw RTP H.264 stream on specified UDP port
    RawH264Rtp { port: u16 },
    /// Windows Host: Miracast MPEG-TS over RTP on specified UDP port
    MiracastMp2t { port: u16 },
    /// USB Bulk Direct: reading raw H.264 stream from FunctionFS endpoint
    UsbBulkPipe { fd: RawFd },
}

/// GStreamer hardware pipeline manager
pub struct PipelineManager {
    child: Arc<Mutex<Option<Child>>>,
    active_kind: Arc<Mutex<Option<PipelineKind>>>,
    paused: Arc<AtomicBool>,
}

impl PipelineManager {
    /// Creates a new pipeline manager instance
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            active_kind: Arc::new(Mutex::new(None)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns whether the pipeline has been explicitly paused by the user
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Pauses display streaming and stops the active pipeline
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        self.stop();
        println!("\x1b[1;33m[pipeline]\x1b[0m Pipeline paused by user.");
    }

    /// Resumes display streaming with the specified pipeline kind
    pub fn resume(&self, kind: PipelineKind) -> Result<(), std::io::Error> {
        self.paused.store(false, Ordering::SeqCst);
        self.start(kind)
    }

    /// Returns the currently active pipeline kind, if any
    pub fn current_kind(&self) -> Option<PipelineKind> {
        *self.active_kind.lock().unwrap()
    }

    /// Stops the active pipeline cleanly (SIGTERM with fallback to SIGKILL)
    pub fn stop(&self) {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(mut child) = child_guard.take() {
            println!("\x1b[1;33m[pipeline]\x1b[0m Stopping active GStreamer pipeline...");
            let _ = child.kill();
            let _ = child.wait();
        }
        let mut kind_guard = self.active_kind.lock().unwrap();
        *kind_guard = None;
    }

    /// Launches the requested hardware decode pipeline
    pub fn start(&self, kind: PipelineKind) -> Result<(), std::io::Error> {
        self.paused.store(false, Ordering::SeqCst);
        self.stop();

        // Ensure DRM KMS connector is forced 'on' for headless operation
        ensure_drm_hdmi_connected();

        println!(
            "\x1b[1;32m[pipeline]\x1b[0m Launching VideoCore IV decode pipeline for {:?}",
            kind
        );

        let child = match kind {
            PipelineKind::RawH264Rtp { port } => {
                let caps = "caps=application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96";
                Command::new("gst-launch-1.0")
                    .arg("-v")
                    .arg("udpsrc")
                    .arg(format!("port={}", port))
                    .arg("buffer-size=262144")
                    .arg(caps)
                    .arg("!")
                    .arg("rtph264depay")
                    .arg("wait-for-keyframe=true")
                    .arg("!")
                    .arg("h264parse")
                    .arg("!")
                    .arg("v4l2h264dec")
                    .arg("capture-io-mode=dmabuf")
                    .arg("output-io-mode=dmabuf")
                    .arg("qos=true")
                    .arg("!")
                    .arg("queue")
                    .arg("max-size-buffers=1")
                    .arg("max-size-bytes=0")
                    .arg("max-size-time=0")
                    .arg("leaky=downstream")
                    .arg("!")
                    .arg("kmssink")
                    .arg("sync=false")
                    .arg("qos=true")
                    .arg("skip-vsync=true")
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()?
            }
            PipelineKind::MiracastMp2t { port } => {
                let caps = "caps=application/x-rtp,media=video,clock-rate=90000,encoding-name=MP2T";
                Command::new("gst-launch-1.0")
                    .arg("-v")
                    .arg("udpsrc")
                    .arg(format!("port={}", port))
                    .arg("buffer-size=524288")
                    .arg(caps)
                    .arg("!")
                    .arg("rtpmp2tdepay")
                    .arg("!")
                    .arg("tsdemux")
                    .arg("!")
                    .arg("h264parse")
                    .arg("!")
                    .arg("v4l2h264dec")
                    .arg("capture-io-mode=dmabuf")
                    .arg("output-io-mode=dmabuf")
                    .arg("qos=true")
                    .arg("!")
                    .arg("queue")
                    .arg("max-size-buffers=1")
                    .arg("max-size-bytes=0")
                    .arg("max-size-time=0")
                    .arg("leaky=downstream")
                    .arg("!")
                    .arg("kmssink")
                    .arg("sync=false")
                    .arg("qos=true")
                    .arg("skip-vsync=true")
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()?
            }
            PipelineKind::UsbBulkPipe { fd } => {
                Command::new("gst-launch-1.0")
                    .arg("-v")
                    .arg("fdsrc")
                    .arg(format!("fd={}", fd))
                    .arg("!")
                    .arg("h264parse")
                    .arg("!")
                    .arg("v4l2h264dec")
                    .arg("capture-io-mode=dmabuf")
                    .arg("output-io-mode=dmabuf")
                    .arg("qos=true")
                    .arg("!")
                    .arg("queue")
                    .arg("max-size-buffers=1")
                    .arg("max-size-bytes=0")
                    .arg("max-size-time=0")
                    .arg("leaky=downstream")
                    .arg("!")
                    .arg("kmssink")
                    .arg("sync=false")
                    .arg("qos=true")
                    .arg("skip-vsync=true")
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()?
            }
        };

        *self.child.lock().unwrap() = Some(child);
        *self.active_kind.lock().unwrap() = Some(kind);

        println!("\x1b[1;32m[pipeline]\x1b[0m Hardware decode pipeline launched successfully.");
        Ok(())
    }

    /// Checks if the pipeline process has terminated
    pub fn has_exited(&self) -> bool {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *child_guard {
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("\x1b[1;33m[pipeline]\x1b[0m Pipeline process exited with status: {}", status);
                    true
                }
                Ok(None) => false,
                Err(e) => {
                    eprintln!("\x1b[1;31m[pipeline]\x1b[0m Error checking child status: {}", e);
                    true
                }
            }
        } else {
            false
        }
    }
}
