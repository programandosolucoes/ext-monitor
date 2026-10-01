# Blueprint 23: Miracast GPU Hardware Acceleration & Automated Host Launcher

> [🇧🇷 Versão em Português](../pt/23-aceleracao-hardware-gpu-miracast-e-launcher-host.md) | 🇺🇸 English Version

*Date: 2026-10-01*  
*Status: Implemented, Validated on Real Hardware (AMD Radeon 610M / RDNA2 / Mendocino), and Operational*  
*Author: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Overview and Problem Context

When projecting a Linux desktop to the Raspberry Pi Zero W appliance in **Mode 2 (Miracast / Wi-Fi Display)** using the GNOME network projection utility (`gnome-network-displays`), GStreamer typically defaults to CPU-intensive software encoding (`x264enc`) unless specific feature rank overrides are provided. This produces substantial CPU load on the host laptop, induces thermal throttling, and introduces frame delivery latency.

To achieve sub-realtime wireless mirroring with hardware H.264 encoding across diverse laptop platforms, two requirements emerged:
1. **Interactive Web UI Guidance:** Clear tooltips and a dedicated GPU Command Box on the receiver's Web UI (`http://192.168.7.2:8080`) providing copy-paste commands tailored for **AMD**, **Intel**, **NVIDIA**, and **Auto** configurations.
2. **Automated Zero-Friction Launching:** When the user clicks "Conectar Miracast" in the Web UI, the Raspberry Pi Zero should signal the laptop's background agent (`ext-sender`) over UDP port `5001`. The host agent must automatically probe the host system's GPU vendor, apply optimal GStreamer hardware acceleration ranking, update desktop launcher entries, and spawn `gnome-network-displays` with low-latency parameters.

---

## 2. Deterministic Sysfs GPU Vendor Detection

Rather than relying on shell parsing or external tools, the host agent detects the laptop's GPU directly through the Linux Direct Rendering Manager (DRM) sysfs hierarchy at `/sys/class/drm/card*/device/vendor`:

| Vendor | PCI Vendor ID | Hardware Acceleration Engine | Prioritized GStreamer Encoders | Ranking Parameter |
| :--- | :--- | :--- | :--- | :--- |
| **AMD** | `0x1002` | VA-API / AMDGPU (RDNA, Vega, Polaris) | `vaapih264enc`, `vah264enc` | `GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX` |
| **Intel** | `0x8086` | Intel Media SDK / VA-API / QSV | `vaapih264enc`, `vah264enc`, `qsvh264enc` | `GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX` |
| **NVIDIA** | `0x10de` | NVENC / CUDA / VDPAU | `nvh264enc`, `vaapih264enc` | `GST_PLUGIN_FEATURE_RANK=nvh264enc:MAX,vaapih264enc:MAX` |
| **Hybrid** | Multiple | Discrete NVIDIA + AMD/Intel Integrated | `nvh264enc`, `vaapih264enc`, `vah264enc` | `GST_PLUGIN_FEATURE_RANK=nvh264enc:MAX,vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX` |

A fallback mechanism via `lspci` is provided if sysfs is restricted or virtualized.

---

## 3. End-to-End Control Architecture

```
 ┌──────────────────────────────────────────────────────────────────┐
 │                    Raspberry Pi Zero Appliance                   │
 │                                                                  │
 │  Web UI (http://192.168.7.2:8080)                                │
 │    - Tooltips & GPU Selector Box (AMD / Intel / NVIDIA / Auto)   │
 │    - "Conectar Miracast (Win+K / Linux GPU)" Button              │
 │                           │                                      │
 │                           ▼                                      │
 │         POST /api/host/control {"action": "launch_miracast"}     │
 │                           │                                      │
 │                           ▼                                      │
 │         UdpSocket::send_to("192.168.7.1:5001")                   │
 └───────────────────────────┬──────────────────────────────────────┘
                             │
                      UDP 5001 Datagram
                             │
 ┌───────────────────────────▼──────────────────────────────────────┐
 │                       Laptop Host (Linux)                        │
 │                                                                  │
 │  ext-sender (Rust Daemon - ext-monitor-sender.service)           │
 │    - UDP 5001 Listener (control.rs)                              │
 │    - Action: ControlAction::LaunchMiracast                       │
 │    - Handler: miracast_launcher::launch_gnome_network_displays() │
 │    - Atomic Debounce (< 2.5s) prevents duplicate instances       │
 │    - Preserves DISPLAY, WAYLAND_DISPLAY, XDG_RUNTIME_DIR          │
 │    - Updates ~/.local/share/applications/org.gnome.NetworkDisplays│
 │                           │                                      │
 │                           ▼                                      │
 │    Spawns: env GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,...      │
 │            gnome-network-displays                                │
 └──────────────────────────────────────────────────────────────────┘
```

