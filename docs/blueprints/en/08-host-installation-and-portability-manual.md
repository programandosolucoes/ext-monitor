# Blueprint 08: Host Installation Manual, Cross-Distro Portability and Host Setup

> [🇧🇷 Versão em Português](../pt/08-manual-de-instalacao-e-portabilidade-host.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `scripts/connect.sh`, `scripts/install-host.sh`, `scripts/99-ext-monitor.rules`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Portability Philosophy

This blueprint details the deployment architecture of `ext-monitor`, engineered so that any user can connect the Raspberry Pi Zero to a new PC (Linux or Windows) and immediately establish a zero-latency secondary screen without manually building code from source.

---

## 2. Instant Portable 1-Click Connection (`curl`)

When the Pi Zero is plugged into a Linux workstation via the USB data port, the CDC-ECM network interface initializes automatically and the host receives IP `192.168.7.1` from the appliance's embedded DHCP daemon.

Users can launch the display with a single terminal command:

```bash
curl -sSL http://192.168.7.2:8080/connect.sh | bash
```

### Automated Flow of `connect.sh`:
1. Pings `192.168.7.2` to verify correct physical micro-USB port connection.
2. Creates the portable cache directory `~/.local/share/ext-monitor/`.
3. Downloads the client package (`client.tar.gz`) from the Pi Zero's embedded web server.
4. Extracts the native `ext-sender` binary.
5. Launches the display extension at fluid 60 FPS CFR with active dual-watchdog sleep recovery.

---

## 3. Permanent System Host Installation (`scripts/install-host.sh`)

For users seeking permanent desktop integration (application menu shortcuts, udev rules, KMS capabilities):

```bash
./scripts/install-host.sh
```

### 3.1 Automated Multi-Distro Dependency Resolution
Installs required graphics drivers and media tools across major Linux distributions:
* **Debian / Ubuntu / Pop!_OS / Linux Mint:**
  ```bash
  sudo apt-get install -y gstreamer1.0-tools pipewire pipewire-bin ffmpeg libcap2-bin vainfo mesa-va-drivers
  ```
* **Fedora / RHEL:**
  ```bash
  sudo dnf install -y gstreamer1-tools pipewire pipewire-utils ffmpeg libcap mesa-va-drivers libva-utils
  ```
* **Arch Linux / Manjaro:**
  ```bash
  sudo pacman -S --noconfirm gstreamer pipewire pipewire-media-session ffmpeg libva-mesa-driver libva-utils
  ```

### 3.2 Security and KMS Capabilities (`cap_sys_admin`)
To enable direct kernel scanout reading via DRM/KMS without needing root or sudo privileges:
```bash
sudo setcap cap_sys_admin+ep /usr/local/bin/ext-sender
sudo usermod -a -G video,render $USER
```

### 3.3 Low-Latency Udev Rules (`/etc/udev/rules.d/99-ext-monitor.rules`)
```udev
ACTION=="add|change", SUBSYSTEM=="net", KERNEL=="enx*|usb*", ATTRS{idVendor}=="1d50", ATTRS{idProduct}=="614d", \
    RUN+="/sbin/ip link set dev %k txqueuelen 100", RUN+="/sbin/ip link set dev %k mtu 1500"
ACTION=="add|change", SUBSYSTEM=="net", KERNEL=="enx*|usb*", ATTRS{idVendor}=="1d6b", ATTRS{idProduct}=="0104", \
    RUN+="/sbin/ip link set dev %k txqueuelen 100", RUN+="/sbin/ip link set dev %k mtu 1500"
```
* **`txqueuelen 100`:** Reduces interface queue depth from 1000 to 100 packets, preventing bufferbloat and cutting cursor latency by >15ms.

---

## 4. Driverless Windows 10/11 Miracast (`Win + K`)

On Windows, **zero drivers or executables are required**:
1. Press keyboard shortcut **`Win + K`**.
2. Select **"ExtMonitor-Pi0"**.
3. Windows connects instantly over native Wi-Fi Display (WFD RTSP TCP 7236) and projects the desktop to HDMI.
