# Blueprint 29: Root Cause Analysis of 1-Second Freeze (mDNS Loop Storm), 100% Pure Rust Damage Pacer, and Direct Kernel DRM/KMS Scanout

> [🇧🇷 Versão em Português](../pt/29-diagnostico-congelamento-1s-mdns-pacer-rust-e-scanout-kms.md) | 🇺🇸 English Version

*Date: 2026-10-02*  
*Status: Production Approved and Verified at Solid 60 FPS in Mode 1 (UDP) and Mode 3 (USB)*  
*Author: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Overview and Engineering Objectives

During real-world verification of **Ext-Monitor** running continuous video streaming across Network Mode (Mode 1: UDP RTP) and USB Bulk (Mode 3), three architectural requirements were enforced:
1. **Strict Language Rule:** Complete removal of external scripting interpreters (Python). The entire software stack must be **100% pure Rust in-process**.
2. **Elimination of the Periodic 1-Second Freeze:** A subtle symptom where playback was smooth for a period, then froze for exactly ~1 second at regular intervals, before immediately resuming smoothness.
3. **Compositor Independence (GNOME Mutter) via Kernel DRM/KMS Direct Scanout:** In-depth verification of direct scanout capabilities in the Linux kernel DRM subsystem without mandatory reliance on GNOME Mutter/PipeWire screencast portals.

This blueprint details the root cause analysis spanning network protocols, kernel socket buffers, and display subsystems, together with their definitive Rust solutions.

---

## 2. Anatomy and Resolution of the 1-Second Freeze

### 2.1 The Multicast Echo Storm Symptom
* **Observed Symptom:** While streaming over UDP (Mode 1), video played smoothly, but every fixed interval it stalled for ~1.0 second and resumed.
* **Kernel Telemetry on Hardware (Pi Zero):**
  Querying `/proc/net/snmp` on the Raspberry Pi Zero revealed kernel socket buffer saturation:
  ```
  Udp: InDatagrams NoPorts InErrors OutDatagrams RcvbufErrors SndbufErrors InCsumErrors
  Udp: 13360937    365137  11875296 23759925     11875296     0            0
  ```
  * `RcvbufErrors` had accumulated over **9.7 million dropped UDP packets**.
  * The drop rate exceeded **3,300 packets per second**.
  * The `ext-receiver` process consumed 83% to 100% of the Pi Zero's single ARM1176JZF-S core, driving CPU temperature up to 56.2 °C.

### 2.2 Root Cause: Infinite Echo Loop in mDNS Responder
Auditing [`receiver/src/mdns.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/mdns.rs) revealed:
1. **Missing `IP_MULTICAST_LOOP` Disablement:** The multicast UDP socket (`224.0.0.251:5353`) looped back its own transmitted multicast frames.
2. **Missing DNS QR Bit Check:** The responder did not check whether incoming packets were queries or responses (`(flags & 0x8000) == 0`). Any packet matching keywords (`display`, `miracast`, `pi-zero`) triggered a response packet sent back to `224.0.0.251:5353`.
3. **High-Frequency Feedback Loop:** The responder received its own response, treated it as a new query, and immediately transmitted another response, generating an **infinite multicast loop at bus line rate**.

### 2.3 Why Did the Freeze Last Exactly 1.0 Second?
The video encoder pipeline operates at:
$$\text{key-int-max} = 60 \quad \text{at} \quad 60\text{ FPS} \implies \text{GOP} = 1.0\text{ second}$$
* The mDNS packet storm exhausted the kernel's UDP receive buffer (`so_rcvbuf`).
* When H.264 video RTP packets were dropped by the kernel (`RcvbufErrors`), P-frame predictive slices were lost.
* The Broadcom VideoCore IV hardware decoder (`bcm2835-codec`) cannot decode predictive slices without valid reference pictures.
* **The hardware decoder stalled until the next self-contained IDR frame arrived**, which occurs exactly once every 1.0 second (60 frames)!

### 2.4 The Rust Fix
In [`receiver/src/mdns.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/mdns.rs):
1. Disabled multicast loopback:
   ```rust
   let loop_opt: libc::c_int = 0;
   libc::setsockopt(
       fd,
       libc::IPPROTO_IP,
       libc::IP_MULTICAST_LOOP,
       &loop_opt as *const _ as *const libc::c_void,
       std::mem::size_of::<libc::c_int>() as libc::socklen_t,
   );
   ```
