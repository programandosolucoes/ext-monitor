# Blueprint 31: Web UI Transport Mismatch Diagnosis and Telemetry Synchronization (Mode 1 UDP Network vs Mode 3 USB Bulk)

> [🇧🇷 Versão em Português](../pt/31-diagnostico-mismatch-transporte-ui-e-sincronizacao-telemetria-modo1-vs-modo3.md) | 🇺🇸 English Version

*Date: 2026-10-02*  
*Status: Production Verified and Synchronized*  
*Author: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Context and User Inquiry

During smooth and stable playback of Star Trek on Pluto TV, the user noted a discrepancy on the Web Dashboard (`http://192.168.7.2:8080`):
* The assistant reported that the live transmission was running on **Mode 1: Network UDP (port 5000 for H.264 video, port 5004 for PCM audio)**.
* However, upon inspecting the Web Dashboard, the active transport selector highlighted **Mode 3: USB Bulk Direct**, and Card 3 displayed the purple tag *"Enabled (USB Bulk)"*.
* The user requested validation to clarify where the interpretation mismatch originated.

---

## 2. Root Cause Analysis

A thorough audit across the source code and runtime environment pinpointed **three distinct factors**:

### 2.1 Cause 1: Static HTML with Hardcoded `active` Classes on Mode 3
In `receiver/src/web_ui.rs`, the embedded HTML markup had active button classes hardcoded to Mode 3:
```html
<!-- Line 741 (Tab 1 Transport Grid) -->
<button class="btn-toggle active" id="btnTransport_mode3" onclick="setActiveTransport('mode3_usb_bulk')">

<!-- Line 901 (Mode 3 Card) -->
<button class="btn-toggle active" id="btnMode3Extend" ...>

<!-- Line 948 & 951 (Tab 2 - Host Remote Control) -->
<span class="control-value" id="valHostTransport">USB Bulk Direct (Mode 3)</span>
<button class="btn-toggle active" id="btnHostTransport_mode3" onclick="setActiveTransport('mode3_usb_bulk')">
```
When any web browser opened the page, it instantly rendered Mode 3 highlighted before JavaScript could execute or poll the server.

### 2.2 Cause 2: Initial JS State and Deferred `pollTelemetry`
In the client-side JavaScript initialization:
```javascript
// Line 3181: initialized to Mode 3
let currentTransport = 'mode3_usb_bulk';

// Lines 4426-4429: pollTelemetry only ran on a 2-second interval
pollNetworkStatus();
setInterval(pollTelemetry, 2000);
```
Because `pollTelemetry()` was not invoked immediately upon page load, the UI retained the initial HTML state for at least 2 full seconds.

### 2.3 Cause 3: Daemon Listener Status vs Active Stream Pipeline
On the Raspberry Pi Zero, multiple hardware daemon listeners run concurrently to allow instant zero-reboot transport switching:
* Mode 1: UDP socket listening on ports 5000 and 5004.
* Mode 2: Miracast daemon listening on TCP 7236.
* Mode 3: USB FunctionFS listener.

Each individual mode card displayed a daemon readiness badge:
* Card 1: `badgeMode1` -> *"Enabled (UDP 5000)"*
* Card 2: `badgeMode2` -> *"Enabled (TCP 7236)"*
* Card 3: `badgeMode3` -> *"Enabled (USB Bulk)"*

A user looking at Card 3 saw "Enabled (USB Bulk)" and reasonably inferred that the live stream was traversing USB Bulk, even though that badge only reflected daemon availability. The actual live video feed is indicated exclusively by the top active banner (`activeStreamBanner`) and `/api/status`.

---

## 3. Technical Proof of Active Mode 1 UDP Transport

Inspection of host processes and Pi Zero device nodes confirmed that transmission is 100% Mode 1 UDP Network:

1. **Host Sender Processes:**
   * **Video:** PID 1200419 — `gst-launch-1.0 ... udpsink host=192.168.7.2 port=5000`
   * **Audio:** PID 1172744 — `gst-launch-1.0 ... udpsink host=192.168.7.2 port=5004`
2. **Pi Zero Receiver API (`curl http://192.168.7.2:8080/api/status`):**
   ```json
   {
     "active_transport": "mode1_udp",
     "active_mode": {
       "id": "mode1_udp",
       "name": "Mode 1: UDP Network (Linux Wayland / X11)",
       "icon": "🐧",
       "protocol": "RTP H.264 / RFC 4571",
       "port": 5000,
       "details": "UDP Port 5000 • Latency < 15ms • VA-API/M2M Pipeline"
     },
     "stream_state": "active"
   }
   ```
3. **No USB Bulk Hardware Device Node:**
   * `/dev/usb-display-bulk` does not exist on the Pi Zero.
   * The physical USB cable is configured as an ultra-fast virtual network interface (**USB CDC-ECM / RNDIS** subnet `192.168.7.0/24`). All 6.1 Mbps of multimedia traffic consists of UDP datagrams over `usb0`.

---

## 4. Fix Implemented

In [receiver/src/web_ui.rs](file:///home/carlos/ide/ext-monitor/receiver/src/web_ui.rs):

1. Default active classes updated to `btnTransport_mode1`, `btnHostTransport_mode1`, and `btnMode1Extend`.
2. Initial JavaScript `currentTransport` set to `'mode1_udp'`.
3. Immediate execution of `updateModeAndTopologyButtons();` and `pollTelemetry();` on page load.
4. Binary cross-compiled for ARM musl (`ext-receiver`) cleanly without affecting live video playback.
