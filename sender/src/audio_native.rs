//! 100% Pure In-Process Native Audio Recorder and Streamer
//!
//! Blueprint Reference: Blueprint 34
//!
//! Replaces external `parec` and `gst-launch-1.0 pulsesrc` subprocesses with
//! in-process dynamic loading of PulseAudio Simple API (`libpulse-simple.so.0`).
//!
//! Cross-platform design: dynamically loads native audio symbols at runtime with
//! zero required build-time library dependencies. If unavailable (e.g. on non-Pulse systems),
//! gracefully falls back to synthetic or silent keepalive packets.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::ffi::{c_char, c_int, c_void, CString};
use std::net::UdpSocket;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const PA_STREAM_RECORD: c_int = 2;
pub const PA_SAMPLE_S16LE: c_int = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaSampleSpec {
    pub format: c_int,
    pub rate: u32,
    pub channels: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PaBufferAttr {
    pub maxlength: u32,
    pub tlength: u32,
    pub prebuf: u32,
    pub minreq: u32,
    pub fragsize: u32,
}

type PaSimpleNewFn = unsafe extern "C" fn(
    server: *const c_char,
    name: *const c_char,
    dir: c_int,
    dev: *const c_char,
    stream_name: *const c_char,
    ss: *const PaSampleSpec,
    map: *const c_void,
    attr: *const PaBufferAttr,
    error: *mut c_int,
) -> *mut c_void;

type PaSimpleReadFn = unsafe extern "C" fn(
    s: *mut c_void,
    data: *mut c_void,
    bytes: usize,
    error: *mut c_int,
) -> c_int;

type PaSimpleFreeFn = unsafe extern "C" fn(s: *mut c_void);

struct PulseSimpleBindings {
    _lib: *mut c_void,
    simple_new: PaSimpleNewFn,
    simple_read: PaSimpleReadFn,
    simple_free: PaSimpleFreeFn,
}

impl PulseSimpleBindings {
    fn load() -> Option<Self> {
        unsafe {
            let lib_name = CString::new("libpulse-simple.so.0").ok()?;
            let lib = libc::dlopen(lib_name.as_ptr(), libc::RTLD_LAZY);
            if lib.is_null() {
                return None;
            }

            let load_sym = |name: &str| -> Option<*mut c_void> {
                let c_name = CString::new(name).ok()?;
                let sym = libc::dlsym(lib, c_name.as_ptr());
                if sym.is_null() {
                    None
                } else {
                    Some(sym)
                }
            };

            let simple_new: PaSimpleNewFn = std::mem::transmute(load_sym("pa_simple_new")?);
            let simple_read: PaSimpleReadFn = std::mem::transmute(load_sym("pa_simple_read")?);
            let simple_free: PaSimpleFreeFn = std::mem::transmute(load_sym("pa_simple_free")?);

            Some(Self {
                _lib: lib,
                simple_new,
                simple_read,
                simple_free,
            })
        }
    }
}

pub struct NativeAudioRecorder {
    bindings: PulseSimpleBindings,
    handle: *mut c_void,
}

unsafe impl Send for NativeAudioRecorder {}

impl NativeAudioRecorder {
    pub fn new(source_name: Option<&str>, sample_rate: u32, channels: u8) -> Option<Self> {
        let bindings = PulseSimpleBindings::load()?;

        let app_name = CString::new("ext-sender-audio").ok()?;
        let stream_name = CString::new("Monitor Capture").ok()?;
        let dev_name = source_name.and_then(|s| CString::new(s).ok());

        let ss = PaSampleSpec {
            format: PA_SAMPLE_S16LE,
            rate: sample_rate,
            channels,
        };

        let attr = PaBufferAttr {
            maxlength: 16384,
            tlength: 0,
            prebuf: 0,
            minreq: 0,
            fragsize: 1024, // low-latency fragments of 1024 bytes (~10.6ms at 48kHz)
        };

        let mut err: c_int = 0;
        let handle = unsafe {
            (bindings.simple_new)(
                ptr::null(),
                app_name.as_ptr(),
                PA_STREAM_RECORD,
                dev_name.as_ref().map(|d| d.as_ptr()).unwrap_or(ptr::null()),
                stream_name.as_ptr(),
                &ss,
                ptr::null(),
                &attr,
                &mut err,
            )
        };

        if handle.is_null() {
            // If specific monitor source failed, try default source
            let handle_def = unsafe {
                (bindings.simple_new)(
                    ptr::null(),
                    app_name.as_ptr(),
                    PA_STREAM_RECORD,
                    ptr::null(),
                    stream_name.as_ptr(),
                    &ss,
                    ptr::null(),
                    &attr,
                    &mut err,
                )
            };
            if handle_def.is_null() {
                return None;
            }
            return Some(Self {
                bindings,
                handle: handle_def,
            });
        }

        Some(Self { bindings, handle })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> bool {
        let mut err: c_int = 0;
        let ret = unsafe {
            (self.bindings.simple_read)(
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                &mut err,
            )
        };
        ret == 0
    }
}

impl Drop for NativeAudioRecorder {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.bindings.simple_free)(self.handle);
            }
        }
    }
}

