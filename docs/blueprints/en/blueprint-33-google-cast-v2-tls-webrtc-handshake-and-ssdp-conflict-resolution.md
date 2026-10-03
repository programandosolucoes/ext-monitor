# Blueprint 33: Strict Chromium Cast TLS Certificate Validation, WebRTC Mirroring Handshake, and SSDP/DLNA Conflict Resolution

## 1. Overview and Motivation
Following the introduction of the Chromecast-style receiver feature (`RaspCast`) listening on TLS port 8009 with mDNS announcement `_googlecast._tcp.local`, real-world testing from Google Chrome on the laptop uncovered two critical integration and user-experience issues:

1. **Duplicate / Incompatible Device in Chrome's Cast Menu:** Chrome displayed two entries simultaneously:
   - `RaspCast` (Compatible, working over Google Cast V2);
   - `Ext-Monitor TV & Sound (192.168.7.2)` (Incompatible, labeled by Chrome as *"Available for specific sites"*).
2. **Gray / Blank Screen & Area Selection Window Behavior:**
   - After initiating Cast from Chrome, the TV/Pi Zero screen displayed only a blank gray buffer.
   - The user expected an OS screen or window selection picker, but Chrome started casting directly without opening a picker dialog.

This Blueprint documents the reverse engineering of Chromium source code, the implementation of the WebRTC Mirroring handshake, the logical display activation bugfix in GNOME Mutter, and the resolution of SSDP multicast conflicts.

---

## 2. Architecture and Technical Engineering