---

## 4. Implementation Details

### 4.1 Host Agent Miracast Launcher (`sender/src/miracast_launcher.rs`)
- **Sysfs Scrutiny:** Reads `/sys/class/drm/card[0-9]*/device/vendor` without spawning external processes.
- **Desktop Entry Persistence:** Overwrites or creates `~/.local/share/applications/org.gnome.NetworkDisplays.desktop` so that subsequent launches from GNOME Shell dash also leverage GPU acceleration.
- **Session Environment Injection:** Safely binds to the active display manager session (`DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`, `XDG_RUNTIME_DIR=/run/user/<UID>`).
- **Atomic Debounce:** Employs an `AtomicU64` timestamp preventing race conditions or multiple instances if rapid network packets occur within a 2.5-second window.

### 4.2 Web UI & API Enhancements (`receiver/src/web_ui.rs`, `receiver/src/web.rs`)
- **GPU Interactive Command Box:** Adds visual selection chips (`AMD`, `Intel`, `NVIDIA`, `Auto`) with real-time command syntax preview and 1-click clipboard copying.
- **Contextual Tooltips:** Informs users directly on the connect buttons of the exact command needed if launching manually from a terminal.
- **Packet Deduplication:** Ensures datagram forwarding to `192.168.7.1:5001` does not double-transmit when accessed from the USB interface itself.

---

## 5. Verification on Hardware

Empirical testing on an AMD Ryzen Mendocino laptop (AMD Radeon 610M GPU):
1. **Trigger:** Clicked "Conectar Miracast" in the Web UI at `http://192.168.7.2:8080`.
2. **Host Daemon Log (`journalctl --user -u ext-monitor-sender`):**
   ```text
   [*] Web Command: Lançar Miracast (GNOME Network Displays) com aceleração de GPU
   [miracast-launcher] GPU Detectada: AMD Radeon (VA-API / RDNA / Vega / Mendocino) (Encoder: vaapih264enc / vah264enc)
   [miracast-launcher] Aplicando: GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX
   [miracast-launcher] gnome-network-displays iniciado com sucesso (PID: 817033) com aceleração por GPU!
   ```
3. **Environment Audit:**
   ```bash
   $ tr '\0' '\n' < /proc/817033/environ | grep -E "GST_|NETWORK_DISPLAYS"
   GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX
   NETWORK_DISPLAYS_H264_ENC=vaapih264enc
   ```

---

## 6. Deliverables & Affected Files

- `sender/src/miracast_launcher.rs`: Added GPU hardware detection, desktop entry updater, and subprocess launcher with atomic debounce.
- `sender/src/control.rs`: Added `LaunchMiracast` action parser.
- `sender/src/main.rs`: Integrated `miracast_launcher` module and control dispatch.
- `receiver/src/web.rs`: Added deduplicated UDP 5001 forwarding for Miracast activation.
- `receiver/src/web_ui.rs`: Added GPU chips, preview box, copy button, and contextual tooltips.
- `release/frozen-v0.3.0/ext-receiver`: Recompiled and packaged for ARMv6 musl.
- `build-appliance/boot/initramfs.cpio.gz`: Packaged and OTA-flashed to Pi Zero SD card.
