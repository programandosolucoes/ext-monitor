# Blueprint 30: Direct ALSA IEC958 Hi-Res Audio, Sink Deduplication, IP Anti-Fragmentation, and Web Dashboard Profiles

> [!NOTE]
> 🇺🇸 English Version | [🇧🇷 Versão em Português](../pt/30-audio-hi-res-iec958-deduplicacao-sinks-anti-fragmentacao-ip-e-perfis-web.md)

*Date: 2026-10-02*  
*Status: Approved in Production and Empirically Validated with 96kHz Master Hi-Res Audio During Continuous TV Playback*  
*Author: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Overview and Engineering Scope

During continuous high-fidelity video streaming (movies and series such as Star Trek) over network (Mode 1: UDP RTP) connected to a TV via the Raspberry Pi Zero Mini-HDMI port, extensive architectural enhancements were implemented in the audio subsystem and display pipeline:

1. **Hi-Res Audio Support:** Native ALSA hardware operation at 96 kHz and 192 kHz with 24-bit/subframe IEC 60958-3 channel status byte encoding, unlocking full hardware capabilities configured in `config.txt`.
2. **Web Dashboard Audio Profile Switcher:** Ability to force-load and switch hardware audio clock profiles on demand directly via the Web UI (`http://192.168.7.2:8080`) with real-time feedback of the negotiated ALSA clock.
3. **Resolution of Duplicate Sound Sinks on Host PC:** Elimination of the race condition creating multiple virtual sinks in GNOME sound settings.
4. **Elimination of Startup Lag and Dropouts ("Long Audio Cuts"):** Complete removal of initial speech clipping after silence or dialogue pauses.
5. **Elimination of Periodic Video Flickers and Frame Jumps:** Root cause analysis and resolution of clock-induced vsync hiccups and network packet burst collisions.

---

## 2. Root Cause Analysis and Solutions

### 2.1 Duplicate Sound Devices on Host PC (`Raspberry_Pi_HDMI_Audio`)
* **Symptom:** Two identical `Raspberry_Pi_HDMI_Audio` sink devices appeared in the host system audio selector.
* **Root Cause:** In `sender/src/main.rs`, both `spawn_opus_audio_streamer` and `spawn_audio_spectrum_monitor` (`parec`) were spawned concurrently at startup. Both called `ensure_audio_sink_exists()`. Because `pactl list sinks short` was checked before the first module completed registration in PulseAudio/PipeWire, two `module-null-sink` modules were loaded simultaneously (e.g. IDs 536870916 and 536870917).
* **Fix:**
  * Added a static mutex (`static SINK_MUTEX: Mutex<()>`).
  * Implemented an active deduplication routine that parses `pactl list modules short` and unloads any redundant sink modules (`pactl unload-module <id>`).
  * Guarantees **exactly 1 clean virtual audio device** active on the host PC.

---

### 2.2 Audio Startup Lag and Long Speech Cuts (Teardown on Silence)
* **Symptom:** Audio took time to start, and during dialogue pauses, the beginning syllables of subsequent spoken sentences were cut off.
* **Root Cause:** In `receiver/src/audio.rs`, an aggressive idle timeout executed `pcm_device.take()` after 1.5 seconds of UDP silence, tearing down `/dev/snd/pcmC0D0p`. Whenever dialogue resumed, the receiver had to reopen the device and re-execute `SNDRV_PCM_IOCTL_HW_PARAMS`, `SW_PARAMS`, and `PREPARE`. On the BCM2835 ALSA driver with VideoCore IV firmware, this ioctl handshake takes **200 ms to 400 ms**, causing immediate packet loss for incoming speech.
* **Fix:**
  1. **Pre-warming at Boot:** The ALSA device is opened immediately during receiver thread initialization (`AlsaHdmiDevice::open(initial_rate)`).
  2. **Persistent Device Session:** On silence or timeout, `pcm_device` is **never destroyed**.
  3. **Instant Underrun Recovery with Retry:** In `write_frames`, if a buffer underrun (`EPIPE`) occurs, `SNDRV_PCM_IOCTL_PREPARE` is called and the same frame buffer is rewritten within the same execution cycle:
     ```rust
     if err.raw_os_error() == Some(libc::EPIPE) {
         unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_PREPARE as _) };
         let retry_ret = unsafe { libc::ioctl(self.fd, SNDRV_PCM_IOCTL_WRITEI_FRAMES as _, &mut xfer) };
         if retry_ret >= 0 { return Ok(()); }
     }
     ```
  4. **Host Buffer Tuning:** Calibrated GStreamer's `pulsesrc` with `buffer-time=20000` (20 ms) and `latency-time=5000` (5 ms).