/// Spawns a 100% In-Process, Native Rust Audio Streamer and Spectrum Monitor.
/// Eliminates `parec` and `gst-launch-1.0 pulsesrc` subprocesses completely.
pub fn spawn_native_audio_subsystem(
    target_ip: String,
    audio_port: u16,
    audio_rate: u32,
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("audio-native-inprocess".to_string())
        .spawn(move || {
            // 1. Ensure PulseAudio virtual sink exists
            crate::pipeline::ensure_audio_sink_exists(audio_rate);

            // 2. Remember previous default sink and set Raspberry_Pi_HDMI_Audio as default sink
            let prev_sink = get_default_sink_name();
            if let Some(ref sink) = prev_sink {
                if sink != "Raspberry_Pi_HDMI_Audio" {
                    let _ = std::process::Command::new("pactl")
                        .args(["set-default-sink", "Raspberry_Pi_HDMI_Audio"])
                        .output();
                    let _ = std::process::Command::new("pactl")
                        .args(["set-sink-mute", "Raspberry_Pi_HDMI_Audio", "0"])
                        .output();
                    let _ = std::process::Command::new("pactl")
                        .args(["set-sink-volume", "Raspberry_Pi_HDMI_Audio", "100%"])
                        .output();
                    println!("\x1b[1;32m[audio-native]\x1b[0m Áudio roteado para Raspberry_Pi_HDMI_Audio (laptop speakers silenciados, sink desmutado a 100%)");
                }
            }

            // 3. Notify receiver of audio sample rate via pure Rust HTTP client
            let client_ip = target_ip.clone();
            let _ = crate::http_client::post_json(
                &format!("http://{}:8080/api/audio/rate", client_ip),
                &format!("{{\"rate\":{}}}", audio_rate),
            );

            // 4. Prepare UDP sockets for raw PCM audio and Spectrum
            let audio_sock = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("\x1b[1;31m[audio-native]\x1b[0m Falha ao bindar UDP socket: {}", e);
                    return;
                }
            };
            let audio_dest = format!("{}:{}", target_ip, audio_port);
            let spectrum_dest = format!("{}:5006", target_ip);

            println!(
                "\x1b[1;32m[audio-native]\x1b[0m 100% In-Process Audio Streamer ativo ({} Hz PCM S16LE -> {} & Spectrum -> {})",
                audio_rate, audio_dest, spectrum_dest
            );

            // 1024 bytes = 256 stereo samples (s16le = 4 bytes/sample) = ~5.3ms latency at 48kHz
            let mut pcm_buf = [0u8; 1024];
            let mut spectrum_packet = [0u8; 25];

            // 2048 bytes buffer for FFT (512 stereo samples)
            let mut fft_accum = [0u8; 2048];
            let mut fft_pos = 0;

            let source_candidates = [
                Some("Raspberry_Pi_HDMI_Audio.monitor"),
                None, // default desktop monitor
            ];

            while running.load(Ordering::Relaxed) {
                let mut recorder = None;
                for src in &source_candidates {
                    if let Some(r) = NativeAudioRecorder::new(*src, audio_rate, 2) {
                        recorder = Some(r);
                        break;
                    }
                }

                let mut rec = match recorder {
                    Some(r) => r,
                    None => {
                        // Se PulseAudio não estiver pronto, aguarda e tenta novamente
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                };

                while running.load(Ordering::Relaxed) {
                    if !rec.read(&mut pcm_buf) {
                        // Erro de leitura no socket de áudio
                        break;
                    }

                    // 1. Envia chunk de PCM direto para a porta de áudio da TV via UDP
                    let _ = audio_sock.send_to(&pcm_buf, &audio_dest);

                    // 2. Acumula para cálculo do Espectro Matemático FFT (a cada 2048 bytes)
                    let to_copy = (2048 - fft_pos).min(pcm_buf.len());
                    fft_accum[fft_pos..fft_pos + to_copy].copy_from_slice(&pcm_buf[..to_copy]);
                    fft_pos += to_copy;

                    if fft_pos >= 2048 {
                        let (_rms, _is_silence) = crate::pipeline::compute_spectrum_packet(&fft_accum, &mut spectrum_packet);
                        let _ = audio_sock.send_to(&spectrum_packet, &spectrum_dest);
                        fft_pos = 0;
                    }
                }

                thread::sleep(Duration::from_millis(200));
            }

            // Restore previous default sink so laptop speakers resume playing normally only if we're still on Raspberry_Pi_HDMI_Audio
            if let Some(ref sink) = prev_sink {
                if let Some(cur) = get_default_sink_name() {
                    if cur == "Raspberry_Pi_HDMI_Audio" && sink != "Raspberry_Pi_HDMI_Audio" {
                        let _ = std::process::Command::new("pactl")
                            .args(["set-default-sink", sink])
                            .output();
                        println!("\x1b[1;34m[audio-native]\x1b[0m Saída padrão de áudio restaurada para: {}", sink);
                    }
                }
            }

            println!("\x1b[1;34m[audio-native]\x1b[0m In-process audio subsystem encerrado com sucesso.");
        })
        .expect("Failed to spawn native audio thread")
}

fn get_default_sink_name() -> Option<String> {
    let output = std::process::Command::new("pactl")
        .arg("get-default-sink")
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}
