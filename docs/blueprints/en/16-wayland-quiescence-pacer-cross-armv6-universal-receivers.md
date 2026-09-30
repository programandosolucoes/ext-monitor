# Blueprint 16: Wayland Quiescence Pacer, Cross-ARMv6 Compilation and Universal Receivers

> [🇧🇷 Versão em Português](../pt/16-pacer-wayland-quiescencia-e-guia-universal-receptores.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `sender/src/main.rs`, `tools/ext-tool/src/builder.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. GNOME Mutter Quiescence and the Damage Pacer

### The Silicon & Compositor Behavior
Under GNOME Wayland (Mutter), the compositor pauses display rendering updates (`stage_painted`) when no screen damage occurs (*damage-driven rendering*).
Consequently:
1. When the mouse is stationary or outside the secondary monitor, PipeWire buffer generation drops to 0 FPS.
2. Background video streams (e.g. YouTube) pause because browsers flag the surface as idle.

### The Solution: Continuous Pacing
Instead of heavy CPU frame duplication (`imagefreeze`), continuous CFR pacing is integrated directly into the `ext-sender` pipeline:
* Signals a continuous damage heartbeat pulse every 16.6ms (60 FPS).
* Mutter is kept active with steady updates: `clutter_stage_schedule_update()` -> `stage_painted()`.
* **Result:** Videos, animations, and terminals render at constant 60 FPS with sub-15ms latency without requiring mouse motion.

---

## 2. Raspberry Pi Zero (ARMv6) Cross-Compilation

The Broadcom BCM2835 SoC requires `armv6l` compilation (ARM1176JZF-S with VFPv2):

### 2.1 Native Rust Toolchain (`ext-tool build`)
Using our integrated Rust toolchain:
```bash
ext-tool build
```
This automatically invokes the cross-compiler for `arm-unknown-linux-gnueabihf`, strips debugging symbols, and packages the runtime into `initramfs.cpio.gz`.

---

## 3. Non-OTG Raspberry Pi Models (Pi 2, 3, 4, 5)

Full-size Raspberry Pi boards (Pi 2, 3, 4B, 5) lack OTG peripheral mode on their standard USB-A ports. They connect over **Local Network (Gigabit Ethernet or Wi-Fi)**:

### 3.1 SD Card Configuration
* In `config.txt`: Remove `dtoverlay=dwc2`. Keep `dtoverlay=vc4-kms-v3d,cma-128` and `dtparam=audio=on`.
* In `cmdline.txt`: Remove `modules-load=dwc2`. Set `ip=dhcp`.
* Streaming connects directly to the assigned IP: `ext-sender 192.168.1.150:5000`.

---

## 4. Reusing Any Old PC / Laptop as a High-Speed Second Monitor

Any secondary PC running Linux can act as a high-speed receiver:
```bash
# On secondary PC:
cargo build --release -p ext-receiver
./target/release/ext-receiver

# On primary host PC:
ext-sender <secondary-pc-ip>:5000
```