---

### 2.3 Periodic 30-Second Video Flickers (Clock Jitter)
* **Symptom:** Screen flickers and frame stutter occurring every 30 seconds.
* **Root Cause:** `sender/src/time_sync.rs` requested `POST /api/time/sync` every 30 seconds. In `receiver/src/web.rs`, the endpoint unconditionally called `libc::settimeofday(&tv, std::ptr::null())` with `tv_usec: 0`. Resetting `CLOCK_REALTIME` introduces time jumps in the Linux kernel, disrupting the kernel timer wheel and V4L2/KMS vsync deadlines.
* **Fix:**
  * In `receiver/src/web.rs`, `settimeofday` is only executed if the system year is 1970 (cold boot without RTC) or if clock drift exceeds 5 seconds. Routine micro-adjustments during streaming are ignored.
  * Increased sender sync interval from 30 seconds to 300 seconds (5 minutes).

---

### 2.4 Audio Glitches and Video Flickers from IP Packet Fragmentation
* **Symptom:** Audio dropouts and concurrent video glitches under continuous streaming.
* **Low-Level Investigation via `tcpdump` and SNMP Metrics:**
  * Captured UDP packets revealed audio packets transmitted at **2048 bytes** length.
  * The network interface (`enx...`) has an **MTU of 1500 bytes**.
  * Audio packets exceeded the 1472-byte payload threshold and underwent **mandatory IP fragmentation** into 2 packets.
  * Kernel SNMP statistics on Pi Zero (`/proc/net/snmp`) showed:
    $$\text{ReasmReqds} = 1,004,049 \quad \text{fragment reassembly requests}$$
  * Losing a single fragment resulted in dropping the entire audio buffer.
  * Reassembling 200+ fragments/sec on the 1 GHz ARM11 single-core CPU delayed H.264 video RTP packet processing, inducing display flickers.
* **Fix:**
  * Added `audiobuffersplit output-buffer-size=1024` into the GStreamer audio pipeline.
  * Each UDP packet is now exactly **1024 bytes** (256 stereo 16-bit frames).
  * Total frame length on wire: $1024 + 8\text{ (UDP)} + 20\text{ (IPv4)} = \mathbf{1052\text{ bytes}} < 1500\text{ MTU}$.
  * **IP fragmentation dropped to ZERO**.
  * Optimized unity-gain (100% volume) sample path to pure integer operations without floating-point math on ARM11.

---

## 3. Audio Profiles Matrix and Web UI Integration

The receiver Web Dashboard features a dedicated **HDMI Master Audio Profile & Sample Rate** panel:

| Profile | Sample Rate | Bit Depth | IEC958 Code | UDP Buffer Size | Packet Latency |
| :--- | :---: | :---: | :---: | :---: | :---: |
| 🎵 **Hi-Res Studio** (Default) | **96,000 Hz** | 24-bit / S16LE | `0x0A` | 1024 B (256 frames) | ~2.66 ms / packet |
| 🚀 **Ultra Hi-Res** | **192,000 Hz** | 24-bit / S16LE | `0x0E` | 1024 B (256 frames) | ~1.33 ms / packet |
| 🎬 **Cinema Standard** | **48,000 Hz** | 16-bit / S16LE | `0x02` | 1024 B (256 frames) | ~5.33 ms / packet |
| 💿 **CD Fidelity** | **44,100 Hz** | 16-bit / S16LE | `0x00` | 1024 B (256 frames) | ~5.80 ms / packet |

---

## 4. Empirical Validation on Real Hardware

* **Wire Traffic Verification:**
  ```text
  IP 192.168.7.1.52908 > 192.168.7.2.5004: UDP, length 1024
  IP 192.168.7.1.52908 > 192.168.7.2.5004: UDP, length 1024
  ```
  100% packets at 1024 bytes. **Zero IP fragmentation**.
* **ALSA Hardware State (`/proc/asound/card0/pcm0p/sub0/hw_params`):**
  ```text
  access: RW_INTERLEAVED
  format: IEC958_SUBFRAME_LE
  channels: 2
  rate: 96000 (96000/1)
  period_size: 2048
  buffer_size: 8192
  ```
* **CPU and Thermals:**
  * Pi Zero CPU: ~14% for `ext-receiver` with 42% overall idle headroom.
  * SoC Temperature: 50.8 °C stable under continuous playback.
