# Blueprint 32: Audio DAC Panel on Tab 1, Chromecast-Style Web Casting & Service Switching Unit Tests

## 1. Overview & Motivation
In previous releases, the advanced digital audio parameters (Master Volume, Mute, Hi-Res sample rates 96kHz / 192kHz / 48kHz / 44.1kHz, and physical transport selectors Mode 1 UDP, Mode 2 UAC2 Gadget, and Mode 3 USB Bulk Mux) were placed in **Tab 2 (Settings)**, hiding them from the user who mostly monitors operations on **Tab 1 (Monitoring & Telemetry)**.
Additionally, the former web sharing mode was ambiguously titled "IoT Media Center & HDMI Visualizer" with placeholder metadata and lacked an intuitive 1-click casting flow.
Finally, formal Rust unit tests were required to verify that the mode switching state machine (`evaluate_mode_switch`):
1. Immediately pauses transmission and enters Standby when all modes are disabled;
2. Transfers the stream to the selected mode when any mode is activated;
3. Falls back to an alternative enabled mode if the active mode is turned off;
4. Retains the active stream when disabling a secondary background listener.

---

## 2. Architecture & Implementation

### 2.1 Mode Switching Unit Tests (`receiver/src/web.rs`)
Implemented the deterministic state evaluator function:
```rust
pub fn evaluate_mode_switch(
    body: &str,
    current_active_transport: &str,
    mut mode1: bool,
    mut mode2: bool,
    mut mode3: bool,
) -> (bool, bool, bool, ModeSwitchResult)
```
Covered by 7 unit tests in `receiver/src/web.rs` passing with 100% success (`cargo test -p ext-receiver`):
1. `test_evaluate_all_modes_disabled_enters_standby`: Disabling all 3 modes triggers `ModeSwitchResult::EnterStandby`.
2. `test_evaluate_explicit_transport_activation_mode1`: Direct switch to `mode1_udp`.
3. `test_evaluate_explicit_transport_activation_mode2`: Direct switch to `mode2_miracast`.
4. `test_evaluate_explicit_transport_activation_mode3`: Direct switch to `mode3_usb_bulk`.
5. `test_evaluate_mode_toggle_transfers_active_stream`: Enabling a mode transfers the active transmission to it.
6. `test_evaluate_active_mode_disabled_fallback_to_other_mode`: Turning off the active mode gracefully falls back to remaining active listeners.
7. `test_evaluate_secondary_mode_disabled_does_not_switch_active`: Disabling a background listener keeps the active session running.

### 2.2 Hi-Res Digital Audio & Hardware DAC Card on Tab 1
Unified on the main frontend screen (`receiver/src/web_ui.rs` in Tab 1):
- **Master Volume Slider (0% to 100%)** with real-time Mute toggle.
- **Hardware Chime Test Button**: Emits a test chime directly on the TV's HDMI ALSA hardware pipeline via `/api/media/test_sound`.
- **Live 30 FPS Audio Spectrum Canvas**: 24-band frequency visualizer driven by the hardware ALSA PCM audio stream.
- **Master Hardware Clock Profiles**:
  - `96 kHz / 24-bit`: Hi-Res Studio (Default)
  - `192 kHz / 24-bit`: Ultra Hi-Res
  - `48 kHz / 16-bit`: Cinema Standard
  - `44.1 kHz / 16-bit`: CD Fidelity
- **Transport Architecture Selector**:
  - `Mode 1`: UDP Network (Port 5004, sub-5ms)
  - `Mode 2`: USB Audio Class (Plug-and-play UAC2 Gadget)
  - `Mode 3`: USB Bulk Mux (/dev/usb-display-bulk)

### 2.3 Web Sharing & Chromecast-Style Casting Card
Redesigned into an authentic Chromecast experience:
- **Google Cast V2 Status**: Ports 8008/8009 with active mDNS discovery.
- **1-Click Web Caster (`/cast`)**: Prominent button launching HTML5 `getDisplayMedia` with WebCodecs hardware encoding to mirror any Chrome/Firefox tab, Meet/Teams call, or entire screen.
- **Direct Video URL Cast (Play on TV)**: Text input to paste video stream URLs (MP4, WebM, HLS m3u8) with "▶ Cast to TV" action.
- **Now Playing Telemetry Box**: Live updates for track title, artist, and streaming status.

---

## 3. Validation & Results
1. **72 Unit Tests Passing**: All receiver tests executed cleanly (`72 passed; 0 failed`).
2. **Instant RAM OTA**: Pi Zero appliance updated in less than 10 seconds via OTA.
3. **Live API Validation**:
   - `{"rate":192000}` and `{"rate":96000}` tested and confirmed via HTTP API.
   - Disabling all modes sets `stream_state: "paused"` and `active_transport: "standby"`.
   - Re-enabling Mode 1 restores live video playback (`active_transport: "mode1_udp"`).
4. **Framebuffer Verification**: Framebuffer capture verified smooth 60 FPS video playback with synchronized audio.
