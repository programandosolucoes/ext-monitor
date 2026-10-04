# Frontend Architecture and JSDoc Technical Reference
**Document Reference:** `EXT-MON-JSDOC-2026-REV1`  
**Classification:** Complete Web Dashboard JSDoc Specification & Audit Guide  
**File Origin:** `receiver/src/web_ui.rs` (`pub const DASHBOARD_HTML`)  
**Target Standard:** ECMAScript 2020 (ES11) / JSDoc 3.6+ / W3C Web APIs  

---

## 1. Architectural Overview and Design Principles

The Pi Zero Extended Monitor Web Dashboard is embedded directly inside the Rust compiled binary (`ext-receiver`) as a static string literal (`pub const DASHBOARD_HTML: &str`). It provides an appliance management console accessible on HTTP port `8080`.

### Core Engineering Directives:
1. **Zero External Dependencies:** No external CDNs, bundlers, NPM packages, or frameworks (React, Vue, jQuery). Runs 100% locally from the Pi Zero's RAM.
2. **Server-Authoritative Telemetry:** All hardware states (CPU temperature, ALSA sample rate, DRM/KMS scanout state, display EDID) are authoritative on the Linux kernel backend. The UI reconciles via `/api/status` and `/api/config`.
3. **High-Performance Audio Canvas:** Employs a dedicated HTML5 `<canvas>` element rendering 24-bin FFT spectrum telemetry and VU meter levels at 30 FPS (`33.3 ms` tick) using normalized sub-pixel path blitting.
4. **Auditable & Deterministic:** Every event handler and REST API interaction has strict parameter types, defensive payload parsing, and clear error boundaries.

---

## 2. JSDoc Data Models and Type Definitions

```javascript
/**
 * System hardware telemetry returned by the receiver backend.
 * @typedef {Object} SystemTelemetry
 * @property {number} soc_temp - Broadcom BCM2835 silicon temperature in degrees Celsius.
 * @property {number} cpu_load - Normalized CPU utilization percentage (0.0 to 100.0).
 * @property {number} ram_free_mb - Free physical SDRAM in megabytes (out of 512 MB).
 * @property {number} ram_total_mb - Total physical SDRAM in megabytes.
 * @property {string} stream_state - Current decoder pipeline state ('active', 'paused', 'idle', 'standby').
 * @property {number} active_fps - Measured frame delivery rate to DRM/KMS scanout plane.
 * @property {number} bitrate_kbps - Real-time network/USB ingress bitrate in kbps.
 * @property {string} active_transport - Physical transport mode ('mode1_udp', 'mode2_wfd', 'mode3_usb').
 * @property {string} display_topology - Current desktop arrangement ('extend', 'clone', 'standby').
 */

/**
 * Display connector and EDID metadata.
 * @typedef {Object} DisplayInfo
 * @property {string} connector - Linux DRM connector identifier (e.g., 'HDMI-A-1').
 * @property {boolean} connected - Physical HDMI cable connection status.
 * @property {string} model - Parsed monitor or television manufacturer and model name.
 * @property {string} resolution - Native or current display mode (e.g., '1280x720@60Hz').
 * @property {number} preferred_refresh - Preferred vertical refresh rate in Hertz.
 * @property {Array<number>} supported_audio_rates - Audio sample rates parsed from EDID Short Audio Descriptors (SAD).
 */

/**
 * Hardware audio core state and ALSA MAI controller status.
 * @typedef {Object} AudioStatus
 * @property {number} volume - Hardware attenuation volume percentage (0 to 100).
 * @property {boolean} muted - Audio output mute state.
 * @property {number} sample_rate - Current ALSA hardware clock frequency (48000, 96000, 192000).
 * @property {string} transport - Active audio transmission protocol ('udp', 'uac2', 'bulk').
 * @property {string} format - Subframe encapsulation format ('IEC958_SUBFRAME_LE').
 */

/**
 * Host streaming encoder configuration and pipeline parameters.
 * @typedef {Object} EncoderConfig
 * @property {number} bitrate - Target variable bitrate in kilobits per second (400 to 6000).
 * @property {number} fps - Target framerate (15, 30, 60).
 * @property {string} color - Chroma profile ('full' for 24-bit TrueColor, '256' for quantized, 'gray').
 * @property {boolean} drop_only - Economy static frame dropping mode.
 * @property {boolean} skip_to_first - Low-latency queue bypass on initial cursor motion.
 * @property {number} key_int_max - Maximum distance in frames between full IDR I-Frames.
 * @property {string} capture_backend - Host screen capture engine ('kms' or 'mutter').
 * @property {string} scale - Receiver VPU scaling mode ('720p', '1600x900', 'off').
 * @property {boolean} cas - Contrast Adaptive Sharpening filter toggle.
 */

/**
 * Network interface configuration payload.
 * @typedef {Object} NetworkConfig
 * @property {string} interface - Selected physical network interface ('eth0', 'usb0', 'wlan0').
 * @property {string} mode - Addressing protocol ('dhcp' or 'static').
 * @property {string} [ip] - Static IPv4 address string (e.g., '192.168.1.50').
 * @property {string} [netmask] - Static IPv4 subnet mask (e.g., '255.255.255.0').
 * @property {string} [gateway] - Default gateway IPv4 address.
 * @property {string} [dns] - Primary DNS resolver IPv4 address.
 */

/**
 * Host control RPC payload dispatched over UDP port 5001.
 * @typedef {Object} HostControlPayload
 * @property {string} action - Target action ('start_extension' | 'stop_extension' | 'pause' | 'resume' | 'toggle_hud').
 * @property {string} [mode] - Extension arrangement ('extend' | 'clone' | 'standby').
 * @property {string} [transport] - Physical path ('mode1_udp' | 'mode2_wfd' | 'mode3_usb').
 * @property {number} [audio_rate] - Desired audio sample rate in Hertz (48000 | 96000 | 192000).
 * @property {boolean} [audio] - Audio stream enablement flag.
 * @property {number} [bitrate] - Target stream bitrate in kbps.
 * @property {number} [fps] - Target stream framerate.
 */
```

