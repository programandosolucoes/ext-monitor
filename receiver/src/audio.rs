//! Low-Latency HDMI Digital Audio Subsystem for ext-receiver
//!
//! Ingests RTP Opus streams on UDP port 5004 (or configurable)
//! and renders directly to the BCM2835 ALSA HDMI sound card (/dev/snd/pcmC0D0p)
//! or system default audio sink.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use std::process::{Child, Command, Stdio};

pub const DEFAULT_AUDIO_PORT: u16 = 5004;

#[derive(Debug, Clone)]
pub struct AudioStatus {
    pub enabled: bool,
    pub active: bool,
    pub volume: u32,
    pub muted: bool,
    pub port: u16,
}

pub struct AudioReceiver {
    child: Option<Child>,
    port: u16,
    volume: u32,
    muted: bool,
    enabled: bool,
}

impl AudioReceiver {
    pub fn new(port: u16) -> Self {
        Self {
            child: None,
            port,
            volume: 100,
            muted: false,
            enabled: true,
        }
    }

    pub fn start(&mut self) -> Result<(), std::io::Error> {
        if !self.enabled {
            return Ok(());
        }
        self.stop();

        println!(
            "\x1b[1;34m[audio]\x1b[0m Starting low-latency HDMI audio receiver on UDP port {} (vol: {}%, muted: {})...",
            self.port, self.volume, self.muted
        );

        let vol_float = if self.muted {
            0.0f32
        } else {
            (self.volume as f32) / 100.0f32
        };

        // 1. Try alsasink first (optimized for Pi Zero vc4-hdmi or default ALSA)
        let child = match Command::new("gst-launch-1.0")
            .arg("-q")
            .arg("udpsrc")
            .arg(format!("port={}", self.port))
            .arg("caps=application/x-rtp,media=audio,clock-rate=48000,encoding-name=OPUS,payload=96")
            .arg("!")
            .arg("rtpopusdepay")
            .arg("!")
            .arg("opusdec")
            .arg("!")
            .arg("audioconvert")
            .arg("!")
            .arg("audioresample")
            .arg("!")
            .arg("volume")
            .arg(format!("volume={:.2}", vol_float))
            .arg("!")
            .arg("alsasink")
            .arg("sync=false")
            .arg("buffer-time=20000")
            .arg("latency-time=10000")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => Some(c),
            Err(_) => {
                // 2. Fallback to autoaudiosink (for QEMU / generic audio server)
                Command::new("gst-launch-1.0")
                    .arg("-q")
                    .arg("udpsrc")
                    .arg(format!("port={}", self.port))
                    .arg("caps=application/x-rtp,media=audio,clock-rate=48000,encoding-name=OPUS,payload=96")
                    .arg("!")
                    .arg("rtpopusdepay")
                    .arg("!")
                    .arg("opusdec")
                    .arg("!")
                    .arg("audioconvert")
                    .arg("!")
                    .arg("audioresample")
                    .arg("!")
                    .arg("volume")
                    .arg(format!("volume={:.2}", vol_float))
                    .arg("!")
                    .arg("autoaudiosink")
                    .arg("sync=false")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .ok()
            }
        };

        self.child = child;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            println!("\x1b[1;33m[audio]\x1b[0m Stopping audio receiver pipeline...");
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn set_volume(&mut self, vol: u32) {
        self.volume = vol.min(100);
        let _ = self.start();
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        let _ = self.start();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            let _ = self.start();
        } else {
            self.stop();
        }
    }

    pub fn status(&mut self) -> AudioStatus {
        let active = match self.child {
            Some(ref mut c) => match c.try_wait() {
                Ok(None) => true,
                _ => false,
            },
            None => false,
        };
        AudioStatus {
            enabled: self.enabled,
            active,
            volume: self.volume,
            muted: self.muted,
            port: self.port,
        }
    }
}

impl AudioStatus {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"active\":{},\"volume\":{},\"muted\":{},\"port\":{}}}",
            self.enabled, self.active, self.volume, self.muted, self.port
        )
    }
}
