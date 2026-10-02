//! Low-Latency HDMI Digital Audio Subsystem for ext-receiver
//!
//! Ingests 48000 Hz 16-bit stereo PCM audio streams on UDP port 5004 (or configurable)
//! and renders directly to the BCM2835 ALSA HDMI sound card (/dev/snd/pcmC0D0p)
//! using direct kernel ALSA ioctls with zero external library or GStreamer dependencies.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs::{File, OpenOptions};
use std::net::UdpSocket;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const DEFAULT_AUDIO_PORT: u16 = 5004;

// ALSA ioctl definitions for Linux (portable across 32-bit and 64-bit)
const SNDRV_PCM_IOCTL_HW_PARAMS: libc::c_ulong = 0xc25c4111;
const SNDRV_PCM_IOCTL_SW_PARAMS: libc::c_ulong = 0xc0684113;
const SNDRV_PCM_IOCTL_PREPARE: libc::c_ulong = 0x00004140;
const SNDRV_PCM_IOCTL_DRAIN: libc::c_ulong = 0x00004144;
const SNDRV_PCM_IOCTL_DROP: libc::c_ulong = 0x00004143;
const SNDRV_PCM_IOCTL_WRITEI_FRAMES: libc::c_ulong = 0x400c4150;

const SNDRV_PCM_HW_PARAM_ACCESS: usize = 0;
const SNDRV_PCM_HW_PARAM_FORMAT: usize = 1;
const SNDRV_PCM_HW_PARAM_FIRST_INTERVAL: usize = 8;
const SNDRV_PCM_HW_PARAM_CHANNELS: usize = 10;
const SNDRV_PCM_HW_PARAM_RATE: usize = 11;
const SNDRV_PCM_HW_PARAM_PERIOD_SIZE: usize = 13;
const SNDRV_PCM_HW_PARAM_PERIODS: usize = 15;

const SNDRV_PCM_ACCESS_RW_INTERLEAVED: u32 = 3;
const SNDRV_PCM_FORMAT_IEC958_SUBFRAME_LE: u32 = 18;