---

## 3. Exhaustive JSDoc Function Reference

### 3.1. Navigation and Internationalization

#### `switchTab(tabId)`
Switches the currently visible tab in the dashboard navigation container.
- **Parameters:**
  - `{string} tabId` - Tab identifier (`'monitor'`, `'config'`, `'downloads'`, `'sdcard'`, `'manual'`).
- **DOM Mutated:** Toggles `.active` class on `.tab-btn` and `.tab-content` containers.
- **Returns:** `{void}`

#### `t(key)`
Translates an internationalization string key into the user's selected language, falling back to English if the key is missing.
- **Parameters:**
  - `{string} key` - Key name in dictionary (e.g., `'statTemp'`, `'btnApply'`).
- **Returns:** `{string}` - Localized text content.

#### `setLanguage(lang)`
Updates the active UI language across the entire DOM tree and caches the selection.
- **Parameters:**
  - `{string} lang` - Two-letter language code (`'en'`, `'pt'`, `'it'`, `'zh'`).
- **Storage:** Writes `ext_monitor_lang` to `localStorage`.
- **Side Effects:** Re-evaluates all elements marked with `[data-i18n]` and triggers dynamic telemetry label refreshes.
- **Returns:** `{void}`

---

### 3.2. Stream Tuning and Video Pipeline Controls

#### `updateBitrateValue(val)`
Synchronizes the bitrate slider UI and numeric indicator.
- **Parameters:**
  - `{number|string} val` - Bitrate in kilobits per second (e.g., 4000).
- **Storage:** Writes `ext_bitrate` to `localStorage`.
- **Returns:** `{void}`

#### `setBitrate(kbps)`
Sets the active streaming bitrate and updates slider element position.
- **Parameters:**
  - `{number|string} kbps` - Bitrate in kbps.
