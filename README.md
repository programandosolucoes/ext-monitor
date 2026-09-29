# ext-monitor: GPU Hardware-Offloaded USB Second Monitor Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust_100%25-orange.svg)](https://www.rust-lang.org/)
[![Hardware: BCM2835 / VideoCore IV](https://img.shields.io/badge/Hardware-Broadcom_BCM2835-red.svg)](https://www.raspberrypi.com/)
[![Latency: < 15ms](https://img.shields.io/badge/Latency-%3C15ms_Drop--on--Late-brightgreen.svg)]()
[![OS: Linux & Windows](https://img.shields.io/badge/OS-Linux_Wayland_%26_Windows_10%2F11-blueviolet.svg)]()

A high-performance, 100% native Rust engine designed to transform a **Raspberry Pi Zero (v1.2 / v1.3 / W / Zero 2 W)** connected via a single standard **Micro-USB 2.0 cable (OTG 480 Mbps)** into a **zero-latency hardware HDMI second display** for Linux (Wayland / GNOME Mutter) and Windows 10/11 (native Miracast / `Win + K`).

Delivers fluid **60 FPS** at native panel resolution (1280x720 / 1600x900) with **sub-15ms latency** and **~0% CPU load** on the Pi Zero through zero-copy GPU pipeline offloading (Broadcom VideoCore IV V4L2 M2M hardware decoder + KMS/DRM overlay scanout, driven by AMD VA-API, NVIDIA NVENC, or Intel QSV hardware encoding on the host).

---

![Raspberry Pi Zero Dual Monitor Desk Setup](docs/assets/hero-setup.jpg)

---

## 🎯 What It Does & What It Is For

Modern operating systems lack low-friction, driverless ways to add a dedicated secondary display over standard USB cables without expensive DisplayLink adapters or sluggish VNC/RDP network mirrors.

`ext-monitor` solves this at the silicon level:
* **True Physical Display Extension:** Exposes a virtual HDMI output in GNOME Wayland / Mutter or native Windows Wireless Display (`Win + K`). Windows and workspaces snap, drag, and maximize natively.
* **Single-Cable Simplicity:** The Raspberry Pi Zero is powered and communicates entirely over a single micro-USB cable plugged into the host PC. No external power bricks, no extra dongles.
* **100% RAM Embedded Appliance:** Boots directly from RAM (`initramfs.cpio.gz`) in **under 1.8 seconds**. The micro-SD card is uncoupled after boot, ensuring **zero risk of filesystem corruption** upon sudden disconnection.
* **Sub-15ms Real-Time Response:** Implements multi-tier *Drop-on-Late* frame decimation (Moonlight/Sunshine architecture) with 3-deep LIFO ring buffers, eliminating buffer bloat and mouse pointer latency.
* **Zero Host Drivers on Windows & 1-Line Setup on Linux:** Works out-of-the-box with Windows 10/11 via native Miracast (RTSP port 7236). On Linux, connect in one click using `curl -sSL http://192.168.7.2:8080/connect.sh | bash`.

---

## 🔌 Hardware Setup & Correct Port Wiring

<p align="center">
  <img src="docs/assets/hardware-macro.jpg" alt="Raspberry Pi Zero BCM2835 with High-Speed USB and HDMI Connections" width="760" />
</p>

### Port Pinout & Anti-Error Guide

```
                            [ 40-Pin GPIO Header ]
  +------------------------------------------------------------------------+
  | [Micro-SD Slot]                                                        |
  | (SanDisk Card)                [ BCM2835 SoC ]                          |
  |                                                                 [CSI]  |
  +---------[ mini-HDMI ]-----------[ Micro-USB OTG ]---------[ PWR IN ]---+
                   │                        │                     │
                   │                        │                     └── [LEAVE EMPTY!] DO NOT PLUG
                   │                        │                         (Host PC supplies 5V via OTG)
                   │                        └── Micro-USB to PC / Laptop USB Port
                   │                            (5V Bus Power + 480 Mbps Data Transport)
                   │
                   └── mini-HDMI Cable to Secondary Monitor or TV
```

* **mini-HDMI Port (Left):** Dedicated video scanout to your secondary monitor or television.
* **Center Micro-USB Port (OTG / Data + Power):** Plugged directly into the host PC or laptop. Delivers 5V power and handles all data traffic (480 Mbps).
* **Right Micro-USB Port (PWR IN):** **MUST REMAIN EMPTY!** Do not attach an external power supply when connected to a computer to avoid ground loops.
* **Power Draw:** Only ~0.8W (safely within any standard USB 2.0 port power spec).
* **Operating Temperature:** ~44.5°C sustained load (zero thermal throttling).

---

## 🏗️ System Architecture

```
[ HOST PC (Linux Wayland / Multi-GPU / All Distros) ]
  ├── Dual-Engine Capture Pipeline:
  │     ├── Engine 1: Direct Kernel DRM/KMS Scanout (/dev/dri/card*)
  │     │     ├── Zero-copy hardware scanout capture via ioctl GETFB2 + PRIME DMA-BUF export
  │     │     ├── Completely eliminates window occlusion culling & mouse-leave video freezing
  │     │     └── Operates safely without root via Linux capability (cap_sys_admin+ep)
  │     └── Engine 2: GNOME Mutter D-Bus Screencast + PipeWire Graph Linking
  │           └── High-level compositor capture with embedded hardware cursor and auto-recovery
  └── Rust `ext-sender` (GPU Hardware Transmission Engine v0.3.0):
        ├── Hardware Encoder Engine:
        │     ├── AMD / Intel: VA-API Direct DMA-BUF -> `vah264enc` (60 FPS, Adaptive VBR)
        │     ├── NVIDIA: NVENC Zero-Latency -> `nvh264enc` (preset=p1, zerolatency=true)
        │     ├── Intel: QuickSync -> `qsvh264enc` (rate-control=cbr)
        │     └── CPU Software: x264 zerolatency ultrafast fallback
        ├── Transmission Modes (Continuous CFR vs Economy):
        │     └── Default Continuous CFR (30/60 FPS) ensures uninterrupted YouTube/video playback
        ├── Pacing & Leaky Queues:
        │     └── Multi-layer drop-on-late queues drop delayed frames before network transmission
        └── Dual Transport: UDP RTP port 5000 (Mode 1) or USB Bulk Direct FunctionFS (Mode 3)
              │
              ▼ [ Micro-USB OTG Cable / Network Link ]
              │
[ RECEIVER APPLIANCE (Raspberry Pi Zero W / BCM2835 VideoCore IV) ]
  └── Rust `ext-receiver` (All-in-One Multi-Mode Display Daemon):
        ├── Built-in Zero-Gateway DHCP Server: Assigns 192.168.7.1 to PC without breaking main Wi-Fi/Ethernet
        ├── Web Dashboard & Control Socket: Serves management UI on port 8080 and handles hot-apply on UDP 5001
        ├── WFD Miracast RTSP Server: Listens on TCP 7236 for native Windows 10/11 Win + K projections
        ├── Broadcom VideoCore IV Hardware VPU Decoder:
        │     └── Decodes RFC 6184 H.264 stream via `/dev/video10` (V4L2 M2M `bcm2835-codec`)
        └── KMS/DRM Direct Scanout:
              └── Commits NV12/RGB planes directly to HDMI without X11 or Wayland compositor overhead
```

---

## 🔀 Three Concurrent Operating Modes

The appliance boots an active composite USB gadget providing three concurrent services:

| Mode | Target Platform | Protocol / Port | Latency | Key Advantage |
| :--- | :--- | :--- | :---: | :--- |
| **Mode 1: Linux Wayland** | Linux (GNOME / KDE) | UDP 5000 (Video) + **UDP 5004 (Audio Opus)** | **< 15 ms** | 60 FPS H.264 + 48kHz HDMI Digital Audio, hot-apply bitrate & volume |
| **Mode 2: Windows Miracast** | Windows 10 / 11 | RTSP Port 7236 (Wi-Fi Display) | **~30 ms** | Zero drivers required on Windows; connect using **`Win + K`** |
| **Mode 3: USB Bulk Direct** | Offline / High-Security | USB FunctionFS (`0xFF` Bulk) | **< 1 ms** | Bypasses IP/network stack completely; works behind strict firewalls |

### Granular Mode Flags (ON / OFF Switches)
Through the embedded Web Dashboard (`http://192.168.7.2:8080`), each mode has an independent toggle switch (`flag de ligar e desligar`). Users can turn off unused modes (e.g., disable Miracast and USB Bulk to allocate 100% of the SoC RAM and bandwidth exclusively to Mode 1). All settings persist across browser refreshes (**F5**) via local storage and server-side state synchronization.

---

## ⚡ Quick Start & Usage

### 1. One-Click Linux Connection (Zero Installation)
On any Linux PC, plug the micro-USB cable into the center OTG port and run:
```bash
curl -sSL http://192.168.7.2:8080/connect.sh | bash
```

### 2. Manual CLI Transmitter Launch
```bash
# Extend desktop at 30 FPS, Economy color mode, 400 kbps (recommended):
./scripts/start.sh extend auto 30 false economy --bitrate=400

# Full 24-bit TrueColor mode with VA-API hardware encoding:
./scripts/start.sh extend vaapi 30 false full --bitrate=1500

# Mirror primary display (clone mode) at 60 FPS:
./scripts/start.sh clone auto 60
```

### 3. Native Windows 10 / 11 Setup
1. Connect the Raspberry Pi Zero micro-USB cable to your Windows PC.
2. Press **`Win + K`** on your keyboard.
3. Select **"Pi Zero Wireless Display"** from the Cast menu. Windows negotiates the connection automatically.

---

## 🌐 Web Control Panel & Telemetry Dashboard

Access `http://192.168.7.2:8080` from any browser on your network to manage the appliance:

* **Tab 1: Monitoring & Telemetry:** Real-time SoC temperature, CPU load, free RAM, and active mode toggle switches.
* **Tab 2: Stream Optimization:** Live sliders and quick buttons for Bitrate (150k to 15M), Framerate (15, 30, 60 FPS), Color Profiles (24-bit TrueColor, 256 Economy, Monochrome), **HDMI Digital Audio Volume (0-100%) and Mute Toggle**, and Pause/Resume.
* **Tab 3: Client Tool Downloads:** Instant downloads for standalone `ext-sender` binaries, portable `client.tar.gz`, `connect.sh`, and `99-ext-monitor.rules`.
* **Tab 4: Micro-SD & Firmware Upgrade:** Safely mount `/mnt/boot` directly over USB to upgrade firmware binaries without removing the SD card from the Pi.
* **Tab 5: Operating Manual:** Complete offline guide for zero-IP USB serial recovery (`/dev/ttyACM0`) and comparative benchmarks.

---

## 📊 Technical Comparison

| Solution | Framerate | Latency | Pi Zero CPU Load | Screen Tearing | Kernel Stability |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **GUD USB Display (Legacy)** | 5–12 FPS | > 250 ms | **100% (choked)** | Severe tearing | Mutter atomic commit failure |
| **VNC / RDP Mirror** | 15–25 FPS | 80–150 ms | **75–90%** | Block artifacts | Not recognized as physical DRM output |
| **ext-monitor (Rust + VA-API + VideoCore IV)** | **60 FPS** | **< 15 ms** | **~0.5%** | **Zero Tearing** | **100% native Wayland & Windows integration** |

---

## 📖 Manual de Operação & Documentação

* **[Manual de Operação Completo e Definitivo (docs/MANUAL-DE-OPERACAO.md)](docs/MANUAL-DE-OPERACAO.md):** Guia exaustivo de todos os scripts, modos (Linux Wayland, Windows Miracast, USB Bulk Direct), integração com GNOME Mutter/PipeWire, flags da CLI, telemetria e gravação manual do SD card.

## 📚 Technical Blueprints Index

For in-depth reverse-engineering specifications, silicon geometry analyses, and firmware details, refer to the [Technical Blueprints](docs/blueprints/README.md):

* **[Blueprint 01: 32MB Appliance Image & BCM2835 Geometry](docs/blueprints/01-imagem-32mb-e-geometria-bcm2835.md):** The 65,525 cluster Boot ROM boundary, Sector 1 alignment, and FAT16 2KB cluster formatting.
* **[Blueprint 02: End-to-End Architecture vs. GUD](docs/blueprints/02-arquitetura-transmissor-receptor-e-comparativo-gud.md):** Why GUD saturates USB with LZ4 and how H.264 V4L2 M2M achieves 60 FPS with 0.8% CPU.
* **[Blueprint 03: Packet Transmission & Drop-on-Late](docs/blueprints/03-transmissao-pacotes-drop-on-late-e-pipeline.md):** RFC 6184 NAL unit fragmentation, MTU optimization, and post-sleep queue flushing.
* **[Blueprint 04: PipeWire & Suspend/Resume Recovery](docs/blueprints/04-pipewire-mutter-screencast-e-wayland.md):** Dual Rust watchdogs for instant recovery after PC S3 sleep.
* **[Blueprint 05: CPU/GPU Optimization & Scalers](docs/blueprints/05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md):** ARM1176JZF-S compiler flags, VA-API/NVENC zero-copy, and CAS edge-sharpening filters.
* **[Blueprint 06: Concurrent Modes & USB ConfigFS Super-Gadget](docs/blueprints/06-modos-de-operacao-concorrentes-e-usb-gadget.md):** Endpoint allocation, dynamic FunctionFS class `0xFF` discovery, and UDC auto-binding.
* **[Blueprint 07: Micro-SD Protection & USB In-Situ Upgrades](docs/blueprints/07-cartao-sd-em-ram-e-upgrade-usb.md):** Pure RAM execution, zero flash wear, and FAT16 partition mounting over USB.
* **[Blueprint 08: Installation Manual & Multi-Distro Host Setup](docs/blueprints/08-manual-de-instalacao-e-portabilidade-host.md):** One-liner deployment, low-latency udev rules, and USB serial ACM recovery.
* **[Blueprint 09: Empirical Hardware Tests & Boot Diagnostics](docs/blueprints/09-testes-empiricos-e-diagnosticos-hardware.md):** Monocore Pi Zero v1.3 test matrix, VideoCore IV rainbow splash screen diagnostics, and 1.8s boot timeline.
* **[Blueprint 10: Refatoração Modular Clean Code & V4L2 M2M](docs/blueprints/10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md):** Reestruturação limpa do pipeline Rust, state machine do decodificador V4L2 M2M e alinhamento de AU.
* **[Blueprint 11: KMS DRM DMA-BUF Zero-Copy & Correção EFAULT](docs/blueprints/11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md):** Resolução definitiva do errno 14 (EFAULT), alocação prévia de arrays DRM e scanout direto no plano primário.
* **[Blueprint 12: Splash Quadrilíngue, EDID Realtime & Desconexão](docs/blueprints/12-splash-quadrilingue-telemetria-edid-realtime-e-ciclo-vida-desconexao.md):** Tela de boas-vindas com QR Code dinâmico, leitura de EDID em tempo real e retorno gracioso ao desconectar.
* **[Blueprint 13: Captura Direta no Kernel Linux DRM/KMS e Dual-Engine Universal](docs/blueprints/13-captura-direta-drm-kms-e-dual-engine-universal.md):** Extração atômica PRIME DMA-BUF direto do scanout da GPU AMD/Intel/NVIDIA via ioctl GETFB2.
* **Blueprint 14: Subsistema de Áudio HDMI Digital, Codec Opus e ALSA](docs/blueprints/14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md):** Transmissão de áudio digital estéreo de ultra-baixa latência (< 25ms) via PipeWire e ALSA na porta UDP 5004 com sincronia A/V e controle Web.
* **[Blueprint 15: Pipeline Zero-Copy, Quiescência Wayland e Ciclo de Vida](docs/blueprints/15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md):** Preservação do zero-copy VA-API, retenção de framebuffer KMS no receptor e console silencioso.
* **[Blueprint 16: Pacer Wayland de Quiescência, Compilação Cross-ARMv6 e Receptores Universais](docs/blueprints/16-pacer-wayland-quiescencia-e-guia-universal-receptores.md):** Eliminação definitiva da dormência do GNOME Mutter a 60 FPS, compilação ARMv6, Raspberry Pis sem OTG e PC secundário.

---

## 🚀 Guia de Instalação e Deploy Multi-Dispositivo

### 1. Compilação para Raspberry Pi Zero (ARMv6)
O BCM2835 do Pi Zero v1.2/v1.3/W requer compilação específica para `armv6l`:
```bash
# Via cross (Docker recomendado):
cargo install cross --git https://github.com/cross-rs/cross
cd receiver && cross build --target arm-unknown-linux-musleabihf --release

# O binário gerado estará em: target/arm-unknown-linux-musleabihf/release/ext-receiver
```

### 2. Uso em Modelos Raspberry Pi Não-OTG (Pi 2, Pi 3, Pi 4, Pi 5)
Esses modelos possuem portas USB controladas por hub e não suportam modo gadget periférico OTG. A transmissão deve ocorrer via **Ethernet ou Wi-Fi**.

#### Correção dos Parâmetros de Boot no Cartão SD:
1. No arquivo `config.txt`, desative o overlay `dwc2`:
   ```ini
   # DESATIVAR DWC2 OTG EM MODELOS NÃO-OTG:
   # dtoverlay=dwc2
   dtoverlay=vc4-kms-v3d,cma-128
   dtparam=audio=on
   ```
2. No arquivo `cmdline.txt`, remova o `modules-load=dwc2`:
   ```text
   console=tty3 quiet loglevel=0 vt.global_cursor_default=0 ip=dhcp
   ```
3. Conecte apontando para o IP da rede local do Pi:
   ```bash
   ./scripts/start.sh extend auto 60 false full <IP_DO_PI>:5000
   ```

### 3. Transformar um PC / Laptop Convencional em Segunda Tela
Você pode usar qualquer notebook ou desktop com Linux (GNOME, KDE, XFCE, i3, Sway), Windows ou macOS como tela secundária sem Raspberry Pi:

#### Opção A: GStreamer (Pipeline leve padrão)
```bash
gst-launch-1.0 -v udpsrc port=5000 buffer-size=524288 \
    caps="application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96" ! \
    rtph264depay ! h264parse ! avdec_h264 ! autovideosink sync=false
```

#### Opção B: FFmpeg / ffplay (Universal, funciona em qualquer SO / ambiente sem GNOME)
```bash
ffplay -fflags nobuffer -flags low_delay -framedrop -strict experimental \
       -an -sn -sync ext -protocol_whitelist file,udp,rtp \
       -i rtp://0.0.0.0:5000
```
Com aceleração VA-API (Intel/AMD):
```bash
ffplay -vcodec h264_vaapi -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 \
       -fflags nobuffer -flags low_delay -framedrop -an -protocol_whitelist file,udp,rtp rtp://0.0.0.0:5000
```

#### Opção C: MPV Player
```bash
mpv --no-cache --untimed --no-correct-pts --fps=60 --profile=low-latency --hwdec=auto rtp://0.0.0.0:5000
```

### 4. Direcionar para a Tela 1 ou 2 em PCs com Múltiplos Monitores
Caso o PC receptor possua 2 ou mais telas conectadas e você queira projetar especificamente na **Tela 2**:

* **Em Modo Direto KMS DRM (`kmssink`):**
  ```bash
  # Identifique o connector-id via modetest:
  modetest -c | grep -E "id|name|status"
  # Execute direcionando ao conector desejado (ex: connector-id=45):
  gst-launch-1.0 udpsrc port=5000 ... ! kmssink connector-id=45 sync=false
  ```
* **Em Sessão Gráfica com `ffplay`:**
  ```bash
  # Posiciona a janela em tela cheia no segundo monitor (ex: x=1920):
  ffplay -left 1920 -top 0 -fs -fflags nobuffer -flags low_delay -protocol_whitelist file,udp,rtp rtp://0.0.0.0:5000
  ```
* **Com `mpv`:**
  ```bash
  mpv --fs --screen=1 --profile=low-latency rtp://0.0.0.0:5000
  ```

### 5. Modo USB Bulk Padrão com Fallback Automático
O `ext-sender` opera por padrão no modo **USB Bulk Direto** de alta velocidade (480 Mbps). Se o cabo estiver conectado a um dispositivo sem suporte USB gadget ou via rede (como um PC secundário ou Pi via Wi-Fi), o transmissor detecta a ausência e faz o **fallback automático para Rede UDP (porta 5000)** de forma transparente.

---

## 🛠️ Building the Appliance Image

To compile the entire system and build a bootable 32MB SD card image:
```bash
# Build complete universal appliance image (Pi Zero 1 & Zero 2 W):
./scripts/build-fast-appliance.sh

# Flash directly to micro-SD card (replace /dev/sdX with your card reader):
sudo dd if=build-appliance/ext-monitor-pi0-appliance.img of=/dev/sdX bs=4M status=progress conv=fsync
```


---

## 📄 License & Attribution

Distributed under the **MIT License**. See [LICENSE](LICENSE) for full details.

**Author:** Carlos Alberto ([psncarlosalberto4ti@gmail.com](mailto:psncarlosalberto4ti@gmail.com))