#[repr(C)]
#[derive(Copy, Clone)]
struct SndMask {
    bits: [u32; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct SndInterval {
    min: u32,
    max: u32,
    flags: u32,
}

#[repr(C)]
struct SndPcmHwParams {
    flags: u32,
    masks: [SndMask; 3],       // ACCESS (0), FORMAT (1), SUBFORMAT (2)
    mres: [SndMask; 5],
    intervals: [SndInterval; 12], // SAMPLE_BITS (8) .. TICK_TIME (19)
    ires: [SndInterval; 9],
    rmask: u32,
    cmask: u32,
    info: u32,
    msbits: u32,
    rate_num: u32,
    rate_den: u32,
    fifo_size: libc::c_ulong,
    reserved: [u8; 64],
}

#[repr(C)]
struct SndPcmSwParams {
    tstamp_mode: u32,
    period_step: u32,
    sleep_min: u32,
    avail_min: libc::c_ulong,
    xfer_align: libc::c_ulong,
    start_threshold: libc::c_ulong,
    stop_threshold: libc::c_ulong,
    silence_threshold: libc::c_ulong,
    silence_size: libc::c_ulong,
    boundary: libc::c_ulong,
    proto: u32,
    tstamp_type: u32,
    reserved: [u8; 56],
}

#[repr(C)]
struct SndXferi {
    result: libc::c_long,
    buf: *const libc::c_void,
    frames: libc::c_ulong,
}

fn param_init(p: &mut SndPcmHwParams) {
    unsafe {
        std::ptr::write_bytes(p, 0, 1);
        for m in &mut p.masks {
            m.bits[0] = !0;
            m.bits[1] = !0;
        }
        for i in &mut p.intervals {
            i.min = 0;
            i.max = !0;
        }
        p.rmask = !0;
        p.info = !0;
    }
}

fn set_mask(p: &mut SndPcmHwParams, param: usize, bit: u32) {
    if param < p.masks.len() {
        let m = &mut p.masks[param];
        m.bits[0] = 0;
        m.bits[1] = 0;
        m.bits[(bit >> 5) as usize] |= 1 << (bit & 31);
    }
}

fn set_int(p: &mut SndPcmHwParams, param: usize, val: u32) {
    if param >= SNDRV_PCM_HW_PARAM_FIRST_INTERVAL {
        let idx = param - SNDRV_PCM_HW_PARAM_FIRST_INTERVAL;
        if idx < p.intervals.len() {
            let i = &mut p.intervals[idx];
            i.min = val;
            i.max = val;
            i.flags |= 4; // integer = 1
        }
    }
}

fn set_min(p: &mut SndPcmHwParams, param: usize, val: u32) {
    if param >= SNDRV_PCM_HW_PARAM_FIRST_INTERVAL {
        let idx = param - SNDRV_PCM_HW_PARAM_FIRST_INTERVAL;
        if idx < p.intervals.len() {
            let i = &mut p.intervals[idx];
            i.min = val;
        }
    }
}

fn get_iec958_rate_code(rate: u32) -> u8 {
    match rate {
        32000 => 0x03,
        44100 => 0x00,
        48000 => 0x02,
        88200 => 0x08,
        96000 => 0x0A,
        176400 => 0x0C,
        192000 => 0x0E,
        _ => 0x02,
    }
}

/// Native ALSA PCM Device wrapper for BCM2835 HDMI sound output
struct AlsaHdmiDevice {
    _file: File,
    fd: RawFd,
    rate: u32,
}

impl AlsaHdmiDevice {
    fn open(rate: u32) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/snd/pcmC0D0p")?;
        let fd = file.as_raw_fd();

        let mut hw_params: SndPcmHwParams = unsafe { std::mem::zeroed() };
        param_init(&mut hw_params);
        set_mask(&mut hw_params, SNDRV_PCM_HW_PARAM_ACCESS, SNDRV_PCM_ACCESS_RW_INTERLEAVED);
        set_mask(&mut hw_params, SNDRV_PCM_HW_PARAM_FORMAT, SNDRV_PCM_FORMAT_IEC958_SUBFRAME_LE);
        set_int(&mut hw_params, SNDRV_PCM_HW_PARAM_CHANNELS, 2);
        set_int(&mut hw_params, SNDRV_PCM_HW_PARAM_RATE, rate);

        let period_size: u32 = match rate {
            192000 => 2048,
            96000 => 2048,
            88200 => 2048,
            _ => 1024,
        };
        set_min(&mut hw_params, SNDRV_PCM_HW_PARAM_PERIOD_SIZE, period_size);
        set_int(&mut hw_params, SNDRV_PCM_HW_PARAM_PERIODS, 4);

        let ret = unsafe { libc::ioctl(fd, SNDRV_PCM_IOCTL_HW_PARAMS as _, &mut hw_params) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let mut sw_params: SndPcmSwParams = unsafe { std::mem::zeroed() };
        sw_params.avail_min = period_size as libc::c_ulong;
        sw_params.start_threshold = period_size as libc::c_ulong;
        sw_params.stop_threshold = (period_size * 4) as libc::c_ulong;

        let ret = unsafe { libc::ioctl(fd, SNDRV_PCM_IOCTL_SW_PARAMS as _, &mut sw_params) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let ret = unsafe { libc::ioctl(fd, SNDRV_PCM_IOCTL_PREPARE as _) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self { _file: file, fd, rate })
    }

    fn write_frames(&mut self, frames: &[u32], frame_count: usize) -> Result<(), std::io::Error> {
        let mut xfer = SndXferi {
            result: 0,
            buf: frames.as_ptr() as *const libc::c_void,
            frames: frame_count as libc::c_ulong,
        };

        let ret = unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_WRITEI_FRAMES as _, &mut xfer) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EPIPE) {
                // Buffer underrun: re-prepare PCM and immediately retry write so no initial audio frames are dropped
                unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_PREPARE as _) };
                let retry_ret = unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_WRITEI_FRAMES as _, &mut xfer) };
                if retry_ret >= 0 {
                    return Ok(());
                }
            }
            return Err(err);
        }
        Ok(())
    }

    fn drain(&mut self) {
        unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_DRAIN as _) };
    }

    fn drop_playback(&mut self) {
        unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_DROP as _) };
    }
}

#[derive(Debug, Clone)]
pub struct AudioStatus {
    pub enabled: bool,
    pub active: bool,
    pub volume: u32,
    pub muted: bool,
    pub port: u16,
    pub rate: u32,
}