- **Returns:** `{void}`

#### `setFps(fps)`
Sets the target display framerate.
- **Parameters:**
  - `{number} fps` - Framerate integer (`15`, `30`, `60`).
- **Storage:** Writes `ext_fps` to `localStorage`.
- **Returns:** `{void}`

#### `setColor(profile)`
Configures the video encoding chroma and quantizer profile.
- **Parameters:**
  - `{string} profile` - Color mode (`'full'`, `'256'`, `'gray'`).
- **Storage:** Writes `ext_color` to `localStorage`.
- **Returns:** `{void}`

#### `setDropOnly(val)`
Toggles the economy static-frame dropping transmission mode.
- **Parameters:**
  - `{boolean} val` - `true` to drop static frames, `false` for continuous 60 FPS video stream.
- **Storage:** Writes `ext_drop_only` to `localStorage`.
- **Returns:** `{void}`

#### `setSkipToFirst(val)`
Toggles low-latency queue bypass on initial mouse movement.
- **Parameters:**
  - `{boolean} val` - `true` to deliver first motion frame immediately.
- **Storage:** Writes `ext_skip_to_first` to `localStorage`.
- **Returns:** `{void}`

#### `updateKeyIntValue(val)`
Updates the periodic IDR keyframe interval and calculates the period in seconds.
- **Parameters:**
  - `{number|string} val` - Keyframe distance in frames.
- **Storage:** Writes `ext_key_int_max` to `localStorage`.
- **Returns:** `{void}`

#### `setKeyInt(val)`
Sets the periodic IDR keyframe interval.
- **Parameters:**
  - `{number|string} val` - Keyframe distance in frames.
- **Returns:** `{void}`

---

### 3.3. Audio Subsystem and Hardware Rate Controls

#### `setAudioRate(rate)`
Changes the ALSA hardware sample rate clock on the receiver and notifies the host streamer via UDP 5001.
- **Parameters:**
  - `{number} rate` - Target sample frequency (`48000`, `96000`, `192000`).
- **HTTP Call:** `POST /api/audio/rate` with `{ "rate": rate }`.
- **Telemetry Action:** Dispatches UDP 5001 control packet to host with `{ "audio_rate": rate }`.
- **Side Effects:** Re-initializes ALSA MAI PCM driver with standard IEC 60958 channel status frames.
- **Returns:** `{void}`

#### `updateAudioRateUI(rate)`
Updates the visual active state of sample rate selection buttons in the Audio DAC panel.
- **Parameters:**
  - `{number} rate` - Active sample frequency in Hertz.
- **DOM Mutated:** Updates `#btnAudioRate48k`, `#btnAudioRate96k`, `#btnAudioRate192k`.
- **Returns:** `{void}`

#### `setAudioTransport(transport)`
Selects the physical transmission path for digital audio.
- **Parameters:**
  - `{string} transport` - Transport identifier (`'udp'`, `'uac2'`, `'bulk'`).
- **HTTP Call:** `POST /api/audio/transport` with `{ "transport": transport }`.
- **Returns:** `{void}`

#### `updateAudioVolume(val)`
Adjusts the hardware ALSA PCM playback attenuation.
- **Parameters:**
  - `{number|string} val` - Volume level (0 to 100).
- **HTTP Call:** `POST /api/audio/volume` with `{ "volume": parseInt(val, 10) }`.
- **Returns:** `{void}`

#### `toggleAudioMute()`
Toggles ALSA hardware audio mute state.
- **HTTP Call:** `POST /api/audio/mute` with `{ "muted": boolean }`.
- **Returns:** `{void}`

#### `initAudioVisualizerCanvas()`
Initializes the HTML5 `<canvas id="canvasAudioVis">` 30 FPS rendering loop for real-time 24-bin FFT spectrum and RMS VU meter telemetry.
- **Tick Rate:** `33.3 ms` (30 frames per second).
- **DOM Mutated:** Redraws 2D canvas context `#canvasAudioVis`.
- **Returns:** `{void}`

