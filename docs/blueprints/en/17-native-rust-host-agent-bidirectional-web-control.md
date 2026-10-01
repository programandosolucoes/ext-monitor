# Blueprint 17: Native Rust Host Daemon and Bidirectional Web Control

> [🇧🇷 Versão em Português](../pt/17-agente-host-rust-controle-web-bidirecional.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `sender/src/control.rs`, `sender/src/main.rs`, `receiver/src/web.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview: From Shell Scripts to Native Rust Daemon

Historically, starting display streaming on the host PC required manual CLI execution. This model suffered from:
1. **Asymmetric Control:** Web dashboard adjustments had no effect until the host command was manually restarted.
2. **Fragility of Shell Scripts:** Bash scripts suffered from race conditions, orphaned background processes, and lost Wayland session variables (`WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`).
3. **No Remote Appliance Control:** Users should never have to open terminal windows to pause displays or change resolutions.

The solution is the **native Rust daemon (`ext-sender`)** managed bidirectionally from the Pi Zero's Web Dashboard.

---

## 2. RPC / UDP 5001 Control Architecture

```
+------------------------------------+             +--------------------------------------+
|       Raspberry Pi Zero W          |             |             Host PC (Linux)          |
|                                    |             |                                      |
|  [ Web Browser ]                   |             |  [ ext-sender Daemon (Rust) ]        |
|          | (HTTP POST)             |             |          |                           |
|          v                         |             |          v                           |
|  [ ext-receiver Web Server 8080 ]  |   UDP 5001  |  [ ControlListener (UdpSocket) ]     |
|   - /api/host/control              | ----------> |   - Start / Stop Streaming           |
|   - /api/config                    |             |   - SetMode (extend / clone)         |
|   - /api/media/control             |             |   - SetFps (15 / 30 / 60)            |
+------------------------------------+             |   - SetBitrate (150k .. 15M)         |
                                                   |   - SetAudio (true / false)          |
                                                   +--------------------------------------+
```

### 2.1 JSON Control Protocol (Port 5001)
Messages are dispatched as lightweight UDP JSON datagrams to `192.168.7.1:5001`:
* `action`: `"start"`, `"stop"`, `"trigger_hud"`, `"hide_hud"`.
* `mode`: `"extend"` (virtual `HDMI-1`) vs `"clone"` (host `eDP-1`).
* `fps`: `15`, `30`, `60`.
* `bitrate`: `150` to `15000` kbps.
* `audio`: `true` or `false`.
* `color`: `"full"`, `"256"`, `"gray"`.

---

## 3. Resolving Mutter Stride Assert Crash (`SIGABRT 6`)

On Linux Wayland with AMD Radeon graphics (DCN 3.1):
* Requesting odd or unaligned virtual monitor dimensions caused GNOME Mutter to fail internal memory stride assertions:
  ```text
  gnome-shell[...]: ../src/backends/meta-screen-cast-virtual-stream-src.c:134: assertion failed: (stride >= width * 4)
  ```
  This immediately crashed GNOME Shell and terminated the user's active desktop session.
* **Resolution:** `ext-sender` strictly enforces 16-pixel pitch alignment on all dynamic resolution changes, guaranteeing safe, crash-proof virtual screen resizing.

---

## 4. User Service (`systemd --user`), Anti-Duplication (PID Lock) and Transport Hot-Switch

### 4.1 Decoupled Execution and Zero-Sudo Auto-Registration
To eliminate terminal dependencies and prevent accidental terminations via `Ctrl+C`:
1. **Transparent Auto-Registration**: When executed without arguments on a system where the service is not yet registered, the binary automatically writes `~/.config/systemd/user/ext-monitor-sender.service` and launches the daemon in the background.
2. **Single-Instance PID Lockfile**: Employs `$XDG_RUNTIME_DIR/ext-monitor-sender.pid`. If an active instance is running, further CLI invocations report the active PID and exit with status 0 without port collisions or GPU stream conflicts.
3. **Native Lifecycle Subcommands**:
   ```bash
   ext-sender start       # Starts background user service
   ext-sender stop        # Gracefully terminates active stream
   ext-sender status      # Displays active PID, FPS, and live telemetry
   ext-sender logs        # Streams real-time journalctl logs
   ext-sender install     # Installs unit file to user systemd
   ext-sender uninstall   # Removes unit file and stops service
   ```

### 4.2 Seamless Transport Hot-Switching (UDP <-> USB Bulk Direct)
When toggling between **Mode 1 (UDP Network)** and **Mode 3 (USB Bulk Direct)** from the web dashboard on port 8080:
* The receiver dispatches a UDP control datagram to port `5001` (`{"transport": "usb_bulk"}` or `{"transport": "network"}`).
* `ext-sender` dynamically comutes its output pipeline:
  - In **USB Bulk mode**: claims the 480 Mbps FunctionFS interface on the Pi Zero and routes frames to an asynchronous pipe writer connected to the Bulk OUT endpoint.
  - In **UDP Network mode**: releases the USB endpoint and comutes pipeline to `udpsink` targeted at `192.168.7.2:5000`.
* The hot-switch takes place in sub-second intervals without display starvation or black screens.