### 2.1 Strict Chromium Cast Certificate Security Requirements (`cast_auth_util.cc` & `cast_cert_validator.cc`)
Reverse-engineering Chromium's Cast authentication source revealed strict cryptographic requirements for authenticated Cast connections:
1. **Maximum Validity of 4 Days:** Chromium strictly enforces `kMaxSelfSignedCertLifetimeInDays = 4`. Self-signed certificates with longer expiration (e.g., 365 days) are rejected outright. The certificate generator in [`sender/src/cast_cert.rs`](file:///home/carlos/ide/ext-monitor/sender/src/cast_cert.rs) was adjusted to `-days 3`.
2. **Mandatory Critical X.509 Extensions:**
   - `basicConstraints = critical, CA:FALSE`
   - `keyUsage = critical, digitalSignature, keyEncipherment`
   - `extendedKeyUsage = clientAuth, serverAuth`
3. **Public Key Match Verification Under Mutex (`CERT_MUTEX`):** The ephemeral certificate and RSA private key must share identical public components (`cert_pkey.public_eq(&rsa_pkey)`), guarded under mutex to avoid test race conditions.
4. **Signature Algorithm:** Device authentication challenge response requires an `RSASSA_PKCS1v15` signature with SHA-256 (`sig_algo = 1`).

### 2.2 Eliminating Incompatible Duplicate Devices (SSDP vs Cast V2 Conflict)
The Raspberry Pi Zero includes a UPnP/DLNA `MediaRenderer` daemon for direct playback via local media players. The SSDP responder in [`receiver/src/media_renderer.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/media_renderer.rs) was broadcasting periodic `NOTIFY` packets with `ST: urn:schemas-upnp-org:device:MediaRenderer:1` and friendly name `Ext-Monitor TV & Sound`.

Because Google Chrome listens on UDP port 1900, it parsed this UPnP descriptor and displayed it as a restricted media sink (*"Available for specific sites"*).

**Implemented Solution:**
1. In `receiver/src/media_renderer.rs`, suppressed periodic `NOTIFY` packets for `MediaRenderer` and `upnp:rootdevice` over `usb0`, keeping only DIAL announcements (`urn:dial-multiscreen-org:device:dial:1`).
2. Routed `M-SEARCH` queries with `ST: ssdp:all` directly to the DIAL descriptor (`/dial/dd.xml`, `RaspCast`).
3. Renamed the internal UPnP descriptor to `RaspCast DLNA (192.168.7.2)` for dedicated DLNA players querying `MediaRenderer` explicitly.
4. In the host SSDP bridge ([`sender/src/discovery.rs`](file:///home/carlos/ide/ext-monitor/sender/src/discovery.rs)), filtered out generic requests and responded solely to DIAL queries.
5. **Result:** Chrome now exclusively displays the native **`RaspCast`**.

### 2.3 WebRTC Mirroring Handshake Protocol (`urn:x-cast:com.google.cast.webrtc`)
When starting screen or tab mirroring, Chrome initiates a WebRTC negotiation offer:
```json
{"offer":{"castMode":"mirroring","supportedStreams":[{"codecName":"opus","index":0,"ssrc":26593,...},{"codecName":"vp9","index":1,"ssrc":91897,...}]},"seqNum":1,"type":"OFFER"}
```
Implemented parsing and synchronous `ANSWER` response generation in [`sender/src/cast_server.rs`](file:///home/carlos/ide/ext-monitor/sender/src/cast_server.rs):
```rust
let answer_json = serde_json::json!({
    "type": "ANSWER",
    "seqNum": seq_num,
    "result": "ok",
    "answer": {
        "castMode": "mirroring",
        "receiverGetStatus": true,
        "sendIndexes": send_indexes,
        "ssrcs": ssrcs,
        "udpPort": 5000
    }
}).to_string();
```
Also added support for `GET_STATUS` requests in the WebRTC namespace.

### 2.4 Resolving the Gray Screen: Mutter Logical Monitor False Positive
The blank/gray screen on the TV was caused by a false positive in GNOME Wayland monitor detection:
1. In `sender/src/pipewire.rs`, `ensure_gnome_displays()` checked if the extended layout was active using `stdout.contains("('HDMI-1'")`.
2. **Root Cause:** Mutter's `GetCurrentState` returns both physical connectors (`(('HDMI-1', 'LRX'...)`) and active logical monitor configurations (`[(0, 0, 1.0, 0, true, [('eDP-1'...)]), (1920, 0, 1.0, 0, false, [('HDMI-1'...)]]`).
3. Because `HDMI-1` was physically connected via kernel EDID override, the check falsely assumed `HDMI-1` was already enabled in Mutter's logical layout, skipping `ApplyMonitorsConfig`.
4. When `MutterScreenCastSession::create_and_start("HDMI-1")` was called, Mutter rejected the request:
   ```
   RecordMonitor('HDMI-1') failed: Failed to record monitor: Monitor not active. Fallback: RecordVirtual...
   ```
5. The fallback `RecordVirtual` created an empty virtual monitor without assigned windows or desktop surfaces, streaming a blank gray buffer.
6. **Fix:** Changed the detection to `stdout.contains("[('HDMI-1'")`. The `[('` prefix is unique to the active logical monitor array in Mutter. This ensures `ApplyMonitorsConfig` runs deterministically, properly enabling `HDMI-1` before screen capture.

### 2.5 Chrome Cast Screen vs Tab Selection Behavior
- **Default Mode ("Cast tab"):** Captures the current browser tab directly without prompting the user with an OS window picker dialog.
- **Screen/Window Mode ("Sources ➔ Cast screen"):** Triggers the GNOME / Wayland XDG Desktop Portal window picker, prompting the user to select either the entire screen (`eDP-1` or `HDMI-1`) or a specific application window.

### 2.6 Interactive User Decision with 4 Operating Modes
To ensure the user maintains complete control with all sharing modes available:
1. **Native Wayland GUI Selection (`zenity --list --radiolist`):**
   - Automatically prompted on the laptop screen upon connection:
     - 🖥️ `extend`: **Extend Desktop** (Enables virtual secondary screen `HDMI-1` on TV).
     - 💻 `clone`: **Mirror Entire PC Screen** (Clones primary laptop display `eDP-1` on TV).
     - 🪟 `window`: **Cast Specific Application Window** (Opens Web Caster with GNOME Portal window selector).
     - 🌐 `tab`: **Cast Browser Tab** (Opens Web Caster `/cast` in Chrome with audio & WebCodecs).
     - ❌ **Cancel / Close / Timeout (45s)**: Aborts transmission without modifying GNOME monitor topology.
2. **Web Dashboard Control (`http://192.168.7.2:8080`):**
   - 3-button selector under *Monitor Display Mode*:
     - `❓ Always Ask (Dialog)` (`mode: 'ask'`)
     - `🖥️ Extended (HDMI-1 TV)` (`mode: 'extend'`)
     - `💻 Cloned (eDP-1 Notebook)` (`mode: 'clone'`)
3. **Command Line Support:**
   - Supports `--mode=ask`, `--mode=extend`, and `--mode=clone`.

### 2.7 Fixing GNOME Shell Dock Focus Freezes (Damage Pacer)
- **Root Cause Analysis:** The damage pacer background thread was creating an X11 window on Xwayland at `(3198, 718)` pulsing `clear_area` every 16ms even when in Standby with `HDMI-1` collapsed. Because this position was outside the visible geometry of `eDP-1`, the GNOME Shell window tracker (`ShellWindowTracker` / Mutter) lost window focus state, causing clicks on open dock applications (such as Antigravity) to spawn new instances instead of focusing the existing window.
- **Solution:** Because default capture uses Kernel DRM/KMS scanout DMA-BUF, X11 damage events are completely unnecessary. We made `damage_pacer` strictly opt-in (`--enable-damage-pacer` under Mutter). The rogue `0x1000001` window is eliminated, restoring dock application switching immediately.

### 2.8 Zero Frozen Frame on TV Upon Stream Termination
- **Root Cause Analysis:** When stopping transmission (in Chrome or via STOP), the host killed GStreamer but did not notify the receiver. The receiver's UDP ingress watchdog was set to 15 seconds, leaving the last decoded frame frozen on the TV during that interval.
- **Dual Solution:**
  1. **Direct HTTP Stop Notification:** On `StopStreaming` or Cast `STOP`, `ext-sender` immediately issues `POST http://192.168.7.2:8080/api/stream/stop`.
  2. **Reduced Ingress Watchdog:** In `receiver/src/ingress/udp.rs`, inactivity timeout reduced from 15s to **1.5s**.
  3. Upon stopping transmission, the TV returns to the standby Ready Splash screen (< 100ms), completely eliminating frozen screen states.

---

## 3. Validation and Results

1. **Unit Test Suite:**
   - `ext-sender`: 77 passing tests (`cargo test -p ext-sender`).
   - `ext-receiver`: 72 passing tests (`cargo test -p ext-receiver`).
2. **Dual-Target Deployment:**
   - Release binary compiled and installed to `/usr/local/bin/ext-sender`.
   - `ext-monitor-sender.service` restarted and active.
   - `ext-receiver` cross-compiled for ARMv6 musl, flashed to Pi Zero boot partition via OTA and deployed live in RAM.
3. **Functional Verification:**
   - Chrome Cast menu shows only `RaspCast`.
   - WebRTC negotiation completes immediately with extended desktop rendering clearly on TV.