---

### 3.4. Display Extension, Topology, and Transport Switching

#### `activateModeWithTopology(transport, topology)`
Primary entrypoint for single-click switching of transport mode and display arrangement.
- **Parameters:**
  - `{string} transport` - Physical transport (`'mode1_udp'`, `'mode2_wfd'`, `'mode3_usb'`).
  - `{string} topology` - Extension topology (`'extend'`, `'clone'`, `'standby'`).
- **HTTP Call:** `POST /api/displays/action` with `{ "action": action, "transport": transport, "mode": topology }`.
- **Side Effects:** Automatically ensures audio capture synchronization if `ext_simultaneous_audio` is enabled.
- **Returns:** `{void}`

#### `setExtensionAction(action)`
Directly sets the display extension action (`extend`, `clone`, `stop`).
- **Parameters:**
  - `{string} action` - Extension action.
- **Returns:** `{void}`

#### `updateModeAndTopologyButtons()`
Synchronizes all active button borders, glowing badges, and status labels based on current transport, topology, and streaming state.
- **DOM Mutated:** Updates `#extBadgeStatus`, `#lblActiveTransport`, and transport card outlines.
- **Returns:** `{void}`

---

### 3.5. System Operations and Firmware Maintenance

#### `confirmReboot()` / `closeRebootModal()` / `executeReboot()`
Modal lifecycle and execution for hardware appliance reboot.
- **HTTP Call:** `POST /api/reboot`.
- **Returns:** `{void}`

#### `mountSdCard(mount)`
Mounts or unmounts the physical micro-SD boot partition (`/dev/mmcblk0p1` to `/mnt/boot`) to allow live firmware updates without SD removal.
- **Parameters:**
  - `{boolean} mount` - `true` to mount read-write, `false` to unmount.
- **HTTP Call:** `POST /api/sdcard/mount` or `POST /api/sdcard/unmount`.
- **Returns:** `{void}`

#### `saveAndApplyNetworkConfig()`
Submits updated static or DHCP network interface configurations.
- **HTTP Call:** `POST /api/network/config`.
- **Returns:** `{void}`

---

## 4. End-to-End REST API Contract Matrix

| Endpoint | HTTP Method | Request Payload (`application/json`) | Response Payload (`application/json`) | Status Codes |
| :--- | :---: | :--- | :--- | :---: |
| `/api/status` | `GET` | *None* | `SystemTelemetry` + `AudioStatus` | `200 OK` |
| `/api/config` | `GET` | *None* | `EncoderConfig` | `200 OK` |
| `/api/displays` | `GET` | *None* | `Array<DisplayInfo>` | `200 OK` |
| `/api/displays/action` | `POST` | `{ "action": string, "transport": string, "mode": string }` | `{ "status": "ok" }` | `200 OK`, `400 Bad Request` |
| `/api/audio` | `GET` | *None* | `AudioStatus` | `200 OK` |
| `/api/audio/rate` | `POST` | `{ "rate": number }` | `{ "status": "ok", "rate": number }` | `200 OK`, `400 Bad Request` |
| `/api/audio/volume` | `POST` | `{ "volume": number }` | `{ "status": "ok", "volume": number }` | `200 OK` |
| `/api/audio/mute` | `POST` | `{ "muted": boolean }` | `{ "status": "ok", "muted": boolean }` | `200 OK` |
| `/api/audio/transport` | `POST` | `{ "transport": string }` | `{ "status": "ok", "transport": string }` | `200 OK` |
| `/api/network` | `GET` | *None* | `{ "usb0": {...}, "eth0": {...}, "wlan0": {...}, "config": NetworkConfig }` | `200 OK` |
| `/api/network/config` | `POST` | `NetworkConfig` | `{ "status": "applied" }` | `200 OK`, `500 Error` |
| `/api/sdcard/status` | `GET` | *None* | `{ "mounted": boolean, "path": string }` | `200 OK` |
| `/api/sdcard/mount` | `POST` | *None* | `{ "status": "mounted", "path": "/mnt/boot" }` | `200 OK`, `500 Error` |
| `/api/sdcard/unmount` | `POST` | *None* | `{ "status": "unmounted" }` | `200 OK`, `500 Error` |
| `/api/reboot` | `POST` | *None* | `{ "status": "rebooting" }` | `200 OK` |
| `/api/stream/pause` | `POST` | *None* | `{ "status": "paused" }` | `200 OK` |
| `/api/stream/resume` | `POST` | *None* | `{ "status": "resumed" }` | `200 OK` |