impl AudioStatus {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"active\":{},\"volume\":{},\"muted\":{},\"port\":{},\"rate\":{}}}",
            self.enabled, self.active, self.volume, self.muted, self.port, self.rate
        )
    }
}

pub struct AudioReceiver {
    thread_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    muted: Arc<AtomicBool>,
    rate: Arc<AtomicU32>,
    port: u16,
    enabled: bool,
}

impl AudioReceiver {
    pub fn new(port: u16) -> Self {
        Self {
            thread_handle: None,
            running: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(100)),
            muted: Arc::new(AtomicBool::new(false)),
            rate: Arc::new(AtomicU32::new(96000)),
            port,
            enabled: true,
        }
    }

    pub fn set_rate(&mut self, r: u32) {
        println!("\x1b[1;36m[audio-native]\x1b[0m Audio target sample rate configured: {} Hz", r);
        self.rate.store(r, Ordering::SeqCst);
    }

    pub fn rate(&self) -> u32 {
        self.rate.load(Ordering::Relaxed)
    }

    pub fn start(&mut self) -> Result<(), std::io::Error> {
        if !self.enabled {
            return Ok(());
        }
        self.stop();

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let active = self.active.clone();
        let volume = self.volume.clone();
        let muted = self.muted.clone();
        let rate = self.rate.clone();
        let port = self.port;

        println!(
            "\x1b[1;34m[audio-native]\x1b[0m Starting 100% Pure Rust HDMI ALSA Audio Receiver on UDP port {} (Hi-Res {} Hz, vol: {}%, muted: {})...",
            port, rate.load(Ordering::Relaxed), volume.load(Ordering::Relaxed), muted.load(Ordering::Relaxed)
        );

        let handle = thread::Builder::new()
            .name("audio-alsa-rx".to_string())
            .spawn(move || {
                let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[audio-native] Failed to bind UDP {}: {}\x1b[0m", port, e);
                        return;
                    }
                };
                let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

                let initial_rate = rate.load(Ordering::Relaxed);
                let mut pcm_device: Option<AlsaHdmiDevice> = AlsaHdmiDevice::open(initial_rate).ok();
                if let Some(ref d) = pcm_device {
                    println!("\x1b[1;32m[audio-native] Pre-warmed /dev/snd/pcmC0D0p ({} Hz Hi-Res Stereo IEC958)\x1b[0m", d.rate);
                }
                let mut frame_counter: usize = 0;
                let mut last_active = Instant::now();
                let mut udp_buf = [0u8; 16384];
                let mut iec_buffer: Vec<u32> = Vec::with_capacity(4096);

                while running.load(Ordering::Relaxed) {
                    match socket.recv_from(&mut udp_buf) {
                        Ok((len, _src)) => {
                            if len < 4 || len % 4 != 0 {
                                continue;
                            }
                            let frame_count = len / 4;

                            let current_rate = rate.load(Ordering::Relaxed);

                            // If hardware device rate differs from requested target rate, reconfigure ALSA
                            if let Some(ref dev) = pcm_device {
                                if dev.rate != current_rate {
                                    println!("\x1b[1;33m[audio-native] Switching ALSA hardware rate: {} Hz -> {} Hz\x1b[0m", dev.rate, current_rate);
                                    pcm_device = None;
                                }
                            }

                            if pcm_device.is_none() {
                                match AlsaHdmiDevice::open(current_rate) {
                                    Ok(dev) => {
                                        println!("\x1b[1;32m[audio-native] Opened /dev/snd/pcmC0D0p ({} Hz Hi-Res Stereo IEC958)\x1b[0m", current_rate);
                                        pcm_device = Some(dev);
                                    }
                                    Err(e) => {
                                        eprintln!("\x1b[1;33m[audio-native] Cannot open /dev/snd/pcmC0D0p at {} Hz: {}. Retrying at 48000 Hz...\x1b[0m", current_rate, e);
                                        if current_rate != 48000 {
                                            rate.store(48000, Ordering::Relaxed);
                                            if let Ok(dev) = AlsaHdmiDevice::open(48000) {
                                                println!("\x1b[1;32m[audio-native] Fallback opened /dev/snd/pcmC0D0p at 48000 Hz\x1b[0m");
                                                pcm_device = Some(dev);
                                            }
                                        }
                                        if pcm_device.is_none() {
                                            thread::sleep(Duration::from_millis(200));
                                            continue;
                                        }
                                    }
                                }
                            }

                            active.store(true, Ordering::Relaxed);
                            last_active = Instant::now();

                            let active_rate = pcm_device.as_ref().map(|d| d.rate).unwrap_or(current_rate);
                            let rate_code = get_iec958_rate_code(active_rate);

                            let status_bytes: [u8; 24] = [
                                0x00, // Consumer mode, PCM audio, No emphasis
                                0x00, // General category
                                0x00, // Source / channel
                                rate_code, // Sampling frequency (IEC 60958-3)
                                0x02, // 16-bit word length
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ];

                            let vol = volume.load(Ordering::Relaxed);
                            let is_muted = muted.load(Ordering::Relaxed);
                            let vol_scale = if is_muted {
                                0.0f32
                            } else {
                                (vol as f32) / 100.0f32
                            };

                            iec_buffer.clear();

                            for i in 0..frame_count {
                                let l_raw = i16::from_le_bytes([udp_buf[i * 4], udp_buf[i * 4 + 1]]);
                                let r_raw = i16::from_le_bytes([udp_buf[i * 4 + 2], udp_buf[i * 4 + 3]]);

                                let s_l = ((l_raw as f32) * vol_scale) as i16 as u16 as u32;
                                let s_r = ((r_raw as f32) * vol_scale) as i16 as u16 as u32;

                                let block_frame = frame_counter % 192;
                                let status_byte = status_bytes[block_frame / 8];
                                let status_bit = (status_byte >> (block_frame % 8)) & 1;

                                // Channel 0: Left
                                let mut sub_l = (s_l << 12) & 0x0FFF_F000;
                                if status_bit != 0 {
                                    sub_l |= 0x4000_0000;
                                }
                                // Even parity over bits 4..30 in single CPU cycle
                                if ((sub_l & 0x7FFF_FFF0).count_ones() & 1) != 0 {
                                    sub_l |= 0x8000_0000;
                                }
                                sub_l |= if block_frame == 0 { 0x08 } else { 0x02 };

                                // Channel 1: Right
                                let mut sub_r = (s_r << 12) & 0x0FFF_F000;
                                if status_bit != 0 {
                                    sub_r |= 0x4000_0000;
                                }
                                if ((sub_r & 0x7FFF_FFF0).count_ones() & 1) != 0 {
                                    sub_r |= 0x8000_0000;
                                }
                                sub_r |= 0x04;

                                iec_buffer.push(sub_l);
                                iec_buffer.push(sub_r);

                                frame_counter = frame_counter.wrapping_add(1);
                            }

                            if let Some(ref mut dev) = pcm_device {
                                let _ = dev.write_frames(&iec_buffer, frame_count);
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock => {
                            if active.load(Ordering::Relaxed) && last_active.elapsed() > Duration::from_millis(1500) {
                                active.store(false, Ordering::Relaxed);
                                // Note: We deliberately KEEP pcm_device open and warm!
                                // Tearing down and reopening /dev/snd/pcmC0D0p takes ~300ms on the bcm2835
                                // ALSA driver, which causes audio delay and clipped syllables when dialogue resumes.
                            }
                        }
                        Err(e) => {
                            eprintln!("\x1b[1;33m[audio-native] UDP recv error: {}\x1b[0m", e);
                        }
                    }
                }

                if let Some(ref mut dev) = pcm_device {
                    dev.drain();
                    dev.drop_playback();
                }
                active.store(false, Ordering::Relaxed);
                println!("\x1b[1;34m[audio-native]\x1b[0m Native ALSA Audio Receiver stopped.");
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        self.thread_handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            println!("\x1b[1;33m[audio]\x1b[0m Stopping native ALSA audio receiver thread...");
            let _ = handle.join();
        }
        self.active.store(false, Ordering::SeqCst);
    }

    pub fn set_volume(&mut self, vol: u32) {
        self.volume.store(vol.min(100), Ordering::SeqCst);
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted.store(muted, Ordering::SeqCst);
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
        AudioStatus {
            enabled: self.enabled,
            active: self.active.load(Ordering::Relaxed),
            volume: self.volume.load(Ordering::Relaxed),
            muted: self.muted.load(Ordering::Relaxed),
            port: self.port,
            rate: self.rate.load(Ordering::Relaxed),
        }
    }
}