2. Validated DNS QR bit (queries only):
   ```rust
   let flags = u16::from_be_bytes([buf[2], buf[3]]);
   let is_query = (flags & 0x8000) == 0;
   if !is_query {
       continue; // Ignore DNS responses to break the echo loop
   }
   ```
3. Sent responses via direct unicast (`socket.send_to(&response_packet, src)`) rather than broadcasting back to `224.0.0.251`.

### 2.5 Post-Fix Verification
* Pi Zero kernel `RcvbufErrors` delta: **0 drops**.
* Pi Zero CPU utilization: **Dropped from ~83% to 0.28% - 0.74%**.
* SoC Temperature: **Dropped from 56.2 °C to 48.2 °C**.
* Video Stream: **Zero 1-second freezes; perfectly fluid 60 FPS**.

---

## 3. 100% Pure Rust In-Process Wayland Damage Pacer

### 3.1 Eliminating External Scripting Dependencies
Background Python scripts (`wayland-damage-pacer.py`) introduced external interpreter dependencies and lacked deterministic lifecycle management.

### 3.2 Native Rust Implementation ([`sender/src/damage_pacer.rs`](file:///home/carlos/ide/ext-monitor/sender/src/damage_pacer.rs))
Implemented via direct dynamic FFI against `libX11.so.6` and `libXext.so.6`:
* **Unmanaged 1x1 Pixel Window:** Created with `CW_OVERRIDE_REDIRECT = 1` (`override_redirect = 1`), preventing GNOME Mutter from managing, decorating, or focusing the pacer.
* **100% Click-Through Transparency:** Configured with `XShapeCombineRectangles(SHAPE_INPUT, 0, 0, NULL, 0, SHAPE_SET, 0)`, eliminating any mouse or touch bounding box.
* **60 Hz Active Damage Commits:** Every 16.6 ms, the Rust background thread draws alternating pixel colors (`0x00000000` / `0x00010101`) using an allocated Graphics Context (`XCreateGC` + `XFillRectangle` + `XFlush`).
* This alternating commit forces Xwayland to submit genuine `wl_surface.commit` events to Mutter, keeping the PipeWire clock running at continuous 60 FPS even when the cursor is stationary or leaves the display.

---

## 4. Direct Kernel DRM/KMS Scanout vs GNOME Mutter

### 4.1 DRM Master Privileges
To eliminate reliance on GNOME Mutter/PipeWire, direct DRM/KMS scanout capture was investigated using [`src/bin/ext_kms_probe.rs`](file:///home/carlos/ide/ext-monitor/sender/src/bin/ext_kms_probe.rs):
1. In active Wayland sessions, Mutter acquires `DRM_MASTER` on `/dev/dri/cardX`.
2. Unprivileged processes attempting `DRM_IOCTL_MODE_GETFB2` receive permission errors or `None`.

### 4.2 Proof of Concept: PRIME DMA-BUF Export with `CAP_SYS_ADMIN`
Assigning `CAP_SYS_ADMIN` to the binary (`setcap cap_sys_admin+ep /usr/local/bin/ext-sender`) enables direct hardware scanout export:
```
[INFO] Device /dev/dri/card1: Driver 'amdgpu'
[INFO] Found CRTC 368 with Framebuffer ID 416
[INFO] FB 416: 1280x720, format=AR24, pitches=[5120], offsets=[0]
[SUCCESS] buffer_to_prime_fd exported: OwnedFd { fd: 4 } directly from CRTC scanout!
```
* The GPU framebuffer is exported directly as a PRIME DMA-BUF file descriptor with zero CPU copy overhead, enabling true kernel-level scanout capture independent of desktop compositors.

---

## 5. Engineering Directives

1. **Multicast Sockets on Embedded Hardware:** Always explicitly disable `IP_MULTICAST_LOOP` and validate query/response flags before replying. Multicast echo loops exhaust kernel buffers and silently destroy real-time media streams.
2. **Wayland Damage Pacing:** Implement in-process in Rust using X11 `override_redirect` and empty shape masks to ensure continuous frame clocking without user input interception.
3. **Direct DRM/KMS Architecture:** With Linux `cap_sys_admin+ep`, applications can export scanout framebuffers as PRIME DMA-BUFs directly from the kernel display driver.