---

## 5. Auditability and Verification Procedure

To audit frontend code compliance across all three embedded web subsystems and verify that no syntax regressions or API contract mismatches exist:

1. **Extract and Lint All Embedded JavaScript Sources:**
   ```bash
   # 1. Main Dashboard UI (web_ui.rs)
   sed -n '/<script>/,/<\/script>/p' receiver/src/web_ui.rs | sed '1d;$d' > /tmp/dashboard_lint.js
   node --check /tmp/dashboard_lint.js

   # 2. 1-Click Web Cast (web_cast.rs)
   sed -n '/<script>/,/<\/script>/p' receiver/src/web_cast.rs | sed '1d;$d' > /tmp/web_cast_lint.js
   node --check /tmp/web_cast_lint.js

   # 3. Swagger UI Console (swagger.rs)
   sed -n '/<script>/,/<\/script>/p' receiver/src/swagger.rs | sed '1d;$d' > /tmp/swagger_lint.js
   node --check /tmp/swagger_lint.js
   ```

2. **Automated Unit & Integration Test Suite:**
   Run cargo test to ensure all backend handlers backing the JSDoc contracts pass deterministic validation:
   ```bash
   cargo test -p ext-receiver
   ```

---

## 6. 1-Click Web Cast Subsystem Architecture (`receiver/src/web_cast.rs`)

**Document Sub-Reference:** `EXT-MON-JSDOC-WEBCAST-REV1`  
**File Origin:** `receiver/src/web_cast.rs` (`pub const WEB_CAST_HTML`)  
**URL Route:** `GET /cast`  
**Streaming Protocol:** Raw RFC 6455 WebSocket (`/api/stream/ws`) transporting Annex B H.264 NAL units.

### 6.1. Architecture & Pipeline Overview

The Web Cast subsystem allows any browser without host software installation (Windows, macOS, Linux, ChromeOS, Android) to mirror a screen, window, or specific tab directly to the Pi Zero monitor over Wi-Fi or USB ethernet.

```
+-----------------------------------------------------------------------------------+
| Browser (Web Cast Client - /cast)                                                 |
|                                                                                   |
|  [navigator.mediaDevices.getDisplayMedia()]                                       |
|        |                                                                          |
|        v                                                                          |
|  [MediaStreamTrackProcessor]                                                      |
|        |                                                                          |
|        v (VideoFrame)                                                             |
|  [WebCodecs VideoEncoder] (AVC Baseline 3.1 Annex B)                              |
|        |                                                                          |
|        | (Raw Uint8Array NAL units, IDR every 30 frames)                          |
|        v                                                                          |
|  [WebSocket Client] === ws://<pi-ip>:8080/api/stream/ws ===> [Pi Zero Receiver]  |
|                                                                    |              |
|                                                          [V4L2 M2M Decoder]       |
|                                                                    |              |
|                                                          [DRM/KMS HDMI Out]       |
+-----------------------------------------------------------------------------------+
```

### 6.2. Web Cast JSDoc State and Function Reference

```javascript
/**
 * Global Web Cast client state.
 * @namespace WebCastState
 */

/** @type {MediaStream|null} Active display media stream captured from screen or browser tab */
let activeStream = null;

/** @type {WebSocket|null} Active binary WebSocket connection to Pi Zero (/api/stream/ws) */
let activeWs = null;

/** @type {VideoEncoder|null} WebCodecs hardware-accelerated video encoder instance */
let activeEncoder = null;

/** @type {number|null} Interval handle for the elapsed duration timer tick */
let timerInterval = null;

/** @type {number} Total seconds elapsed in current casting session */
let secondsElapsed = 0;
```

