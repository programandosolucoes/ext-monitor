# Blueprint 04: PipeWire Integration, GNOME Mutter Screencast D-Bus and Wayland Architecture

> [🇧🇷 Versão em Português](../pt/04-pipewire-mutter-screencast-e-wayland.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `sender/src/screencast.rs`, `sender/src/pipewire.rs`, `sender/src/pipeline.rs`, `sender/src/main.rs`  
**Date:** September 2026 (Updated with Modular Refactoring and RAII Drop Cleanup)  

---

## 1. Overview and Security Constraints

In modern Linux desktop environments running **Wayland** with the **GNOME Mutter** compositor, user-space applications are strictly sandboxed from accessing desktop pixel buffers directly. Unlike legacy X11, arbitrary screen scraping via shared memory is blocked by the protocol.

`ext-monitor` overcomes this architectural challenge through native integration with the GNOME Mutter D-Bus API (`org.gnome.Mutter.ScreenCast`) and the **PipeWire** multimedia graph server, allowing programmatic instantiation of **true virtual extended displays without needing physical HDMI dummy plugs**.

---

## 2. The D-Bus Handshake with GNOME Mutter

To dynamically instantiate and capture the virtual monitor, `ext-sender` executes an automated sequence over the D-Bus session bus:

```
[ ext-sender ]                          [ org.gnome.Mutter.ScreenCast ]
      │                                                │
      ├─── 1. CreateSession(properties) ──────────────>│
      │<─── Returns Session Object (/org/gnome/...) ───┤
      │                                                │
      ├─── 2. RecordVirtual(session, properties) ─────>│
      │       (Spawns virtual HDMI-1 or VIRTUAL)       │
      │<─── Returns Stream Object ─────────────────────┤
      │                                                │
      ├─── 3. OpenPipeWireRemote() ───────────────────>│
      │<─── Returns Unix File Descriptor (fd) ─────────┤
      │                                                │
      ├─── 4. Start() ────────────────────────────────>│
      │<─── Signal: PipeWire Node ID (e.g., 48) ───────┤
```

### 2.1 D-Bus Interface Specifications
* **Destination:** `org.gnome.Mutter.ScreenCast`
* **Object Path:** `/org/gnome/Mutter/ScreenCast`
* **Interface:** `org.gnome.Mutter.ScreenCast`
* **`CreateSession` Method:** Initializes a session handle for virtual display control.
* **`RecordVirtual` Method:** Requests Mutter to instantiate a virtual display output with user-specified resolution (e.g., `1280x720` or `1600x900` at 60Hz). GNOME adds this display to its multi-monitor layout, making it fully draggable and manageable in GNOME Display Settings.

---

## 3. Zero-Copy DMA-BUF Video Routing with PipeWire

Once `ext-sender` obtains the PipeWire remote file descriptor and numerical `node_id`, zero-copy video streaming is established:

```
[ Mutter Virtual Display ]
         │ (GPU Hardware Render)
         ▼
[ DMA-BUF Memory Pages ] <── Buffer mapped directly in GPU VRAM
         │
         ▼ (Pointer exchange with zero CPU copy)
[ PipeWire Stream Node ]
         │
         ▼ (pipewiresrc path=NODE_ID)
[ Hardware Encoder (VA-API / NVENC / QSV) ]
```

### 3.1 Benefits of DMA-BUF Zero-Copy
* Video memory is never copied to CPU user-space (zero `memcpy` operations).
* Hardware video encoders read the identical physical memory pages populated by the GPU compositor.
* Host CPU consumption remains negligible (< 2%).

---

## 4. Damage Tracking, Window Occlusion and Framerate Stabilization

### 4.1 Mutter Idle Damage Quiescence
GNOME Mutter suppresses buffer emissions when the virtual screen is completely static (no cursor movement, no redraws).
* To prevent encoder timeouts and keep downstream decoders locked in sync, `ext-sender` configures the PipeWire source with `keepalive-time` matching the frame deadline (e.g., 16ms for 60 FPS).
* PipeWire resends the latest valid buffer periodically during quiet intervals, maintaining continuous CFR transmission without extra CPU burn.

### 4.2 Browser Window Occlusion (Chrome / Firefox on Wayland)
A known browser optimization issue on Wayland causes video players (like YouTube) on secondary monitors to freeze when the mouse cursor leaves the browser window:
* **Root Cause:** Chrome's `CalculateNativeWinOcclusion` and Firefox's `media.suspend-bkgnd-video.enabled` identify unfocused windows without pointer hover as occluded, suspending `<video>` rendering.
* **Remedy:** Set `chrome://flags/#calculate-native-win-occlusion` to **Disabled** or set `media.suspend-bkgnd-video.enabled = false` in Firefox.

### 4.3 Transmission Profiles: Continuous (CFR) vs Economy
* **Continuous CFR (Default):** Transmits rock-solid 60 FPS video packets at all times, ensuring smooth fluid interactions.
* **Economy Mode (`--economy`):** Drops repeated frames when static, saving up to 95% bandwidth on static text and code editing.

---

## 5. Host Power Suspend/Resume Resilience (Dual Rust Watchdogs)

When a host laptop suspends (S3 suspend-to-RAM), GNOME Mutter tears down the ScreenCast D-Bus session and destroys the PipeWire source node. Traditional streaming processes stall indefinitely on orphaned sockets.

### 5.1 Dual Rust Watchdog Architecture
`ext-sender` continuously monitors pipeline health:
1. **Health Watchdog (1500ms):** Queries `pw-cli info <node_id>`. When the node vanishes during sleep, it forcibly terminates child encoder instances (`kill -9`) and breaks out of the loop.
2. **Link Watchdog (3000ms):** Verifies the PipeWire audio/video link, restoring graph bindings if the media graph restarts.
3. Upon system wake, `ext-sender` automatically renegotiates a fresh ScreenCast D-Bus session with GNOME Mutter and resumes streaming within 1.8 seconds—completely hands-free.
