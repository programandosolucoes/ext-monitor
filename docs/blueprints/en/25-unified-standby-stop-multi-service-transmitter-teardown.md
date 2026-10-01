# Blueprint 25: Unified Multi-Service Standby Stop, Miracast RTSP Teardown, and Host Transmitter Termination

## 1. Overview and Problem Context
When streaming in Mode 2 (Miracast) or other modes, pressing **"⏹ Disable Extension / Standby"** in the Web Dashboard failed to completely stop the video transmission.

### Identified Root Causes:
1. **Active Host Transmitters:** `gnome-network-displays` (or external GStreamer streamer pipelines) remained alive on the laptop, continuing their capture and VAAPI hardware encode loops, pouring UDP packets into port 5002.
2. **Orphaned RTSP Sessions on Pi Zero:** While `pipeline_mgr.pause()` stopped the internal decoder, `wfd.rs` did not track active client TCP/RTSP sockets. As a result, the Wi-Fi Display (WFD) session remained open indefinitely.
3. **Worker Exit Splash Inconsistency:** Upon terminating the Miracast ingress worker, the code rendered the Miracast connection guide (`SplashEngine::show_miracast()`) rather than the Standby/Ready splash screen (`SplashEngine::show_ready()`), confusing user telemetry.

---

## 2. Core Requirements
1. **Host-Side Transmitter Termination:**
   - The laptop `ext-sender` daemon must terminate any active `gnome-network-displays` instances via POSIX signals (`SIGTERM` and `SIGKILL`).
   - Internal child pipelines (Mode 1 UDP and Mode 3 USB) as well as external GStreamer instances targeting the Pi Zero must be killed immediately.
2. **Immediate RTSP Session Teardown on Receiver:**
   - Module `wfd.rs` must track connected client TCP streams and force immediate disconnection via `shutdown(Shutdown::Both)`.
   - Socket closure delivers an immediate EOF to the client (Windows `Win + K` or GNOME Displays), triggering clean remote disconnection.
3. **Preserve Physical HDMI Connection (No Display Disconnect):**
   - The HDMI framebuffer `/dev/fb0` and DRM KMS connector guard (`ensure_drm_hdmi_connected`) must remain **100% active**. The TV/monitor must never go black or lose the digital HDMI signal.
   - The framebuffer must display the Ready / Standby splash screen with network info and QR code.
4. **Comprehensive Multi-Service Stop (All 3 Modes):**
   - Mode 1 (Network UDP port 5000): V4L2 M2M decoder paused.
   - Mode 2 (Miracast RTSP 7236 / UDP 5002): RTSP sessions torn down, decoder paused.
   - Mode 3 (USB Bulk Direct): FunctionFS endpoint detached, decoder paused.
   - The Pi Zero supervisor loop remains paused (`is_paused == true`), preventing unintended pipeline restoration.

---

## 3. Architecture & Control Flow

```
[ User clicks "⏹ Disable Extension / Standby" ]
                        │
                        ▼
       ┌─────────────────────────────────┐
       │ Web UI: setExtensionAction('stop)│
       └────────────────┬────────────────┘
                        │
       ┌────────────────┴────────────────────────┐
       ▼                                         ▼
POST /api/stream/stop                     POST /api/host/control
(Ext-Receiver on Pi Zero)                  (Ext-Sender on Host Laptop UDP 5001)
       │                                         │
       ├─► CONFIG.mode1/2/3 = false              ├─► stop_gnome_network_displays() (pkill)
       ├─► pipeline_mgr.pause()                  ├─► child.kill() & child.wait()
       ├─► wfd::terminate_active_sessions()      ├─► pkill gst-launch-1.0.*192.168.7.2
       ├─► Frame cache cleared                   ├─► close_usb_transport()
       ├─► SplashEngine::show_ready()            └─► is_paused = true
       └─► HDMI Signal Active on /dev/fb0
```

---

## 4. Verification and Live Testing
1. **Automated Unit Tests:** 34 tests across sender and receiver passing with 100% success (`cargo test`).
2. **OTA Firmware Deployment:** `initramfs.cpio.gz` flashed to SD card and validated with SHA256 (`952e5b460a0fa7b5adf1deae9a96b4afefd31b0af45a98b07e18dee35249c7a0`).
3. **Standby Verification:**
   - `POST /api/stream/stop` returns `{"status":"paused"}`.
   - Host logs confirm: `[miracast-launcher] Finalizando instâncias de gnome-network-displays...` and `[*] Todos os transmissores de vídeo do Host foram finalizados. Modo Standby ativo.`
   - Screen capture confirms pristine Standby splash rendered on TV without HDMI interruption.
4. **Stream Resumption:** Re-enabling Mode 1 restored low-latency (<15ms) 60 FPS VAAPI streaming instantly.