#### Functions:

##### `updateTimer()`
- **Description:** Increments `secondsElapsed` by 1 and updates DOM element `#statTimer` with `MM:SS` formatting.
- **Returns:** `{void}`

##### `startBtn.addEventListener('click', async () => { ... })`
- **Description:** Initiates screen capture negotiation:
  1. Reads target resolution (`1080p30`, `720p30`, `480p30`) and target bitrate from DOM selectors.
  2. Prompts user via `navigator.mediaDevices.getDisplayMedia()`.
  3. Establishes binary WebSocket connection to `ws://<host>/api/stream/ws` (`ws.binaryType = 'arraybuffer'`).
  4. Detects hardware `VideoEncoder` (WebCodecs API). Configures `avc1.42001f` with Annex B format.
  5. Spawns asynchronous frame pump using `MediaStreamTrackProcessor.readable.getReader()`.
  6. Dispatches periodic IDR keyframes (`keyFrame: true` every 30 frames) for rapid decoder synchronization.
- **Returns:** `{Promise<void>}`

##### `fallbackMediaRecorder(stream, ws)`
- **Description:** Legacy fallback encoder for older browsers lacking WebCodecs support. Uses standard `MediaRecorder` with `video/webm; codecs=h264` chunks pushed over WebSocket at 40 ms timeslices.
- **Parameters:**
  - `{MediaStream} stream` - Source display stream.
  - `{WebSocket} ws` - Target binary WebSocket channel.
- **Returns:** `{void}`

##### `stopCasting()`
- **Description:** Teardown routine: clears interval timers, closes WebCodecs encoder, terminates WebSocket connection, stops all audio/video `MediaStreamTrack` instances, hides local loopback preview `#previewVideo`, and resets UI badges to standby state.
- **Returns:** `{void}`

---

## 7. Swagger UI Interactive Console Architecture (`receiver/src/swagger.rs`)

**Document Sub-Reference:** `EXT-MON-JSDOC-SWAGGER-REV1`  
**File Origin:** `receiver/src/swagger.rs` (`pub const SWAGGER_HTML`)  
**URL Route:** `GET /swagger`  
**OpenAPI Specification:** `GET /api/openapi.json` (OpenAPI 3.0.3, generated at compile time)

### 7.1. Interactive Console Specification

The Swagger UI console provides live interactive API exploration directly on the Pi Zero, allowing network engineers and developers to test REST endpoints, inspect payload schemas, and view real-time responses.

```javascript
/**
 * @file Swagger UI Interactive Console Initialization
 * @description Bootstraps the Swagger UI interactive API documentation interface.
 * Fetches OpenAPI 3.0.3 specification from `/api/openapi.json` and renders interactive
 * API explorer controls into `#swagger-ui`.
 */

/**
 * Window load lifecycle event handler.
 * Instantiates SwaggerUIBundle with custom layout, filtering, and OpenAPI endpoint mapping.
 * 
 * @callback WindowOnLoadCallback
 * @returns {void}
 */
window.onload = function() {
    /**
     * Global Swagger UI instance handle.
     * @type {object}
     */
    window.ui = SwaggerUIBundle({
        url: "/api/openapi.json",
        dom_id: '#swagger-ui',
        deepLinking: true,
        presets: [
            SwaggerUIBundle.presets.apis,
            SwaggerUIStandalonePreset
        ],
        plugins: [
            SwaggerUIBundle.plugins.DownloadUrl
        ],
        layout: "BaseLayout",
        docExpansion: "list",
        filter: true,
        showExtensions: true,
        showCommonExtensions: true,
        defaultModelsExpandDepth: 1,
        displayRequestDuration: true
    });
};
```

